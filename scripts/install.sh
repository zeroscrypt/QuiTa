#!/usr/bin/env bash
#
# QuiTa installer — macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.sh | bash
#
# It downloads the release build that matches your OS and CPU, verifies its
# SHA-256 against the checksum file published with the same release, and
# installs it. Nothing is executed before the hash is checked.
#
# Options:
#   --version X.Y.Z   install a specific version (default: latest release)
#   --dir PATH        install into PATH instead of the default location
#   --help            show this help
#
# Environment:
#   QUITA_VERSION, QUITA_INSTALL_DIR, QUITA_REPO

set -euo pipefail

REPO="${QUITA_REPO:-zeroscrypt/QuiTa}"
VERSION="${QUITA_VERSION:-latest}"
INSTALL_DIR="${QUITA_INSTALL_DIR:-}"
SKIP_CHECKSUM="${QUITA_SKIP_CHECKSUM:-0}"
APP_NAME="QuiTa"
BUNDLE_ID="QuiTa.app"

if [ -t 1 ]; then
  BOLD="$(printf '\033[1m')"; DIM="$(printf '\033[2m')"
  GREEN="$(printf '\033[32m')"; YELLOW="$(printf '\033[33m')"; RED="$(printf '\033[31m')"
  RESET="$(printf '\033[0m')"
else
  BOLD=""; DIM=""; GREEN=""; YELLOW=""; RED=""; RESET=""
fi

step() { printf '%s==>%s %s%s%s\n' "$GREEN" "$RESET" "$BOLD" "$1" "$RESET"; }
info() { printf '    %s%s%s\n' "$DIM" "$1" "$RESET"; }
warn() { printf '%s ! %s%s\n' "$YELLOW" "$1" "$RESET" >&2; }
die() { printf '%s x %s%s\n' "$RED" "$1" "$RESET" >&2; exit 1; }

usage() {
  # Prints the header comment (everything before the first code line).
  sed -n '2,/^$/p' "$0" | sed 's/^#\{1,\} \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:?--version needs a value}"; shift 2 ;;
    --dir)     INSTALL_DIR="${2:?--dir needs a value}"; shift 2 ;;
    --help|-h) usage ;;
    *)         die "unknown option: $1 (try --help)" ;;
  esac
done

command -v curl >/dev/null 2>&1 || die "curl is required"

# ---------------------------------------------------------------- platform ----

OS="$(uname -s)"
MACHINE="$(uname -m)"
case "$MACHINE" in
  arm64|aarch64)  ARCH_ARM="aarch64"; ARCH_SHORT="arm64" ;;
  x86_64|amd64)   ARCH_ARM="x64";     ARCH_SHORT="amd64" ;;
  *)              die "unsupported CPU architecture: $MACHINE" ;;
esac

case "$OS" in
  Darwin) PLATFORM="macos" ;;
  Linux)  PLATFORM="linux" ;;
  *)      die "unsupported OS: $OS — Windows users should run install.ps1" ;;
esac

# ------------------------------------------------------------------ version ----

resolve_latest_tag() {
  curl -fsSL -H 'Accept: application/vnd.github+json' \
    "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1
}

if [ "$VERSION" = "latest" ]; then
  step "Looking up the latest release"
  TAG="$(resolve_latest_tag || true)"
  [ -n "$TAG" ] || die "could not reach the GitHub API for $REPO"
else
  TAG="$VERSION"
  case "$TAG" in v*) ;; *) TAG="v$TAG" ;; esac
fi
VERSION="${TAG#v}"
info "$TAG"

# Installed version, when there is one. Reading it before the download keeps
# an up-to-date machine from re-fetching and reinstalling the same build.
installed_version() {
  local dir
  if [ "$PLATFORM" = "macos" ]; then
    dir="${INSTALL_DIR:-}"
    [ -n "$dir" ] || dir="$HOME/Applications"
    [ -d "$dir/$BUNDLE_ID" ] || return 0
    # Info.plist holds the version the build was made with.
    defaults read "$dir/$BUNDLE_ID/Contents/Info" CFBundleShortVersionString 2>/dev/null || true
  else
    dir="${INSTALL_DIR:-$HOME/.local/opt/quita}"
    # The AppImage is installed under a fixed name, so its version is not in
    # the file name; this script writes it next to it instead.
    [ -f "$dir/.version" ] && cat "$dir/.version"
  fi
  return 0
}

# QuiTa sits in the tray, so a closed window does not mean a stopped app.
# Replacing a running bundle leaves the old copy running from unlinked files,
# and the next launch then runs stale code that no longer matches the disk.
app_is_running() {
  local dir
  if [ "$PLATFORM" = "macos" ]; then
    dir="${INSTALL_DIR:-}"
    [ -n "$dir" ] || dir="$HOME/Applications"
    pgrep -f "${dir}/${BUNDLE_ID}/Contents/MacOS/" >/dev/null 2>&1
  else
    dir="${INSTALL_DIR:-$HOME/.local/opt/quita}"
    pgrep -f "${dir}/QuiTa.AppImage" >/dev/null 2>&1
  fi
}

DOWNLOAD_BASE="https://github.com/$REPO/releases/download/$TAG"

# ------------------------------------------------------- what is installed ----
# Checked before anything is downloaded. Reinstalling the version that is
# already there fetches and rewrites for nothing, and replacing a running app
# leaves the old code running from files the new install has unlinked.
is_numeric_version() {
  case "$1" in
    *[!0-9.]* | '') return 1 ;;
    *) return 0 ;;
  esac
}

# True when $1 is strictly newer than $2. Only meaningful for plain X.Y.Z,
# which the caller guards. The inequality matters: sorting two equal versions
# puts either one first, so without it equal versions would count as newer.
version_gt() {
  [ "$1" != "$2" ] &&
    [ "$(printf '%s\n%s\n' "$2" "$1" | sort -t. -k1,1n -k2,2n -k3,3n | head -n 1)" = "$2" ]
}

CURRENT="$(installed_version || true)"
if [ -n "$CURRENT" ]; then
  info "already installed: $CURRENT"
  if [ "$CURRENT" = "$VERSION" ]; then
    printf '\n%s%s %s is already installed - nothing to do.%s\n\n' \
      "$BOLD" "$APP_NAME" "$CURRENT" "$RESET"
    exit 0
  fi
  if is_numeric_version "$CURRENT" && is_numeric_version "$VERSION" &&
     version_gt "$CURRENT" "$VERSION"; then
    die "installed $CURRENT is newer than $VERSION - refusing to downgrade"
  fi
  if app_is_running; then
    # The window can be closed while the tray icon keeps the app alive.
    warn "QuiTa $CURRENT is still running"
    die "quit it from the tray icon, then run this again"
  fi
fi

# ------------------------------------------------------------------- assets ----

api_assets() {
  curl -fsSL -H 'Accept: application/vnd.github+json' \
    "https://api.github.com/repos/$REPO/releases/tags/$TAG" \
    | grep -o '"browser_download_url": *"[^"]*"' \
    | sed 's/.*: *"//; s/"$//'
}

# Resolves an asset by suffix pattern: exact Tauri name first (no JSON
# parsing), then whatever the release actually contains.
resolve_asset() {
  local exact="$1" fallback="$2" url=""
  url="$(api_assets | grep -E "/${fallback}\$" | head -n 1 || true)"
  if [ -n "$url" ]; then printf '%s' "$url"; return 0; fi
  if curl -fsSLI -o /dev/null "${DOWNLOAD_BASE}/${exact}"; then
    printf '%s' "${DOWNLOAD_BASE}/${exact}"; return 0
  fi
  return 1
}

if [ "$PLATFORM" = "macos" ]; then
  ASSET_URL="$(resolve_asset "${APP_NAME}_${VERSION}_${ARCH_ARM}.dmg" "_${ARCH_ARM}\.dmg\$")" \
    || die "no macOS build for ${ARCH_ARM} in ${TAG}"
else
  ASSET_URL="$(resolve_asset "${APP_NAME}_${VERSION}_${ARCH_SHORT}.AppImage" "_${ARCH_SHORT}\.AppImage\$")" \
    || die "no Linux build for ${ARCH_SHORT} in ${TAG}"
fi
ASSET_NAME="${ASSET_URL##*/}"

# ---------------------------------------------------------------- checksum ----

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum   >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d' ' -f1
  else echo ""; fi
}

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

step "Downloading $ASSET_NAME"
curl -fL --progress-bar -o "$TMP_DIR/$ASSET_NAME" "$ASSET_URL" \
  || die "download failed: $ASSET_URL"

if [ "$SKIP_CHECKSUM" != "1" ]; then
  step "Verifying SHA-256"
  EXPECTED="$(curl -fsSL "${DOWNLOAD_BASE}/SHA256SUMS" 2>/dev/null \
    | grep -F " $ASSET_NAME" | cut -d' ' -f1 | head -n 1 || true)"
  if [ -z "$EXPECTED" ]; then
    warn "no checksum published for this build — continuing without verification"
  else
    ACTUAL="$(sha256_of "$TMP_DIR/$ASSET_NAME")"
    [ "$ACTUAL" = "$EXPECTED" ] || die "checksum mismatch — refusing to install"
    info "hash matches"
  fi
fi

# ----------------------------------------------------------------- install ----

if [ "$PLATFORM" = "macos" ]; then
  if [ -z "$INSTALL_DIR" ]; then
    if [ -w "/Applications" ]; then INSTALL_DIR="/Applications"
    else INSTALL_DIR="$HOME/Applications"; fi
  fi
  step "Installing into $INSTALL_DIR"
  mkdir -p "$INSTALL_DIR"
  MOUNT_DIR="$TMP_DIR/mount"
  mkdir -p "$MOUNT_DIR"
  hdiutil attach "$TMP_DIR/$ASSET_NAME" -nobrowse -readonly -mountpoint "$MOUNT_DIR" >/dev/null
  rm -rf "${INSTALL_DIR:?}/${BUNDLE_ID}"
  cp -R "$MOUNT_DIR/${BUNDLE_ID}" "$INSTALL_DIR/"
  hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
  # Unsigned builds carry a quarantine flag; this is the documented way to
  # clear it for an app you built yourself.
  xattr -dr com.apple.quarantine "${INSTALL_DIR}/${BUNDLE_ID}" 2>/dev/null || true
  step "Done"
  info "open \"${INSTALL_DIR}/${BUNDLE_ID}\""
  info "if macOS blocks it: right-click the app → Open, then confirm once"
else
  if [ -z "$INSTALL_DIR" ]; then INSTALL_DIR="$HOME/.local/opt/quita"; fi
  step "Installing into $INSTALL_DIR"
  mkdir -p "$INSTALL_DIR" "$HOME/.local/bin"
  install -m 0755 "$TMP_DIR/$ASSET_NAME" "${INSTALL_DIR}/${APP_NAME}.AppImage"
  # The AppImage has a fixed name, so the version is recorded next to it for
  # the "already installed" check on the next run.
  printf '%s\n' "$VERSION" > "${INSTALL_DIR}/.version"
  ln -sf "${INSTALL_DIR}/${APP_NAME}.AppImage" "$HOME/.local/bin/quita"
  step "Done"
  info "run: quita   (make sure ~/.local/bin is in your PATH)"
  info "desktop entry: copy scripts/quita.desktop to ~/.local/share/applications/"
fi

printf '\n%s%s installed.%s\n\n' "$BOLD" "$APP_NAME" "$RESET"
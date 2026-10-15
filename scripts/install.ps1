<#
.SYNOPSIS
    QuiTa installer — Windows.

.EXAMPLE
    irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex

.DESCRIPTION
    Downloads the MSI that matches your CPU, verifies its SHA-256 against the
    checksum file published with the same release, then installs it silently.

    Installing on Windows fails in several different ways that all collapse
    into the same opaque exit code, so each known cause is checked before
    msiexec runs and each exit code is explained afterwards:

      * the MSI installs for every user, so it needs administrator rights;
      * installing the version that is already there, or an older one, fails;
      * another installer holding the Windows Installer lock, a damaged
        package, or an app left running all fail as well.

    When the install fails anyway, the full MSI log is kept and the part of it
    that states the actual error is printed, so the cause can be read instead
    of guessed at.

.PARAMETER Version
    Release to install, e.g. 1.1.0. Defaults to the latest release.

.EXAMPLE
    irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex -Version 1.1.0
#>

[CmdletBinding()]
param(
  [string]$Version = 'latest'
)

$ErrorActionPreference = 'Stop'
$Repo = if ($env:QUITA_REPO) { $env:QUITA_REPO } else { 'zeroscrypt/QuiTa' }
$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x86' }

function Write-Step($m) { Write-Host "==> $m" -ForegroundColor Green }
function Write-Info($m) { Write-Host "    $m" -ForegroundColor DarkGray }
function Write-Note($m) { Write-Host "    $m" -ForegroundColor Yellow }

# GitHub rejects API requests that arrive without a User-Agent.
$headers = @{ 'User-Agent' = 'QuiTa-installer' }

function Get-InstalledQuiTa {
  # Windows keeps the uninstall record here for every kind of install; both
  # the MSI and the NSIS installer register, so this finds either.
  $paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
  )
  @(Get-ItemProperty -Path $paths -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -like 'QuiTa*' })
}

function Test-Administrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  (New-Object Security.Principal.WindowsPrincipal $identity).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Turns an msiexec exit code into something a person can act on. The codes are
# a fixed list from the installer API, so this mapping cannot go stale.
function Get-MsiExitHint([int]$code) {
  switch ($code) {
    1603 {
      'the install failed for a reason Windows does not report here; the log below states it'
    }
    1605 {
      'this app is installed for every user, so removing it needs administrator rights too'
    }
    1618 {
      'another installer is running right now - close "Apps & features", Windows Update and any pending setup, then retry'
    }
    1619 {
      'the MSI could not be opened - the download is incomplete or was blocked; retry, or download the file by hand'
    }
    1620 {
      'the MSI is damaged - retry, or download the file by hand'
    }
    1622 {
      'the installer could not open the log file - check that %TEMP% is writable'
    }
    1638 {
      'another version is already installed - remove it first, then install this one'
    }
    1639 {
      'the command line for msiexec was wrong - this is a bug in this script, please report it'
    }
    3010 {
      'the install succeeded but Windows wants to restart to finish'
    }
    default { $null }
  }
}

Write-Step 'Looking up the latest release'
if ($Version -eq 'latest') {
  $release = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases/latest"
  $tag = $release.tag_name
  $assets = $release.assets
} else {
  $tag = if ($Version.StartsWith('v')) { $Version } else { "v$Version" }
  $assets = (Invoke-RestMethod -Headers $headers `
    -Uri "https://api.github.com/repos/$Repo/releases/tags/$tag").assets
}
$tagVersion = $tag -replace '^v', ''
Write-Info $tag

# --- What is already on the machine? -------------------------------------
# Checked before anything is downloaded. Reinstalling the version that is
# already there or an older one both fail, and both are cases where running
# msiexec is the wrong move rather than the fix.
$installed = Get-InstalledQuiTa
if ($installed.Count -gt 0) {
  $current = ($installed | Select-Object -First 1).DisplayVersion
  Write-Info "already installed: $current"
  if ($current -and $current -eq $tagVersion) {
    Write-Host "`nQuiTa $current is already installed - nothing to do." -ForegroundColor Green
    exit 0
  }
  if ($current) {
    # A tag such as 1.0.0-rc.1 is not a [version]; the comparison is wrapped so
    # an unusual name cannot crash the installer before it has done anything.
    $installedIsNewer = $false
    try {
      $installedIsNewer = ([version]$current) -gt ([version]$tagVersion)
    } catch {
      Write-Note 'cannot compare versions numerically - continuing'
    }
    if ($installedIsNewer) {
      Write-Host "`nInstalled $current is newer than $tagVersion - refusing to downgrade." -ForegroundColor Yellow
      Write-Host 'Remove it first, or run without -Version to get the latest.' -ForegroundColor Yellow
      exit 1
    }
    Write-Info "upgrading $current to $tagVersion"
  }
}

# --- Administrator rights -------------------------------------------------
# The MSI installs for every user, so without elevation Windows refuses it
# with 1603 and no explanation whatsoever. Checked before the download, since
# there is no point fetching a file that cannot be installed.
if (-not (Test-Administrator)) {
  Write-Note 'administrator rights are required'
  $hint = @'
Open PowerShell as administrator (right-click the Start button, "Terminal
(Admin)"), then run the same command:

    irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex
'@
  Write-Host "`n$hint" -ForegroundColor Yellow
  exit 1
}

# --- Is the app running? --------------------------------------------------
# Windows locks files that are in use. QuiTa sits in the tray by default, so
# a closed window does not mean a stopped app.
$running = @(Get-Process -Name 'quita' -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
  Write-Note 'QuiTa is running - the installer cannot replace a running binary'
  Write-Host 'Quit it from the tray icon first, then run this again.' -ForegroundColor Yellow
  exit 1
}

$asset = $assets | Where-Object { $_.name -like "*_${Arch}_en-US.msi" } | Select-Object -First 1
if (-not $asset) { throw "no Windows build for $Arch in $tag" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("quita-" + [Guid]::NewGuid().ToString('N'))
# The log lives outside $tmp on purpose: $tmp is removed on the way out, and a
# log the reader cannot open afterwards is no log at all.
$logFile = Join-Path ([IO.Path]::GetTempPath()) ("quita-install-" + [Guid]::NewGuid().ToString('N') + ".log")
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  $file = Join-Path $tmp $asset.name
  Write-Step "Downloading $($asset.name)"
  Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $file -UseBasicParsing

  # Authenticity, not just integrity. SHA256SUMS travels in the same release
  # as the file it describes, so a compromised account can change both at
  # once. The signature is published under the key already baked into every
  # installed copy, and verifying it is what closes that gap. minisign is
  # rarely present on Windows, so this reports rather than skipping quietly.
  Write-Step 'Verifying the signature'
  $minisign = Get-Command minisign -ErrorAction SilentlyContinue
  if ($minisign) {
    $pubFile = Join-Path $tmp 'quita.pub'
    [IO.File]::WriteAllBytes($pubFile, [Convert]::FromBase64String('dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDk1NEUwMjBERDg3MDFGMUYKUldRZkgzRFlEUUpPbGY2VzNiendWSXlhbXJDenpnM1BTMDN6NVJBbUZNSXFNQ1lzYkpLeEZGT2gK'))
    $sigFile = Join-Path $tmp "$($asset.name).sig"
    try {
      Invoke-WebRequest -Uri "$($asset.browser_download_url).sig" -OutFile $sigFile -UseBasicParsing
    } catch {
      Write-Warning 'no signature published for this file; only the checksum was checked'
    }
    if (Test-Path $sigFile) {
      & $minisign.Source -V -q -p $pubFile -m $file -x $sigFile
      if ($LASTEXITCODE -ne 0) { throw 'The signature does not match - refusing to install' }
      Write-Note 'signed by the release key'
    }
  } else {
    Write-Warning 'minisign is not installed, so the signature could not be checked.'
  }

  Write-Step 'Verifying SHA-256'
  $expected = $null
  $sumsFile = Join-Path $tmp 'SHA256SUMS'
  try {
    Invoke-WebRequest -Uri "https://github.com/$Repo/releases/download/$tag/SHA256SUMS" `
      -OutFile $sumsFile -UseBasicParsing
    $expected = (Select-String -Path $sumsFile -Pattern " $($asset.name)$").Line.Split()[0]
  } catch {
    # Installing without a hash would mean trusting an unverified download,
    # which is the one thing this script exists to prevent. The opt-out is
    # an environment variable on purpose: a deliberate act that stays in
    # the shell history, rather than something a missing file decides.
    if ($env:QUITA_SKIP_CHECKSUM -eq '1') {
      Write-Note 'QUITA_SKIP_CHECKSUM is set - installing without verification'
    } else {
      throw 'No checksum published for this build - refusing to install. Set QUITA_SKIP_CHECKSUM=1 to install anyway, at your own risk.'
    }
  }
  if ($expected) {
    $actual = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected.ToLower()) { throw 'checksum mismatch - refusing to install' }
    Write-Info 'hash matches'
  }

  Write-Step 'Installing'
  # /l*v writes the full log. The exit code alone says nothing about the
  # cause; the log is the only place that states what actually went wrong.
  $proc = Start-Process msiexec.exe -Wait -PassThru -ArgumentList @(
    '/i', "`"$file`"", '/quiet', '/norestart', '/l*v', "`"$logFile`""
  )
  # Start-Process -Wait can return before the exit code is readable.
  $proc.WaitForExit()
  $code = [int]$proc.ExitCode

  # 3010 means "installed, restart to finish" - it is a success, not a failure.
  if ($code -eq 0 -or $code -eq 3010) {
    Remove-Item $logFile -Force -ErrorAction SilentlyContinue
    if ($code -eq 3010) {
      Write-Host "`nQuiTa installed. Restart Windows to finish." -ForegroundColor Green
    } else {
      Write-Host "`nQuiTa installed.`n" -ForegroundColor Green
    }
    exit 0
  }

  Write-Host "`nmsiexec failed with exit code $code" -ForegroundColor Red
  $hint = Get-MsiExitHint $code
  if ($hint) { Write-Host "  $hint" -ForegroundColor Yellow }

  # The interesting part of an MSI log is the block around "Return value 3",
  # which is where the installer states what it could not do.
  if (Test-Path $logFile) {
    Write-Host "`nInstaller log: $logFile" -ForegroundColor DarkGray
    $failure = Select-String -Path $logFile -Pattern 'Return value 3' -Context 0, 12 |
      Select-Object -Last 1
    if ($failure) {
      Write-Host "`nLast error reported by the installer:" -ForegroundColor Red
      $failure.Context.PostContext |
        Where-Object { $_ -and $_ -notmatch '^\s*$' } |
        Select-Object -First 8 |
        ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
    }
  }
  exit $code
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
<div align="center">

# QuiTa

**Quick Tasks App**
A task manager that lives in the tray.

Captured a thought in a second — back to work.

[![CI](https://github.com/zeroscrypt/QuiTa/actions/workflows/ci.yml/badge.svg)](https://github.com/zeroscrypt/QuiTa/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zeroscrypt/QuiTa?label=release&logo=github&style=flat-square&labelColor=1f2430)](https://github.com/zeroscrypt/QuiTa/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-4c6fff?style=flat-square&labelColor=1f2430)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db?style=flat-square&logo=tauri&logoColor=24c8db&labelColor=1f2430)](https://tauri.app)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-ff3e00?style=flat-square&logo=svelte&logoColor=ff3e00&labelColor=1f2430)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-stable-dea584?style=flat-square&logo=rust&logoColor=dea584&labelColor=1f2430)](https://www.rust-lang.org)
[![SQLite](https://img.shields.io/badge/SQLite-bundled-003b57?style=flat-square&logo=sqlite&logoColor=003b57&labelColor=1f2430)](https://sqlite.org)
![No accounts](https://img.shields.io/badge/no%20accounts%20%2F%20no%20cloud%20%2F%20no%20telemetry-4c6fff?style=flat-square&labelColor=1f2430)

English · **[Русский](README.ru.md)**

</div>

---

> [What this is](#what-this-is) · [Install](#install) · [Features](#features) · [How it works](#how-it-works) · [Data](#data) · [Development](#development) · [Project](#project)

---

## What this is

Every task manager offers **planning**. QuiTa offers **capture**.

A thought arrives while you are three hours into something else. Any serious
planner turns that into friction: find a window, find a list, pick a project,
think about a due date, write a description. The thought is gone and you end up
in a different app.

```
   ⌘⇧Q             window appears, caret already in the input
   "ping the supplier about the invoice"
   ⏎                saved, window hides itself
   ────
   ~0.4 s — back in the editor
```

The task is saved. That is the whole product. Everything else — tabs, subtasks,
due dates, drag & drop — exists to make the capture worthwhile *later*, never
to slow down the moment you write it.

No accounts. No cloud. No telemetry. The database is one SQLite file on your
disk.

### Who it is for

- [x] People who **lose thoughts between tools** — a thought is not a project, it is a note that must not dissolve.
- [x] People who **do not like** task managers and do not want to run a system.
- [x] People who need a **private file**, not a service that can shut down or turn paid.

---

## Install

- [x] **Ready build, one command** — macOS and Linux
- [x] **Ready build, one command** — Windows
- [x] **Download the file** — `.dmg`, `.msi`, `.AppImage`, `.deb`
- [x] **Build from source**
- [x] **Integrity check** — SHA-256 for every artifact in `SHA256SUMS`

### macOS and Linux — one command

```bash
curl -fsSL https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.sh | bash
```

The script detects your OS and CPU, downloads the right file, verifies its
SHA-256 against the published checksum, and only then unpacks it. Nothing runs
before the check passes.

### Windows — one command

```powershell
irm https://raw.githubusercontent.com/zeroscrypt/QuiTa/main/scripts/install.ps1 | iex
```

Run it from PowerShell **as administrator** — the installer is machine-wide,
and without the rights Windows refuses it silently, with exit code `1603`.
The script checks for that up front and says so when the rights are missing.

### Download manually

[**Releases**](https://github.com/zeroscrypt/QuiTa/releases/latest) → pick the
file for your system:

| Platform | File |
|----------|------|
| macOS (Apple Silicon, Intel) | `QuiTa_<version>_aarch64.dmg` · `QuiTa_<version>_x64.dmg` |
| Windows | `QuiTa_<version>_x64-setup.exe` · `QuiTa_<version>_x64_en-US.msi` |
| Linux | `QuiTa_<version>_amd64.AppImage` · `QuiTa_<version>_amd64.deb` |

### From source

**Required:** Node.js 22+, pnpm 12+, Rust stable. On Linux also
`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`.

```bash
git clone https://github.com/zeroscrypt/QuiTa.git
cd QuiTa
pnpm install
pnpm tauri dev          # run with hot reload
pnpm tauri build        # produce installers
```

`pnpm tauri build` produces `.app` + `.dmg` on macOS, `.msi` + `.exe` on
Windows, `.AppImage` + `.deb` on Linux.

> You do not need a signing key to build locally: the update artifacts are
> produced only in CI, where the key lives as a GitHub secret.

---

## Features

- [x] **Tabs as projects** — create, rename by double click, reorder by drag, automatic colour from a 10-colour palette. Deleting a tab moves its tasks to a neighbour: data loss is impossible.
- [x] **Subtasks** — exactly one level of nesting, and no more is possible.
- [x] **Due dates** — calendar dates with no timezone shift, overdue tasks highlighted, chips for today / tomorrow / +7 days.
- [x] **Comments** — autosaved, opened with a double click.
- [x] **Storm entry** — the field never closes after `Enter`, the caret stays put.
- [x] **Drag & drop** — three zones, no timers and no dwell.
- [x] **Recycle bin** — soft cascade delete with automatic retention cleanup.
- [x] **Search** — no delay, keeps the parent context.
- [x] **Dark and light theme** — follows the system or pinned manually.
- [x] **Русский and English** — switched in settings, applied immediately.
- [x] **Lives in the tray** — closing the window hides the app instead of quitting.
- [x] **Hotkey** — `⌘⇧Q` on macOS, `Win+Alt+Q` on Windows, `Ctrl+Alt+Q` on Linux — opens the window with the caret in the input.
- [x] **Export and import** — a snapshot of the whole app as JSON.
- [x] **In-app updates** — a green strip above the tabs: "Version 1.1.4 is available".
- [x] **Resizable window** — double-click the logo to fill the height and widen by 1.6×.
- [x] **Overdue highlighting** — a separate colour, not just a date.
- [x] **Keyboard first** — `Enter` saves, `Esc` cancels, the caret never jumps.

### What QuiTa will never do

- [ ] ~~Sync between devices~~ — not available. Honest export instead.
- [ ] ~~Collaboration~~ — not available.
- [ ] ~~Telemetry, analytics, crash reports~~ — not available.

Each of these is a reason to stop being a buffer.

---

## How it works

### Storm

```
   ┌──────────────────────────────────────────┐
   │ Write the release notes                  │  ⏎
   │ Send the invoice to the supplier         │  ⏎
   │ Collect the parcel                       │  ⏎
   └──────────────────────────────────────────┘
```

`Enter` creates the task, clears the field and keeps the caret. Press `+` on a
root task and the subtask field behaves identically.

### Three drag zones

```
   ┌─────────────────────────────────────────┐
   │ ░░░░░░░░ insert above ░░░░░░░░░░░░░░░░░ │  top quarter
   │ ▓▓▓▓▓ release → becomes a subtask ▓▓▓▓▓ │  middle half
   │ ░░░░░░░░ insert below ░░░░░░░░░░░░░░░░░ │  bottom quarter
   └─────────────────────────────────────────┘
```

- [x] **No holding, no timers.** The insertion point is visible *before* you let go.
- [x] **The last position is real.** Empty space under the list means "insert at the end".
- [x] **Tasks and tabs both drag.** A task onto a tab moves it with its subtasks.
- [x] **Cycles are impossible** — a task cannot nest inside its own subtree.

### Completion rules

- [x] **Completing a parent either cascades to its subtasks or leaves them alone** — your choice in settings.
- [x] **Reopening restores exactly the subtasks the cascade completed**, never the ones you finished yourself.
- [x] **A subtask never touches its parent.** Unchecking one leaves the task closed and ◐ partially done.

---

## Data

- [x] **One SQLite file on the device.**
- [x] **No network connection at all** — except the update check.
- [x] **Export is a snapshot of everything**: tasks with their hierarchy, the recycle bin, tabs, settings.
- [x] **Import never deletes** — it only adds and updates records by `id`.
- [x] **Broken hierarchy is repaired**: a subtask without a parent becomes a root task instead of vanishing.

```json
{
  "version": 2,
  "exportedAt": "2026-10-14T09:41:00Z",
  "tabs": [{ "id": "…", "name": "Work", "color": "#4c6fff", "sortOrder": 0 }],
  "settings": { "theme": "dark", "language": "en", "newestFirst": true },
  "tasks": [{ "id": "…", "title": "Ping the supplier", "parentId": null, "tabId": "…" }]
}
```

One format — JSON: every tool reads it and it contains no parser a hostile file
could break. Version 1 files import as-is.

| Platform | Database path |
|----------|---------------|
| macOS | `~/Library/Application Support/com.quita.app/quita.db` |
| Windows | `%APPDATA%\com.quita.app\quita.db` |
| Linux | `~/.local/share/com.quita.app/quita.db` |

> `Application Support` is not synced by iCloud Drive. If you want one database
> across two Apple devices, move the file yourself or use the export: iCloud
> syncs whole files, so two running apps would overwrite each other.

---

## Development

### Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Shell | **Tauri 2** | Megabytes instead of hundreds; native window, tray and hotkeys. |
| UI | **Svelte 5** (runes) | Fine-grained reactivity — the app is almost entirely lists. |
| Language | **TypeScript** | Strict types and a shared contract with Rust. |
| Logic | **Rust** | Completion rules, cascades, migrations — pure functions under test. |
| Storage | **SQLite** (`rusqlite`, bundled) | One file, no server, no native dependencies. |

```
frontend (Svelte 5 + TS)  ──invoke──▶  Tauri commands  ──▶  repository (rusqlite)
        │                                                          │
        └────────────────────  events  ◀────────────────────────────┘
```

Business rules live in `src-tauri/src/domain/rules.rs` as a pure function
returning a plan, and the repository only applies it. Everything interesting is
therefore testable without a window.

### Commands

| Command | What it does |
|---------|--------------|
| `pnpm tauri dev` | Run with hot reload |
| `pnpm tauri build` | Installers for the current platform |
| `pnpm check` | Svelte and TypeScript types |
| `pnpm test` | 107 unit tests |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 92 Rust tests (18 unit + 74 integration against a real database) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets` | Lints, errors as warnings |
| `cargo audit` | Vulnerabilities in dependencies |

### What it costs

| | |
|---|---|
| App size | 17.6 MB |
| `.dmg` size | 5.8 MB |
| Database | ≈210 bytes per task |
| Query time | 1–3 ms 

---

## Project

### Documentation

| | |
|---|---|
| [`docs/PRODUCT.md`](docs/PRODUCT.md) | Product description, positioning, boundaries |
| [`docs/UX_SPEC.md`](docs/UX_SPEC.md) | Layout, gestures, keyboard |
| [`docs/TASK_RULES.md`](docs/TASK_RULES.md) | Completion, cascade, reparenting, cycles |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Process boundaries, IPC, stores |
| [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md) | Tables, columns, migrations, export format |
| [`docs/TEST_PLAN.md`](docs/TEST_PLAN.md) | Automated tests and a manual checklist |
| [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) | Release, signing, notarization |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | What is planned next |

### Contributing

Issues and pull requests are welcome — read
[`CONTRIBUTING.md`](CONTRIBUTING.md) first. Report security problems to
[`SECURITY.md`](SECURITY.md), not to public issues.

### License

[MIT](LICENSE) © [zeroscrypt](https://github.com/zeroscrypt)
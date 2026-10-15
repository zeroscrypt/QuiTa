use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::platform::tray::show_main_window;

/// Accelerator on macOS: Command+Shift+Q, the platform convention for a
/// utility that is toggled from anywhere.
pub const SHORTCUT_MACOS: &str = "CmdOrCtrl+Shift+Q";

/// Accelerator on Windows and Linux: the Super (Windows) key with Alt+Q.
///
/// `CmdOrCtrl+Shift+Q` would become Ctrl+Shift+Q there. Windows does not
/// reserve that combination itself, but plenty of applications claim it as a
/// global hotkey — a running Telegram or a JetBrains IDE can take it before we
/// register, and then the shortcut never fires for us. Super+Alt+Q is not bound
/// by the system and is rarely claimed by anything else, on both systems.
pub const SHORTCUT_OTHER: &str = "Super+Alt+Q";

/// The accelerator to register for the given target OS, in the spelling the
/// `Shortcut` parser understands.
pub fn shortcut_for(target_os: &str) -> &'static str {
    if target_os == "macos" {
        SHORTCUT_MACOS
    } else {
        SHORTCUT_OTHER
    }
}

/// The same combination written the way the user's keyboard labels it, for the
/// settings hint and the documentation.
pub fn shortcut_label(target_os: &str) -> &'static str {
    if target_os == "macos" {
        "⌘⇧Q"
    } else {
        "Win+Alt+Q"
    }
}

/// Registers the global shortcut with its handler. If registration fails — a
/// missing accessibility permission on macOS, or another app already owning
/// the combination — the app keeps working through the tray icon. The failure is
/// logged, never fatal.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.clone();
    // `std::env::consts::OS` rather than the runtime target: the accelerator
    // cannot change while the process runs.
    let shortcut_str = shortcut_for(std::env::consts::OS);

    let shortcut = Shortcut::from_str(shortcut_str).map_err(|e| -> Box<dyn std::error::Error> {
        format!("invalid global shortcut {shortcut_str}: {e}").into()
    })?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if matches!(event.state(), ShortcutState::Pressed) {
                show_main_window(&handle);
                if let Some(win) = handle.get_webview_window("main") {
                    // Lets the UI focus the quick-add input (unless search is active).
                    let _ = win.emit("quita:global-shortcut", ());
                }
            }
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_keeps_the_command_convention() {
        assert_eq!(shortcut_for("macos"), "CmdOrCtrl+Shift+Q");
        assert_eq!(shortcut_label("macos"), "⌘⇧Q");
    }

    #[test]
    fn windows_and_linux_use_super_alt() {
        // Not because Windows reserves Ctrl+Shift+Q — it does not — but
        // because other applications register it globally and can win the race.
        assert_eq!(shortcut_for("windows"), "Super+Alt+Q");
        assert_eq!(shortcut_for("linux"), "Super+Alt+Q");
        assert_eq!(shortcut_label("windows"), "Win+Alt+Q");
        assert_eq!(shortcut_label("linux"), "Win+Alt+Q");
    }

    #[test]
    fn every_platform_key_combination_parses() {
        // A typo in an accelerator string is a silent runtime failure otherwise.
        for os in ["macos", "windows", "linux"] {
            let parsed = Shortcut::from_str(shortcut_for(os));
            assert!(parsed.is_ok(), "{os}: {parsed:?}");
        }
    }

    #[test]
    fn the_label_matches_the_accelerator_on_each_platform() {
        for os in ["macos", "windows", "linux"] {
            let keys = shortcut_label(os)
                .chars()
                .filter(|c| !matches!(c, '+' | ' '))
                .count();
            let modifiers = shortcut_for(os).matches('+').count();
            assert!(keys > modifiers, "{os}: label and accelerator disagree");
        }
    }
}

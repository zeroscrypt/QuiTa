use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::platform::tray::show_main_window;

/// Global shortcut that toggles the QuiTa window: Cmd+Shift+Q on macOS,
/// Ctrl+Shift+Q elsewhere. Chosen to avoid common OS and app conflicts.
pub const SHORTCUT_STR: &str = "CmdOrCtrl+Shift+Q";

/// Registers the global shortcut with its handler. If registration fails (for
/// example missing accessibility permission on macOS), the app keeps working
/// via the tray icon — the failure is logged, never fatal.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.clone();

    let shortcut = Shortcut::from_str(SHORTCUT_STR).map_err(|e| -> Box<dyn std::error::Error> {
        format!("invalid global shortcut {SHORTCUT_STR}: {e}").into()
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

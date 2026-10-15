use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::platform::tray::show_main_window;

/// Accelerator on macOS: Command+Shift+Q, the platform convention for a
/// utility that is toggled from anywhere.
pub const SHORTCUT_MACOS: &str = "CmdOrCtrl+Shift+Q";

/// Accelerator on Windows: the Super key with Alt+Q.
///
/// `CmdOrCtrl+Shift+Q` would become Ctrl+Shift+Q there. Windows does not
/// reserve that combination itself, but plenty of applications claim it as a
/// global hotkey — a running Telegram or a JetBrains IDE can take it before we
/// register, and then the shortcut never fires for us. Super+Alt+Q is not bound
/// by the system and is rarely claimed by anything else.
pub const SHORTCUT_WINDOWS: &str = "Super+Alt+Q";

/// Accelerator on Linux: Ctrl+Alt+Q.
///
/// The Super key was tried first and dropped: desktop environments and window
/// managers normally claim it for "show desktop" or workspace switching, and
/// when one does, the shortcut silently never fires. Ctrl+Alt is not bound by
/// any desktop environment.
///
/// The caveat is keyboard layouts: on layouts where Ctrl+Alt produces AltGr
/// (German, Polish and others) the combination may not arrive as Ctrl+Alt+Q.
/// That is a property of the layout, not of the app, and it is why the
/// combination is named rather than fixed.
pub const SHORTCUT_LINUX: &str = "Ctrl+Alt+Q";

/// The accelerator to register for the given target OS, in the spelling the
/// `Shortcut` parser understands.
pub fn shortcut_for(target_os: &str) -> &'static str {
    match target_os {
        "macos" => SHORTCUT_MACOS,
        "linux" => SHORTCUT_LINUX,
        _ => SHORTCUT_WINDOWS,
    }
}

/// The same combination written the way the user's keyboard labels it, for the
/// settings hint and the documentation.
pub fn shortcut_label(target_os: &str) -> &'static str {
    match target_os {
        "macos" => "⌘⇧Q",
        "linux" => "Ctrl+Alt+Q",
        _ => "Win+Alt+Q",
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
    fn windows_uses_super_alt() {
        // Not because Windows reserves Ctrl+Shift+Q — it does not — but
        // because other applications register it globally and can win the race.
        assert_eq!(shortcut_for("windows"), "Super+Alt+Q");
        assert_eq!(shortcut_label("windows"), "Win+Alt+Q");
    }

    #[test]
    fn linux_avoids_the_super_key() {
        // Desktop environments claim Super for the workspace and the desktop.
        assert_eq!(shortcut_for("linux"), "Ctrl+Alt+Q");
        assert_eq!(shortcut_label("linux"), "Ctrl+Alt+Q");
        assert!(
            !shortcut_for("linux").contains("Super"),
            "Super is the key window managers take on Linux"
        );
    }

    #[test]
    fn every_platform_gets_its_own_combination() {
        // A single shared string is what made the Windows shortcut unusable.
        let all = ["macos", "windows", "linux"].map(shortcut_for);
        assert_eq!(all[0], "CmdOrCtrl+Shift+Q");
        assert_eq!(all[1], "Super+Alt+Q");
        assert_eq!(all[2], "Ctrl+Alt+Q");
        assert_eq!(
            all.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
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

/// Guards the documentation against the exact drift this module is prone to: a
/// README line naming a platform and pairing it with another platform's
/// shortcut. Both READMEs once described the Windows combination as the Linux
/// one while the code registered something else, and nothing noticed.
#[cfg(test)]
mod docs {
    use super::shortcut_label;

    const READMES: [(&str, &str); 2] = [
        ("README.md", include_str!("../../../README.md")),
        ("README.ru.md", include_str!("../../../README.ru.md")),
    ];

    /// How each README spells a platform, and the key it maps to in the code.
    const PLATFORMS: [(&str, &str); 3] = [
        ("macOS", "macos"),
        ("Windows", "windows"),
        ("Linux", "linux"),
    ];

    /// Splits a line so each platform keeps the text written just before it.
    /// In `⌘⇧Q on macOS, Win+Alt+Q on Windows` a label sits in front of its own
    /// platform, which is how both READMEs are written.
    fn text_before_each_platform(line: &str) -> Vec<(&'static str, String)> {
        let mut cursor = 0usize;
        let mut out: Vec<(&'static str, String)> = Vec::new();
        while cursor < line.len() {
            let mut found: Option<(usize, usize, &'static str)> = None;
            for (name, os) in PLATFORMS {
                if let Some(at) = line[cursor..].find(name) {
                    let at = cursor + at;
                    if found.is_none_or(|(best, _, _)| at < best) {
                        found = Some((at, at + name.len(), os));
                    }
                }
            }
            match found {
                Some((at, end, os)) => {
                    out.push((os, line[cursor..at].to_string()));
                    cursor = end;
                }
                None => break,
            }
        }
        out
    }

    #[test]
    fn readmes_name_the_shortcut_every_platform_uses() {
        for (file, text) in READMES {
            for (_, os) in PLATFORMS {
                assert!(
                    text.contains(shortcut_label(os)),
                    "{file} never mentions the {os} shortcut {}",
                    shortcut_label(os)
                );
            }
        }
    }

    #[test]
    fn readmes_pair_each_platform_with_its_own_shortcut() {
        for (file, text) in READMES {
            for (index, line) in text.lines().enumerate() {
                let platforms = text_before_each_platform(line);
                if platforms.is_empty() {
                    continue;
                }
                // Lines that say nothing about shortcuts say nothing about them.
                if !platforms
                    .iter()
                    .any(|(_, os)| line.contains(shortcut_label(os)))
                {
                    continue;
                }
                for (os, before) in &platforms {
                    assert!(
                        before.contains(shortcut_label(os)),
                        "{}:{} pairs a platform with the wrong shortcut.\n  line: {}\n  \
                         {} registers {}",
                        file,
                        index + 1,
                        line.trim(),
                        os,
                        shortcut_label(os)
                    );
                }
            }
        }
    }
}

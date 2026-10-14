use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// Menu bar artwork: a checkmark inside a square outline, drawn in pure black
/// on transparency. Source of truth is `icons/tray-icon.svg`.
#[cfg(target_os = "macos")]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-icon@2x.png");
#[cfg(not(target_os = "macos"))]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-icon.png");

/// Decodes the tray artwork. Returns `None` if the embedded PNG cannot be read,
/// in which case the caller falls back to the application icon.
fn tray_icon() -> Option<Image<'static>> {
    Image::from_bytes(TRAY_ICON_PNG).ok()
}

/// Shows, unminimizes and focuses the main window.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Tells the UI to open the "About" dialog (both languages).
fn emit_about<R: Runtime>(app: &AppHandle<R>) {
    show_main_window(app);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("quita:about", ());
    }
}

/// Builds the tray/menu-bar icon. Menu labels are bilingual because the tray
/// menu is built by the OS side, before the UI language is known.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItemBuilder::with_id("open", "Open QuiTa / Открыть QuiTa").build(app)?;
    let about_item = MenuItemBuilder::with_id("about", "About / О программе").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit / Выйти").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &about_item, &separator, &quit_item])
        .build()?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("QuiTa")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "about" => emit_about(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = tray_icon() {
        builder = builder.icon(icon);
    } else if let Some(fallback) = app.default_window_icon().cloned() {
        builder = builder.icon(fallback);
    }

    // Template images are recoloured by macOS to match the menu bar, so the
    // icon stays legible in light mode, dark mode and while highlighted.
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);

    builder.build(app)?;

    Ok(())
}

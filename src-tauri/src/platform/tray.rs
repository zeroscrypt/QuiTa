use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// Menu bar artwork: a checkmark inside a square outline, drawn in pure black
/// on transparency. Source of truth is `icons/tray-icon.svg`.
///
/// Only the alpha channel carries the shape. macOS redraws template images
/// itself, so it takes the artwork as it is; on Windows and Linux nothing
/// recolours the glyph, and the same black pixels would disappear into a dark
/// taskbar.
#[cfg(target_os = "macos")]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-icon@2x.png");
#[cfg(not(target_os = "macos"))]
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../icons/tray-icon.png");

/// Ink for the tray glyph.
///
/// A dark surface needs a light glyph and the other way round, so the choice
/// follows the theme the window reports. The default is light: that is what
/// Windows and the common Linux panels expect, and a theme that cannot be read
/// should not leave the icon invisible.
///
/// macOS ignores the colour because the icon is a template image, but asking
/// anyway keeps the artwork legible there too if the template flag ever goes.
fn ink_for(app: &AppHandle<impl Runtime>) -> u8 {
    let theme = app
        .get_webview_window("main")
        .and_then(|window| window.theme().ok());
    match theme {
        Some(tauri::Theme::Light) => 0,
        _ => 255,
    }
}

/// Replaces the colour of every pixel while keeping its alpha, which is what
/// defines the shape.
fn repaint(pixels: &mut [u8], ink: u8) {
    let (quads, remainder) = pixels.as_chunks_mut::<4>();
    debug_assert!(
        remainder.is_empty(),
        "RGBA data is a whole number of pixels"
    );
    for [r, g, b, _alpha] in quads {
        *r = ink;
        *g = ink;
        *b = ink;
    }
}

/// Decodes the tray artwork and paints it in `ink`. Returns `None` if the
/// embedded PNG cannot be read, in which case the caller falls back to the
/// application icon.
fn tray_icon(ink: u8) -> Option<Image<'static>> {
    let decoded = Image::from_bytes(TRAY_ICON_PNG).ok()?;
    let (width, height) = (decoded.width(), decoded.height());
    let mut pixels = decoded.rgba().to_vec();
    repaint(&mut pixels, ink);
    Some(Image::new_owned(pixels, width, height))
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

    if let Some(icon) = tray_icon(ink_for(app)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repaint_changes_colour_but_keeps_the_shape() {
        // The alpha is the only thing that carries the outline; repainting it
        // would either erase the glyph or turn it into a filled square.
        let mut pixels = vec![0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 255];
        repaint(&mut pixels, 255);

        assert_eq!(
            &pixels[0..4],
            &[255, 255, 255, 0],
            "transparent stays transparent"
        );
        assert_eq!(
            &pixels[4..8],
            &[255, 255, 255, 128],
            "semi-transparent keeps its alpha"
        );
        assert_eq!(
            &pixels[8..12],
            &[255, 255, 255, 255],
            "opaque becomes white"
        );
    }

    #[test]
    fn dark_ink_is_reachable_too() {
        // A light taskbar needs the glyph drawn in ink, not merely in white.
        let mut pixels = vec![255, 255, 255, 255];
        repaint(&mut pixels, 0);
        assert_eq!(pixels, vec![0, 0, 0, 255]);
    }

    #[test]
    fn the_embedded_artwork_decodes_and_has_the_expected_shape() {
        let icon = tray_icon(255).expect("the tray PNG is embedded in the binary");
        assert_eq!(icon.rgba().len() % 4, 0, "RGBA is four bytes per pixel");
        assert!(icon.width() > 0 && icon.height() > 0);
    }
}

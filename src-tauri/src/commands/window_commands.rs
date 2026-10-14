use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::error::TaskError;

const DEFAULT_HEIGHT: f64 = 600.0;
const DEFAULT_WIDTH: f64 = 420.0;
/// Heuristic: if the window is already (almost) full height, collapse back.
const FULL_HEIGHT_TOLERANCE: f64 = 40.0;

/// Toggles the window between its default size and the full size of the
/// current monitor (snapped to the top-left of the work area). Bound to a
/// double click on the logo.
#[tauri::command]
pub fn toggle_window_height(window: WebviewWindow) -> Result<(), TaskError> {
    let scale = window
        .scale_factor()
        .map_err(|e| TaskError::Internal(e.to_string()))?;
    let size = window
        .outer_size()
        .map_err(|e| TaskError::Internal(e.to_string()))?;
    let position = window
        .outer_position()
        .map_err(|e| TaskError::Internal(e.to_string()))?;
    let current_height = size.height as f64 / scale;
    let current_width = size.width as f64 / scale;

    let monitor = window
        .current_monitor()
        .map_err(|e| TaskError::Internal(e.to_string()))?;
    let (monitor_size, monitor_position) = match monitor.as_ref() {
        Some(m) => (*m.size(), *m.position()),
        None => (
            PhysicalSize::new(1920u32, 1080u32),
            PhysicalPosition::new(0i32, 0i32),
        ),
    };
    let monitor_width = monitor_size.width as f64 / scale;
    let monitor_height = monitor_size.height as f64 / scale;

    let full = current_height >= monitor_height - FULL_HEIGHT_TOLERANCE;
    let target_height = if full { DEFAULT_HEIGHT } else { monitor_height };
    let target_width = if full {
        DEFAULT_WIDTH
    } else {
        // Grow the width along with the height, but never wider than the monitor.
        (current_width.max(DEFAULT_WIDTH) * 1.6).min(monitor_width)
    };

    window
        .set_size(PhysicalSize::new(
            (target_width * scale) as u32,
            (target_height * scale) as u32,
        ))
        .map_err(|e| TaskError::Internal(e.to_string()))?;

    // Growing downwards/rightwards could push the window off-screen: keep the
    // top-left corner inside the monitor.
    let min_x = monitor_position.x;
    let min_y = monitor_position.y;
    let max_x = min_x + monitor_size.width as i32 - 120;
    let max_y = min_y + monitor_size.height as i32 - 80;
    let x = position.x.clamp(min_x, max_x);
    let y = position.y.clamp(min_y, max_y);
    if x != position.x || y != position.y {
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| TaskError::Internal(e.to_string()))?;
    }

    Ok(())
}

use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::error::TaskError;

const DEFAULT_HEIGHT: f64 = 600.0;
const DEFAULT_WIDTH: f64 = 420.0;
/// A window this close to the full work area counts as already expanded.
const FULL_HEIGHT_TOLERANCE: f64 = 40.0;

/// A rectangle in physical pixels, the unit the window API speaks.
#[derive(Clone, Copy, Debug)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Size and position for one step of the height toggle.
///
/// The size is in logical pixels because it is derived from the work area by
/// dividing out the scale factor; the position stays in physical pixels
/// because that is what the window API takes.
#[derive(Debug, PartialEq)]
struct Placement {
    width: f64,
    height: f64,
    x: i32,
    y: i32,
}

/// The furthest origin a window of `size` may take so that all of it stays
/// inside the stretch that starts at `origin` and is `available` long.
///
/// The `max` guards a work area smaller than the window: without it `clamp`
/// would be called with a minimum above its maximum and panic.
fn clamp_origin(current: i32, origin: i32, available: i32, size: i32) -> i32 {
    let last = (origin + available - size).max(origin);
    current.clamp(origin, last)
}

/// Decides the window size and position for one toggle step.
///
/// Pure, so the geometry can be tested without a window server. Everything
/// that comes in is in physical pixels; the size that comes out is logical,
/// since that is what the command hands back to the window API after scaling.
fn compute_placement(window: Rect, work: Rect, scale: f64) -> Placement {
    let work_w = work.width as f64 / scale;
    let work_h = work.height as f64 / scale;
    let expanded = window.height as f64 / scale >= work_h - FULL_HEIGHT_TOLERANCE;

    let height = if expanded { DEFAULT_HEIGHT } else { work_h };
    let width = if expanded {
        DEFAULT_WIDTH
    } else {
        // Grow the width along with the height, but never past the work area.
        ((window.width as f64 / scale).max(DEFAULT_WIDTH) * 1.6).min(work_w)
    };
    let physical_w = (width * scale) as i32;
    let physical_h = (height * scale) as i32;

    // The position is clamped so that the whole window fits, not merely its
    // top-left corner. Clamping only the corner leaves a window that starts low
    // on the screen growing straight off the bottom, which is what the corner
    // check was meant to prevent.
    Placement {
        width,
        height,
        x: clamp_origin(window.x, work.x, work.width as i32, physical_w),
        y: clamp_origin(window.y, work.y, work.height as i32, physical_h),
    }
}

/// Toggles the window between its default size and the full height of the
/// current monitor. Bound to a double click on the logo.
///
/// The full height is the work area, not the monitor: on Windows the taskbar
/// would cover the bottom of the window, and on macOS the menu bar would push
/// the top of it off the screen.
#[tauri::command]
pub fn toggle_window_height(window: WebviewWindow) -> Result<(), TaskError> {
    let map_err = |e: tauri::Error| TaskError::Internal(e.to_string());

    let scale = window.scale_factor().map_err(map_err)?;
    let size = window.outer_size().map_err(map_err)?;
    let position = window.outer_position().map_err(map_err)?;

    let current = Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };

    let work = match window.current_monitor().map_err(map_err)? {
        Some(monitor) => {
            let area = *monitor.work_area();
            Rect {
                x: area.position.x,
                y: area.position.y,
                width: area.size.width,
                height: area.size.height,
            }
        }
        // No monitor to measure against; the caller still gets a sane toggle.
        None => Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
    };

    let placement = compute_placement(current, work, scale);

    // Size first, then position: the window manager can undo a move that is
    // still oversized, so the window is fitted before it is placed.
    window
        .set_size(PhysicalSize::new(
            (placement.width * scale) as u32,
            (placement.height * scale) as u32,
        ))
        .map_err(map_err)?;

    if placement.x != current.x || placement.y != current.y {
        window
            .set_position(PhysicalPosition::new(placement.x, placement.y))
            .map_err(map_err)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN_W: u32 = 1920;
    const SCREEN_H: u32 = 1080;
    /// The Windows taskbar along the bottom of a 1920x1080 screen.
    const TASKBAR: u32 = 48;
    /// The macOS menu bar along the top, which pushes the work area down.
    const MENU_BAR: u32 = 25;

    /// Toggles a window of a given physical size and corner against a work
    /// area spanning the full screen width, at 1.0 scaling.
    fn toggle(w: u32, h: u32, x: i32, y: i32, work_y: i32, work_h: u32) -> Placement {
        compute_placement(
            Rect {
                x,
                y,
                width: w,
                height: h,
            },
            Rect {
                x: 0,
                y: work_y,
                width: SCREEN_W,
                height: work_h,
            },
            1.0,
        )
    }

    fn bottom(p: &Placement, scale: f64) -> i32 {
        p.y + (p.height * scale) as i32
    }

    #[test]
    fn expanding_moves_the_top_up_so_the_bottom_stays_on_screen() {
        // The reported failure: a window low on the screen grew downwards and
        // the part below the bottom edge could not be reached.
        let p = toggle(420, 600, 700, 400, 0, SCREEN_H - TASKBAR);
        assert_eq!(p.y, 0, "the top must move up to the work area");
        assert_eq!(bottom(&p, 1.0), (SCREEN_H - TASKBAR) as i32);
    }

    #[test]
    fn expanded_height_stops_above_the_taskbar() {
        let p = toggle(420, 600, 700, 400, 0, SCREEN_H - TASKBAR);
        assert_eq!(
            p.height,
            (SCREEN_H - TASKBAR) as f64,
            "not the full monitor"
        );
    }

    #[test]
    fn expanded_height_starts_below_the_menu_bar() {
        let p = toggle(420, 600, 700, 0, MENU_BAR as i32, SCREEN_H - MENU_BAR);
        assert_eq!(p.y, MENU_BAR as i32);
        assert_eq!(bottom(&p, 1.0), SCREEN_H as i32);
    }

    #[test]
    fn a_window_already_at_the_top_keeps_its_position() {
        let p = toggle(420, 600, 700, 0, 0, SCREEN_H - TASKBAR);
        assert_eq!((p.x, p.y), (700, 0));
    }

    #[test]
    fn widening_past_the_right_edge_pulls_the_window_left() {
        let p = toggle(420, 600, 1800, 0, 0, SCREEN_H - TASKBAR);
        assert_eq!(
            p.x + p.width as i32,
            SCREEN_W as i32,
            "the right edge must land on the work area"
        );
    }

    #[test]
    fn collapsing_keeps_a_position_that_still_fits() {
        // Already at the work area height, so this counts as expanded.
        let p = toggle(420, SCREEN_H - TASKBAR, 700, 300, 0, SCREEN_H - TASKBAR);
        assert_eq!(p.height, DEFAULT_HEIGHT);
        assert_eq!(p.y, 300, "collapsing must not move a window that fits");
    }

    #[test]
    fn collapsing_pulls_back_a_window_hanging_off_the_bottom() {
        // Expanded, but dragged down so its bottom hangs past the work area.
        let p = toggle(420, SCREEN_H - TASKBAR, 700, 900, 0, SCREEN_H - TASKBAR);
        assert_eq!(p.height, DEFAULT_HEIGHT);
        assert_eq!(bottom(&p, 1.0), (SCREEN_H - TASKBAR) as i32);
    }

    #[test]
    fn a_work_area_smaller_than_the_window_does_not_panic() {
        // clamp(min, max) panics when min > max, which a short screen combined
        // with the fixed default height can otherwise produce.
        let p = toggle(420, 300, 0, 0, 0, 200);
        assert_eq!(p.y, 0);
    }

    #[test]
    fn high_dpi_keeps_the_window_inside_the_screen() {
        // A Retina screen: 2880x1800 physical pixels reported as 1440x900
        // logical ones. Mixing logical and physical units here would let the
        // bottom hang off the screen by exactly the scale factor.
        let p = compute_placement(
            Rect {
                x: 900,
                y: 1000,
                width: 840,
                height: 1200,
            },
            Rect {
                x: 0,
                y: 0,
                width: 2880,
                height: 1800,
            },
            2.0,
        );
        assert_eq!(p.height, 900.0, "the work area height in logical units");
        assert_eq!(bottom(&p, 2.0), 1800);
    }

    #[test]
    fn high_dpi_pulls_a_collapsed_window_back_on_screen() {
        // Expanded and dragged low, then collapsed: the shorter window has room
        // to move, and the bottom must still land on the work area.
        let p = compute_placement(
            Rect {
                x: 900,
                y: 1000,
                width: 840,
                height: 1800,
            },
            Rect {
                x: 0,
                y: 0,
                width: 2880,
                height: 1800,
            },
            2.0,
        );
        assert_eq!(p.height, DEFAULT_HEIGHT);
        assert_eq!(bottom(&p, 2.0), 1800);
    }
}

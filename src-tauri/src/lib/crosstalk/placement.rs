// Path: src-tauri/src/lib/crosstalk/placement.rs
// Description: Monitor-aware physical placement of the owned speech-bubble window

use super::{CrosstalkState, LABEL};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

fn visibility(window: &tauri::WebviewWindow, visible: bool, raise: bool) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            IsWindowVisible, SetWindowPos, ShowWindow, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
            SWP_SHOWWINDOW, SW_HIDE,
        };
        let hwnd = window.hwnd()?.0 as isize;
        // Windows can restore owned popups independently of Tao's cached visibility flag.
        window.run_on_main_thread(move || unsafe {
            if visible {
                let flags = SWP_NOACTIVATE
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOOWNERZORDER
                    | SWP_SHOWWINDOW
                    | if raise { 0 } else { SWP_NOZORDER };
                let position = |after| {
                    if SetWindowPos(hwnd as _, after, 0, 0, 0, 0, flags) == 0 {
                        super::report(&format!(
                            "Could not show Crosstalk: {}",
                            std::io::Error::last_os_error()
                        ));
                    }
                };
                // Promotion is immediately undone; clicks must be able to cover the bubble.
                if raise {
                    position(HWND_TOPMOST);
                }
                position(if raise { HWND_NOTOPMOST } else { HWND_TOP });
            } else {
                ShowWindow(hwnd as _, SW_HIDE);
            }
            if let Err(error) = super::pointer::track(hwnd as _, IsWindowVisible(hwnd as _) != 0) {
                super::report(&error);
            }
        })
    }
    #[cfg(not(windows))]
    {
        let _ = raise;
        if visible {
            window.show()
        } else {
            window.hide()
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub side: &'static str,
}

pub fn place(main: [i32; 4], work: [i32; 4], scale: f64, height: f64) -> Placement {
    let gap = (10.0 * scale).round() as i32;
    let preferred = (356.0 * scale).round() as i32;
    let left = main[0] - work[0];
    let right = work[0] + work[2] - main[0] - main[2];
    let outside = left.max(right) >= preferred + gap;
    let width = if outside {
        preferred
    } else {
        preferred.min(main[2] - 2 * gap).max(1)
    };
    let (x, side) = if outside && left > right {
        (main[0] - gap - width, "left")
    } else if outside {
        (main[0] + main[2] + gap, "right")
    } else {
        (main[0] + main[2] - width - gap, "inside")
    };
    let available = (work[3] - 2 * gap)
        .min(main[3] - (105.0 * scale) as i32)
        .max(1);
    let height = ((height * scale).ceil() as i32).clamp(1, available);
    let y = (main[1] + (92.0 * scale) as i32).clamp(
        work[1] + gap,
        (work[1] + work[3] - height - gap).max(work[1] + gap),
    );
    Placement {
        x: x.clamp(work[0], (work[0] + work[2] - width).max(work[0])),
        y,
        width: width as u32,
        height: height as u32,
        side,
    }
}

pub fn layout(app: &AppHandle, raise: bool) -> Result<(), String> {
    let state = app.state::<CrosstalkState>();
    let _layout = state
        .layout
        .lock()
        .map_err(|_| "Crosstalk layout lock failed")?;
    let Some(bubbles) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let Some(main) = app.get_webview_window("main") else {
        return Ok(());
    };
    let visible = main.is_visible().map_err(|e| e.to_string())?
        && !main.is_minimized().map_err(|e| e.to_string())?;
    let (active, height) = {
        let mut view = state
            .view
            .lock()
            .map_err(|_| "Crosstalk view lock failed")?;
        if view.window_visible != visible {
            view.window_visible = visible;
            if !visible {
                view.height = 0.0;
            }
            app.emit_to(
                LABEL,
                "crosstalk-presentation",
                view.effective_presentation(),
            )
            .map_err(|e| e.to_string())?;
        }
        (
            view.effective_presentation().active && view.ready,
            view.height,
        )
    };
    let result = (|| -> tauri::Result<()> {
        if !active || height < 1.0 {
            return visibility(&bubbles, false, false);
        }
        let Some(monitor) = main.current_monitor()? else {
            return visibility(&bubbles, false, false);
        };
        let pos = main.outer_position()?;
        let size = main.outer_size()?;
        let work = monitor.work_area();
        let rect = place(
            [pos.x, pos.y, size.width as i32, size.height as i32],
            [
                work.position.x,
                work.position.y,
                work.size.width as i32,
                work.size.height as i32,
            ],
            main.scale_factor()?,
            height,
        );
        bubbles.emit("crosstalk-side", rect.side)?;
        bubbles.set_size(PhysicalSize::new(rect.width, rect.height))?;
        bubbles.set_position(PhysicalPosition::new(rect.x, rect.y))?;
        visibility(&bubbles, true, raise)
    })();
    result.map_err(|e| format!("Could not position Crosstalk bubbles: {e}"))
}

#[cfg(test)]
mod tests {
    use super::place;

    #[test]
    fn placement_uses_free_side_and_contains_maximized_and_negative_monitor() {
        let work = [0, 0, 2560, 1400];
        assert_eq!(
            place([1300, 400, 1100, 800], work, 1.25, 300.0).side,
            "left"
        );
        assert_eq!(place([0, 200, 1000, 800], work, 1.25, 300.0).side, "right");
        let p = place([-8, -8, 2576, 1416], work, 1.25, 3000.0);
        assert_eq!(p.side, "inside");
        assert!(
            p.x >= 0 && p.y >= 0 && p.x + p.width as i32 <= 2560 && p.y + p.height as i32 <= 1400
        );
        let p = place([-1900, 100, 900, 700], [-1920, 0, 1920, 1040], 1.0, 200.0);
        assert_eq!(p.side, "right");
        assert!(p.x < 0 && p.x + p.width as i32 <= 0);
        let p = place(
            [-1088, 1280, 1096, 640],
            [-1080, 0, 1080, 1920],
            1.0,
            2000.0,
        );
        assert_eq!(p.side, "inside");
        assert!(p.x >= -1080 && p.x + p.width as i32 <= 0);
        assert!(p.y >= 1280 && p.y + p.height as i32 <= 1920);
    }
}

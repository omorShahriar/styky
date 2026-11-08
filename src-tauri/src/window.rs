//! Panel window behaviour: glass material, z-order, positioning, show/hide.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

pub const MAIN: &str = "main";

/// Apply Mica (Windows 11 22H2+), falling back to Acrylic, then to the plain
/// CSS tint. Returns the name of the material that was applied.
pub fn apply_material(window: &WebviewWindow) -> &'static str {
    #[cfg(windows)]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        if apply_mica(window, None).is_ok() {
            return "mica";
        }
        if apply_acrylic(window, Some((24, 24, 26, 80))).is_ok() {
            return "acrylic";
        }
    }
    let _ = window;
    "none"
}

/// Place the panel in the top-right of the primary monitor (first run only).
pub fn place_top_right(window: &WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let scale = monitor.scale_factor();
    let margin = (24.0 * scale) as i32;
    let work = monitor.work_area();
    let x = work.position.x + work.size.width as i32 - size.width as i32 - margin;
    let y = work.position.y + margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Push the panel to the bottom of the z-order (desktop layer).
pub fn sink(window: &WebviewWindow) {
    let _ = window.set_always_on_bottom(true);
}

pub fn show_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.hide();
    }
}

pub fn toggle_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        if w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_panel(app);
        }
    }
}

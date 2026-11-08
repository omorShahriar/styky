//! The Settings window (`settings`): a regular, hidden-on-close window.

use tauri::{AppHandle, Manager};

pub const LABEL: &str = "settings";

/// Show (and focus) the Settings window. It is an ordinary app window, so the
/// Show-Desktop watcher already treats it as "an app is visible"; it is not topmost.
pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        if !w.is_visible().unwrap_or(false) {
            let _ = w.center();
        }
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

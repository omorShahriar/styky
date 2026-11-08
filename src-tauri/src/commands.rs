//! `#[tauri::command]` API used by the UI.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::{theme, window};

#[tauri::command]
pub fn get_accent() -> String {
    theme::accent()
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Enable/disable launch at login. Returns the resulting state.
pub fn apply_autostart(app: &AppHandle, enabled: bool) -> bool {
    let launcher = app.autolaunch();
    let _ = if enabled { launcher.enable() } else { launcher.disable() };
    launcher.is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> bool {
    apply_autostart(&app, enabled)
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    window::hide_panel(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

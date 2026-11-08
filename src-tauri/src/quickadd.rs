//! Quick-add bar: the `quickadd` window and its global hotkey.

use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::db::Db;

pub const LABEL: &str = "quickadd";
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+N";
/// Logical size of the bar when it shows no chips.
pub const WIDTH: f64 = 560.0;
pub const HEIGHT: f64 = 64.0;

fn saved_hotkey(app: &AppHandle) -> String {
    app.state::<Db>()
        .settings()
        .ok()
        .and_then(|s| s.get("quickadd_hotkey").cloned())
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HOTKEY.to_string())
}

fn register(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut: Shortcut = hotkey.parse().map_err(|e| format!("bad hotkey {hotkey:?}: {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                toggle(app);
            }
        })
        .map_err(|e| format!("could not register {hotkey}: {e}"))
}

/// Register the saved hotkey at startup (a failure is logged, not fatal).
pub fn init(app: &AppHandle) {
    let hotkey = saved_hotkey(app);
    if let Err(e) = register(app, &hotkey) {
        eprintln!("styky: {e}");
    }
}

/// Change the hotkey: register the new one first so a bad value keeps the old one.
pub fn rebind(app: &AppHandle, new: &str) -> Result<(), String> {
    let old = saved_hotkey(app);
    if old.eq_ignore_ascii_case(new) {
        return Ok(());
    }
    let gs = app.global_shortcut();
    if let Ok(old_sc) = old.parse::<Shortcut>() {
        let _ = gs.unregister(old_sc);
    }
    if let Err(e) = register(app, new) {
        let _ = register(app, &old);
        return Err(e);
    }
    app.state::<Db>().set_setting("quickadd_hotkey", new)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// Apply the glass material once, at startup.
pub fn setup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        crate::window::apply_material(&w);
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    if w.is_visible().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}

/// Show the bar centred (horizontally) on the monitor under the cursor.
pub fn show(app: &AppHandle) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    // Our own always-on-top window: keep the Show-Desktop watcher off while it is up.
    #[cfg(windows)]
    crate::desktop::suspend(true);
    let _ = w.set_size(LogicalSize::new(WIDTH, HEIGHT));
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (monitor, w.outer_size()) {
        let work = m.work_area();
        let x = work.position.x + (work.size.width as i32 - size.width as i32) / 2;
        let y = work.position.y + (work.size.height as f64 * 0.28) as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("quickadd-shown", ());
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
    #[cfg(windows)]
    crate::desktop::suspend(false);
}

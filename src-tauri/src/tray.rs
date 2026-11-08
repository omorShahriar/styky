//! System tray: left-click toggles the panel; menu offers Show/Hide, Settings, Quit.

use tauri::{
    AppHandle,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt;

use crate::window;

const PAUSE_LABEL: &str = "Pause reminders 1h";

fn clock(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Show / Hide", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quickadd", "Quick add	Ctrl+Alt+N", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", PAUSE_LABEL, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Launch at login",
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Styky", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quick, &pause, &settings, &autostart, &sep, &quit])?;

    let autostart_item = autostart.clone();
    let pause_item = pause.clone();
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Styky")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => window::toggle_panel(app),
            "quickadd" => crate::quickadd::toggle(app),
            "pause" => {
                let until = crate::scheduler::toggle_pause(app);
                let _ = pause_item.set_text(match until {
                    Some(t) => format!("Resume reminders (paused until {})", clock(t)),
                    None => PAUSE_LABEL.to_string(),
                });
            }
            "settings" => crate::settings_win::show(app),
            "autostart" => {
                let on = crate::commands::apply_autostart(app, autostart_item.is_checked().unwrap_or(false));
                let _ = autostart_item.set_checked(on);
            }
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
                window::toggle_panel(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

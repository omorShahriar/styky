//! System tray: left-click toggles the panel; menu offers Show/Hide, Settings, Quit.

use tauri::{
    AppHandle, Emitter,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt;

use crate::window;

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Show / Hide", true, None::<&str>)?;
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
    let menu = Menu::with_items(app, &[&toggle, &settings, &autostart, &sep, &quit])?;

    let autostart_item = autostart.clone();
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Styky")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => window::toggle_panel(app),
            "settings" => {
                // Placeholder until the Settings window lands (M4).
                window::show_panel(app);
                let _ = app.emit("open-settings", ());
            }
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

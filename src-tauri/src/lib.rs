mod commands;
mod theme;
mod tray;
mod window;

use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::StateFlags;

pub fn run() {
    // Visibility/decorations are owned by us, not restored from disk.
    let state_flags = StateFlags::all() & !StateFlags::VISIBLE & !StateFlags::DECORATIONS;

    tauri::Builder::default()
        // A second launch just brings the existing panel forward.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::show_panel(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(state_flags)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_accent,
            commands::get_autostart,
            commands::set_autostart,
            commands::hide_panel,
            commands::quit,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let win = app
                .get_webview_window(window::MAIN)
                .expect("main window is declared in tauri.conf.json");

            let first_run = !has_saved_state(&handle);
            let material = window::apply_material(&win);
            if material == "none" {
                eprintln!("styky: no vibrancy material available, using CSS tint only");
            }
            if first_run {
                window::place_top_right(&win);
            }
            window::sink(&win);
            let _ = win.show();
            window::sink(&win);

            if let Some(watcher) = theme::watch(&handle) {
                app.manage(watcher);
            }
            tray::build(&handle)?;
            Ok(())
        })
        .on_window_event(|win, event| {
            if win.label() != window::MAIN {
                return;
            }
            match event {
                // Close = hide. Quit is only available from the tray / menu.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = win.hide();
                }
                // Drop back to the desktop layer as soon as focus is lost.
                WindowEvent::Focused(false) => {
                    if let Some(w) = win.app_handle().get_webview_window(window::MAIN) {
                        window::sink(&w);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Styky");
}

fn has_saved_state(app: &tauri::AppHandle) -> bool {
    app.path()
        .app_config_dir()
        .map(|d| d.join(".window-state.json").exists())
        .unwrap_or(false)
}

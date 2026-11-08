pub mod api;
pub mod commands;
pub mod db;
#[cfg(windows)]
pub mod desktop;
pub mod license;
pub mod model;
pub mod nlp;
pub mod nlp_repeat;
pub mod proto;
pub mod notify;
pub mod quickadd;
pub mod repeat;
pub mod scheduler;
pub mod settings_win;
pub mod snooze;
pub mod store;
pub mod sort;
pub mod theme;
pub mod tray;
pub mod window;

use tauri::{Manager, WindowEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Bumped on every move/resize; lets the debounced saver know if it is stale.
static MOVE_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn run() {
    let flags = state_flags();

    let mut builder = tauri::Builder::default();
    // The Store updates packaged installs itself; the self-updater is only for the NSIS/GitHub build.
    if !store::is_store() {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    builder
        // A second launch just brings the existing panel forward.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::show_panel(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(flags)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_accent,
            commands::get_autostart,
            commands::set_autostart,
            commands::autostart_blocked,
            commands::is_store,
            commands::hide_panel,
            commands::quit,
            commands::quickadd_hotkey_error,
            commands::open_settings,
            commands::open_settings_pro,
            commands::api_status,
            commands::list_items,
            commands::create_item,
            commands::update_item,
            commands::delete_item,
            commands::undelete_item,
            commands::complete_item,
            commands::restore_item,
            commands::snooze_item,
            commands::reorder,
            commands::get_settings,
            commands::set_setting,
            commands::export_json,
            commands::import_json,
            commands::hide_quickadd,
            commands::set_quickadd_hotkey,
            commands::license_status,
            commands::license_activate,
            commands::license_remove,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let db_path = app.path().app_data_dir()?.join("styky.db");
            app.manage(db::Db::open(&db_path).map_err(|e| format!("cannot open {db_path:?}: {e}"))?);
            first_run_autostart(&handle);
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
            #[cfg(windows)]
            desktop::spawn_watcher(handle.clone());
            tray::build(&handle)?;
            quickadd::setup(&handle);
            quickadd::init(&handle);
            scheduler::spawn(handle.clone());
            spawn_license_check(handle.clone());
            app.manage(std::sync::Arc::new(api::ApiStatus::default()));
            spawn_api(handle.clone());
            Ok(())
        })
        .on_window_event(|win, event| {
            if win.label() == settings_win::LABEL {
                // Close = hide; the window is reused.
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win.hide();
                }
                return;
            }
            if win.label() == quickadd::LABEL {
                // Blur hides the bar (Esc / Enter are handled in the page).
                if let WindowEvent::Focused(false) = event {
                    quickadd::hide(win.app_handle());
                }
                return;
            }
            if win.label() != window::MAIN {
                return;
            }
            match event {
                // Close = hide. Quit is only available from the tray / menu.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = win.app_handle().save_window_state(state_flags());
                    let _ = win.hide();
                }
                // Persist size/position shortly after the user stops dragging.
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    let seq = MOVE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
                    let app = win.app_handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(600));
                        if MOVE_SEQ.load(Ordering::SeqCst) == seq {
                            let _ = app.save_window_state(state_flags());
                        }
                    });
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

/// Serve the local API on the per-user named pipe (see `api.rs`).
fn spawn_api(app: tauri::AppHandle) {
    #[cfg(windows)]
    tauri::async_runtime::spawn(async move {
        let handler_app = app.clone();
        let handler: std::sync::Arc<dyn Fn(&str) -> String + Send + Sync> = std::sync::Arc::new(move |line| {
            let db = handler_app.state::<db::Db>();
            let ctx = api::Ctx { db: &db, now: commands::now_ms(), pro: license::is_pro() };
            let (out, changed) = api::handle_line(&ctx, line);
            if changed {
                let _ = tauri::Emitter::emit(&handler_app, "items-changed", ());
            }
            out
        });
        let status = app.state::<std::sync::Arc<api::ApiStatus>>().inner().clone();
        if let Err(e) = api::serve(proto::pipe_name(), handler, status).await {
            eprintln!("styky: local API is not available: {e}");
        }
    });
    #[cfg(not(windows))]
    let _ = app;
}

/// Load the cached entitlement, then re-check it (if due) at start and every 6 hours.
fn spawn_license_check(app: tauri::AppHandle) {
    license::refresh(&app.state::<db::Db>(), commands::now_ms());
    std::thread::spawn(move || {
        loop {
            license::background_check(&app);
            std::thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

/// Release builds turn "launch at login" on once, the first time they run; after that the user's choice wins.
fn first_run_autostart(app: &tauri::AppHandle) {
    // Packaged: the manifest startup task is already enabled by default; never override the user.
    if tauri::is_dev() || store::is_packaged() {
        return;
    }
    let db = app.state::<db::Db>();
    let seen = db.settings().map(|s| s.contains_key("autostart_initialized")).unwrap_or(true);
    if !seen {
        commands::apply_autostart(app, true);
        let _ = db.set_setting("autostart_initialized", "1");
    }
}

/// Visibility/decorations are owned by us, not restored from disk.
fn state_flags() -> StateFlags {
    StateFlags::all() & !StateFlags::VISIBLE & !StateFlags::DECORATIONS
}

fn has_saved_state(app: &tauri::AppHandle) -> bool {
    app.path()
        .app_config_dir()
        .map(|d| d.join(".window-state.json").exists())
        .unwrap_or(false)
}

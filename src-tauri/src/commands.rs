//! `#[tauri::command]` API used by the UI.

use std::collections::BTreeMap;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;

use crate::db::{Db, Res};
use crate::model::{ExportFile, ImportResult, Item, ItemEdit};
use crate::{theme, window};

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Tell every window that the item list changed.
fn changed(app: &AppHandle) {
    let _ = app.emit("items-changed", ());
}

// ---------------------------------------------------------------- items

#[tauri::command]
pub fn list_items(app: AppHandle, db: State<Db>) -> Res<Vec<Item>> {
    let now = now_ms();
    // Auto-clear old completed items lazily; cheap, and keeps the list honest.
    let days = db
        .settings()?
        .get("autoclear_days")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(7);
    if db.purge_completed(days, now)? > 0 {
        changed(&app);
    }
    db.list(now)
}

#[tauri::command]
pub fn create_item(app: AppHandle, db: State<Db>, edit: ItemEdit) -> Res<Item> {
    let item = db.create(&edit, now_ms())?;
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn update_item(app: AppHandle, db: State<Db>, id: String, edit: ItemEdit) -> Res<Item> {
    let item = db.update(&id, &edit, now_ms())?;
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn delete_item(app: AppHandle, db: State<Db>, id: String) -> Res<()> {
    db.delete(&id)?;
    changed(&app);
    Ok(())
}

/// Undo a delete: put the exact item back.
#[tauri::command]
pub fn undelete_item(app: AppHandle, db: State<Db>, item: Item) -> Res<()> {
    db.undelete(&item)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn complete_item(app: AppHandle, db: State<Db>, id: String) -> Res<Item> {
    let item = db.complete(&id, now_ms())?;
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn restore_item(app: AppHandle, db: State<Db>, id: String) -> Res<Item> {
    let item = db.restore(&id, now_ms())?;
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn snooze_item(app: AppHandle, db: State<Db>, id: String, until: i64) -> Res<Item> {
    let item = db.snooze(&id, until, now_ms())?;
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn reorder(app: AppHandle, db: State<Db>, ids: Vec<String>) -> Res<()> {
    db.reorder(&ids)?;
    changed(&app);
    Ok(())
}

// ------------------------------------------------------------- settings

#[tauri::command]
pub fn get_settings(db: State<Db>) -> Res<BTreeMap<String, String>> {
    db.settings()
}

#[tauri::command]
pub fn set_setting(app: AppHandle, db: State<Db>, key: String, value: String) -> Res<()> {
    db.set_setting(&key, &value)?;
    let _ = app.emit("settings-changed", ());
    changed(&app);
    Ok(())
}

// -------------------------------------------------------- export / import

#[tauri::command]
pub fn export_json(db: State<Db>, path: String) -> Res<usize> {
    let file = db.export(now_ms())?;
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(file.items.len())
}

/// `mode` is `"merge"` or `"replace"`.
#[tauri::command]
pub fn import_json(app: AppHandle, db: State<Db>, path: String, mode: String) -> Res<ImportResult> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: ExportFile =
        serde_json::from_str(&text).map_err(|e| format!("not a Styky export: {e}"))?;
    let result = db.import(&file, mode == "replace")?;
    let _ = app.emit("settings-changed", ());
    changed(&app);
    Ok(result)
}

// ---------------------------------------------------------------- system

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
pub fn hide_quickadd(app: AppHandle) {
    crate::quickadd::hide(&app);
}

/// Rebind the quick-add hotkey (e.g. "Ctrl+Alt+N"); errors if it cannot be registered.
#[tauri::command]
pub fn set_quickadd_hotkey(app: AppHandle, hotkey: String) -> Res<()> {
    crate::quickadd::rebind(&app, hotkey.trim())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

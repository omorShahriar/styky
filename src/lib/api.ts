import { invoke } from "@tauri-apps/api/core";
import type { LicenseStatus } from "./pro";
import type { ImportResult, Item, ItemEdit } from "./types";

export const api = {
  // window / system
  hidePanel: () => invoke<void>("hide_panel"),
  hideQuickadd: () => invoke<void>("hide_quickadd"),
  setQuickaddHotkey: (hotkey: string) => invoke<void>("set_quickadd_hotkey", { hotkey }),
  openSettings: () => invoke<void>("open_settings"),
  openSettingsPro: () => invoke<void>("open_settings_pro"),
  /** Why the local API (CLI / MCP) is not serving, or null when it is fine. */
  apiStatus: () => invoke<string | null>("api_status"),
  licenseStatus: () => invoke<LicenseStatus>("license_status"),
  licenseActivate: (key: string) => invoke<LicenseStatus>("license_activate", { key }),
  licenseRemove: () => invoke<boolean>("license_remove"),
  quickaddHotkeyError: () => invoke<string | null>("quickadd_hotkey_error"),
  quit: () => invoke<void>("quit"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  autostartBlocked: () => invoke<boolean>("autostart_blocked"),
  isStore: () => invoke<boolean>("is_store"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),

  getAccent: () => invoke<string>("get_accent"),

  // items
  listItems: () => invoke<Item[]>("list_items"),
  createItem: (edit: ItemEdit) => invoke<Item>("create_item", { edit }),
  updateItem: (id: string, edit: ItemEdit) => invoke<Item>("update_item", { id, edit }),
  deleteItem: (id: string) => invoke<void>("delete_item", { id }),
  undeleteItem: (item: Item) => invoke<void>("undelete_item", { item }),
  completeItem: (id: string) => invoke<Item>("complete_item", { id }),
  restoreItem: (id: string) => invoke<Item>("restore_item", { id }),
  snoozeItem: (id: string, until: number) => invoke<Item>("snooze_item", { id, until }),
  reorder: (ids: string[]) => invoke<void>("reorder", { ids }),

  // settings
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),

  // export / import
  exportJson: (path: string) => invoke<number>("export_json", { path }),
  importJson: (path: string, mode: "merge" | "replace") =>
    invoke<ImportResult>("import_json", { path, mode }),
};

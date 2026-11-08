import { invoke } from "@tauri-apps/api/core";
import type { ImportResult, Item, ItemEdit } from "./types";

export const api = {
  // window / system
  hidePanel: () => invoke<void>("hide_panel"),
  quit: () => invoke<void>("quit"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),

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

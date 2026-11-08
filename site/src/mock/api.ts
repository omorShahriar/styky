/** Drop-in replacement for src/lib/api.ts (same shape), backed by the in-memory db. */
import type { ImportResult, Item, ItemEdit } from "../../../src/lib/types";
import { db } from "./db";

const ok = <T>(v: T) => Promise.resolve(v);
const noop = () => ok(undefined as void);

export const api = {
  // window / system: nothing to do on the web
  hidePanel: noop,
  hideQuickadd: noop,
  setQuickaddHotkey: (_hotkey: string) => noop(),
  openSettings: noop,
  quickaddHotkeyError: () => ok<string | null>(null),
  quit: noop,
  getAutostart: () => ok(false),
  autostartBlocked: () => ok(false),
  isStore: () => ok(false),
  setAutostart: (enabled: boolean) => ok(enabled),

  // items
  listItems: () => ok<Item[]>(db.list()),
  createItem: (edit: ItemEdit) => ok(db.create(edit)),
  updateItem: (id: string, edit: ItemEdit) => ok(db.update(id, edit)),
  deleteItem: (id: string) => ok(db.remove(id)),
  undeleteItem: (item: Item) => ok(db.undelete(item)),
  completeItem: (id: string) => ok(db.complete(id)),
  restoreItem: (id: string) => ok(db.restore(id)),
  snoozeItem: (id: string, until: number) => ok(db.snooze(id, until)),
  reorder: (ids: string[]) => ok(db.reorder(ids)),

  // settings
  getSettings: () => ok(db.settings()),
  setSetting: (key: string, value: string) => ok(db.setSetting(key, value)),

  // export / import: no files on the web
  exportJson: (_path: string) => ok(0),
  importJson: (_path: string, _mode: "merge" | "replace") => ok<ImportResult>({ added: 0, updated: 0, skipped: 0 }),
};

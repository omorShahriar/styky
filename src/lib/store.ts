import { create } from "zustand";
import { api } from "./api";
import { formatDue } from "./time";
import type { Item } from "./types";

export interface Toast {
  id: number;
  message: string;
  action?: { label: string; run: () => void };
}

interface State {
  items: Item[];
  settings: Record<string, string>;
  now: number;
  loaded: boolean;

  /** keyboard cursor */
  focusedId: string | null;
  /** inline-expanded item, and whether it is in edit mode */
  expandedId: string | null;
  editing: boolean;
  /** a brand-new item being written (id once it has been saved for the first time) */
  draft: { id: string | null } | null;
  completedOpen: boolean;
  toast: Toast | null;
  /** the item whose snooze menu is open (custom = open on the custom picker) */
  snoozeFor: { id: string; custom: boolean } | null;
  /** ids whose check is drawing in before they actually complete */
  checking: string[];

  openSnooze: (id: string, custom?: boolean) => void;
  closeSnooze: () => void;
  snooze: (id: string, until: number) => Promise<void>;
  refresh: () => Promise<void>;
  tick: () => void;
  focus: (id: string | null) => void;
  expand: (id: string | null, editing?: boolean) => void;
  setEditing: (editing: boolean) => void;
  openDraft: () => void;
  closeDraft: () => void;
  setDraftId: (id: string) => void;
  toggleCompleted: () => void;
  showToast: (message: string, action?: Toast["action"], ms?: number) => void;
  dismissToast: () => void;

  /** Check animation, then (250 ms later) complete. */
  completeAnimated: (id: string) => void;
  complete: (id: string) => Promise<void>;
  restore: (id: string) => Promise<void>;
  remove: (item: Item) => Promise<void>;
}

let toastSeq = 0;
let toastTimer: ReturnType<typeof setTimeout> | undefined;

export const useStore = create<State>((set, get) => ({
  items: [],
  settings: {},
  now: Date.now(),
  loaded: false,
  focusedId: null,
  expandedId: null,
  editing: false,
  draft: null,
  completedOpen: false,
  toast: null,
  checking: [],
  snoozeFor: null,

  openSnooze: (id, custom = false) => set({ snoozeFor: { id, custom }, focusedId: id }),
  closeSnooze: () => set({ snoozeFor: null }),
  snooze: async (id, until) => {
    set({ snoozeFor: null });
    try {
      await api.snoozeItem(id, until);
      await get().refresh();
      get().showToast(`Snoozed until ${formatDue(until, Date.now())}`);
    } catch (e) {
      get().showToast(String(e));
    }
  },

  refresh: async () => {
    try {
      const [items, settings] = await Promise.all([api.listItems(), api.getSettings()]);
      const ids = new Set(items.map((i) => i.id));
      const { expandedId, focusedId } = get();
      set({
        items,
        settings,
        now: Date.now(),
        loaded: true,
        // drop references to rows that no longer exist (deleted / auto-cleared / imported over)
        expandedId: expandedId && ids.has(expandedId) ? expandedId : null,
        editing: expandedId && ids.has(expandedId) ? get().editing : false,
        focusedId: focusedId && ids.has(focusedId) ? focusedId : null,
      });
    } catch (e) {
      get().showToast(`Could not load reminders: ${String(e)}`);
    }
  },

  tick: () => {
    set({ now: Date.now() });
    void get().refresh();
  },

  focus: (id) => set({ focusedId: id }),

  expand: (id, editing = false) => set({ expandedId: id, editing: id ? editing : false }),

  setEditing: (editing) => set({ editing }),

  openDraft: () => set({ draft: { id: null }, expandedId: null, editing: false }),

  closeDraft: () => set({ draft: null }),

  setDraftId: (id) => set((s) => (s.draft ? { draft: { id } } : s)),

  toggleCompleted: () => set((s) => ({ completedOpen: !s.completedOpen })),

  showToast: (message, action, ms = 2600) => {
    clearTimeout(toastTimer);
    const id = ++toastSeq;
    set({ toast: { id, message, action } });
    toastTimer = setTimeout(() => {
      if (get().toast?.id === id) set({ toast: null });
    }, ms);
  },

  dismissToast: () => {
    clearTimeout(toastTimer);
    set({ toast: null });
  },

  completeAnimated: (id) => {
    if (get().checking.includes(id)) return;
    set((s) => ({ checking: [...s.checking, id] }));
    setTimeout(() => {
      void get()
        .complete(id)
        .finally(() => set((s) => ({ checking: s.checking.filter((c) => c !== id) })));
    }, 250);
  },

  complete: async (id) => {
    try {
      await api.completeItem(id);
      await get().refresh();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  restore: async (id) => {
    try {
      await api.restoreItem(id);
      await get().refresh();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  remove: async (item) => {
    try {
      await api.deleteItem(item.id);
      if (get().expandedId === item.id) set({ expandedId: null, editing: false });
      await get().refresh();
      get().showToast(
        `Deleted “${item.title}”`,
        {
          label: "Undo",
          run: () => {
            void api.undeleteItem(item).then(() => get().refresh());
            get().dismissToast();
          },
        },
        5000,
      );
    } catch (e) {
      get().showToast(String(e));
    }
  },
}));

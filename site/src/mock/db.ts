/**
 * In-memory stand-in for Styky's SQLite store and Rust commands. It mirrors the semantics of
 * src-tauri (sort.rs ordering, repeat roll-forward, snooze, undelete) closely enough that the
 * real React UI behaves exactly like the app.
 */
import type { Item, ItemEdit, Repeat } from "../../../src/lib/types";
import { emit } from "./events";

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

const clock = (offsetDays: number, h: number, m = 0) => {
  const d = new Date();
  d.setDate(d.getDate() + offsetDays);
  d.setHours(h, m, 0, 0);
  return d.getTime();
};

function seed(): Item[] {
  const now = Date.now();
  const base = (o: Partial<Item> & Pick<Item, "id" | "title">, n: number): Item => ({
    bodyMd: "",
    dueAt: null,
    repeat: "none",
    priority: 1,
    tag: null,
    link: null,
    branch: null,
    status: "open",
    snoozedUntil: null,
    lastFiredAt: null,
    completedAt: null,
    sortOrder: n,
    createdAt: now - (30 - n) * HOUR,
    updatedAt: now,
    repeatAnchorDay: null,
    ...o,
  });
  return [
    base(
      {
        id: "keys",
        title: "Rotate the production API keys",
        priority: 2,
        tag: "security",
        dueAt: now - 22 * MIN,
        lastFiredAt: now - 22 * MIN,
        bodyMd:
          "New keys go in the vault first, **then** CI.\n\n- [x] Generate new keys\n- [ ] Update CI secrets\n- [ ] Revoke the old ones",
      },
      1,
    ),
    base(
      {
        id: "release",
        title: "Release v2.3",
        priority: 2,
        tag: "api",
        branch: "release/2.3",
        link: "https://github.com/omorShahriar/styky/releases",
        dueAt: now + 2 * HOUR + 15 * MIN,
        bodyMd: "Cut the tag once CI is green.\n\n- [ ] Write the changelog\n- [ ] Tag and push\n- [ ] Announce in the channel",
      },
      2,
    ),
    base({ id: "reply", title: "Reply to Dana about the contract", tag: "work", dueAt: now + 5 * HOUR }, 3),
    base(
      {
        id: "ssl",
        title: "Renew the SSL certificate",
        tag: "ops",
        repeat: "monthly",
        dueAt: clock(1, 9),
        repeatAnchorDay: new Date(clock(1, 9)).getDate(),
        bodyMd: "Check the expiry in the dashboard first.",
      },
      4,
    ),
    base({ id: "backup", title: "Back up the photo library", tag: "home", repeat: "weekly", dueAt: clock(3, 10) }, 5),
    base({ id: "dentist", title: "Book the dentist", tag: "home", dueAt: now - 2 * DAY, snoozedUntil: clock(1, 14) }, 6),
    base({ id: "notes", title: "Read the Tauri 2.12 release notes", tag: "reading" }, 7),
    base(
      { id: "shortcuts", title: "Learn the keyboard shortcuts", bodyMd: "Press **Space** to complete, **S** to snooze, **E** to edit." },
      8,
    ),
    base(
      { id: "shipped", title: "Ship v0.1.0", tag: "styky", status: "done", completedAt: now - 3 * HOUR, createdAt: now - 3 * DAY },
      9,
    ),
  ];
}

let items: Item[] = seed();
const settings: Record<string, string> = {
  autoclear_days: "7",
  quickadd_hotkey: "Ctrl+Alt+N",
  theme: "system",
  default_snooze: "10m",
};

const uid = () => Math.random().toString(36).slice(2, 10) + Date.now().toString(36);
const changed = () => queueMicrotask(() => emit("items-changed"));
const find = (id: string) => {
  const i = items.find((x) => x.id === id);
  if (!i) throw new Error("Reminder not found");
  return i;
};
const effective = (i: Item) => i.snoozedUntil ?? i.dueAt;

/** The exact ordering of sort.rs: overdue, urgent, has-due, due time, manual order; done by completion. */
function sorted(now: number): Item[] {
  const key = (i: Item) => {
    const e = effective(i);
    return [e != null && e < now ? 0 : 1, i.priority === 2 ? 0 : 1, e == null ? 1 : 0, e ?? 0, i.sortOrder];
  };
  return [...items].sort((a, b) => {
    if ((a.status === "done") !== (b.status === "done")) return a.status === "done" ? 1 : -1;
    if (a.status === "done") return (b.completedAt ?? 0) - (a.completedAt ?? 0);
    const ka = key(a);
    const kb = key(b);
    for (let n = 0; n < ka.length; n++) if (ka[n] !== kb[n]) return ka[n] - kb[n];
    return a.createdAt - b.createdAt;
  });
}

function next(due: number, repeat: Repeat, now: number, anchor: number | null): number | null {
  if (repeat === "none") return null;
  const d = new Date(due);
  const step = () => {
    if (repeat === "daily") d.setDate(d.getDate() + 1);
    else if (repeat === "weekly") d.setDate(d.getDate() + 7);
    else if (repeat === "weekdays") {
      do d.setDate(d.getDate() + 1);
      while (d.getDay() === 0 || d.getDay() === 6);
    } else {
      const day = anchor ?? d.getDate();
      d.setDate(1);
      d.setMonth(d.getMonth() + 1);
      d.setDate(Math.min(day, new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate()));
    }
  };
  do step();
  while (d.getTime() <= now);
  return d.getTime();
}

export const db = {
  settings: () => ({ ...settings }),
  setSetting(key: string, value: string) {
    settings[key] = value;
    emit("settings-changed");
  },
  list: () => sorted(Date.now()).map((i) => ({ ...i })),
  create(edit: ItemEdit): Item {
    const now = Date.now();
    const item: Item = {
      ...edit,
      id: uid(),
      status: "open",
      snoozedUntil: null,
      lastFiredAt: null,
      completedAt: null,
      sortOrder: Math.max(0, ...items.map((i) => i.sortOrder)) + 1,
      createdAt: now,
      updatedAt: now,
      repeatAnchorDay: edit.repeat === "monthly" && edit.dueAt ? new Date(edit.dueAt).getDate() : null,
    };
    items.push(item);
    changed();
    return { ...item };
  },
  update(id: string, edit: ItemEdit): Item {
    const i = find(id);
    const dueChanged = edit.dueAt !== i.dueAt;
    Object.assign(i, edit, { updatedAt: Date.now() });
    if (dueChanged) {
      i.snoozedUntil = null;
      i.lastFiredAt = null;
    }
    if (edit.repeat === "monthly" && edit.dueAt) i.repeatAnchorDay = new Date(edit.dueAt).getDate();
    changed();
    return { ...i };
  },
  remove(id: string) {
    items = items.filter((i) => i.id !== id);
    changed();
  },
  undelete(item: Item) {
    if (!items.some((i) => i.id === item.id)) items.push({ ...item });
    changed();
  },
  complete(id: string): Item {
    const i = find(id);
    const now = Date.now();
    const at = i.dueAt != null ? next(i.dueAt, i.repeat, now, i.repeatAnchorDay) : null;
    if (at != null) {
      Object.assign(i, { dueAt: at, snoozedUntil: null, lastFiredAt: null, updatedAt: now });
    } else {
      Object.assign(i, { status: "done", completedAt: now, snoozedUntil: null, updatedAt: now });
    }
    changed();
    return { ...i };
  },
  restore(id: string): Item {
    const i = find(id);
    Object.assign(i, { status: "open", completedAt: null, updatedAt: Date.now() });
    changed();
    return { ...i };
  },
  snooze(id: string, until: number): Item {
    const i = find(id);
    Object.assign(i, { snoozedUntil: until, updatedAt: Date.now() });
    changed();
    return { ...i };
  },
  reorder(ids: string[]) {
    ids.forEach((id, n) => {
      const i = items.find((x) => x.id === id);
      if (i) i.sortOrder = n + 1;
    });
    changed();
  },
  reset() {
    items = seed();
    changed();
  },
};

export type Repeat = "none" | "daily" | "weekdays" | "weekly" | "monthly";
/** 0 low, 1 normal, 2 urgent */
export type Priority = 0 | 1 | 2;

export interface Item {
  id: string;
  title: string;
  bodyMd: string;
  dueAt: number | null;
  repeat: Repeat;
  priority: Priority;
  tag: string | null;
  link: string | null;
  branch: string | null;
  status: "open" | "done";
  snoozedUntil: number | null;
  lastFiredAt: number | null;
  completedAt: number | null;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

/** The fields the editor can change. */
export interface ItemEdit {
  title: string;
  bodyMd: string;
  dueAt: number | null;
  repeat: Repeat;
  priority: Priority;
  tag: string | null;
  link: string | null;
  branch: string | null;
}

export interface ImportResult {
  added: number;
  updated: number;
  skipped: number;
}

export const emptyEdit = (): ItemEdit => ({
  title: "",
  bodyMd: "",
  dueAt: null,
  repeat: "none",
  priority: 1,
  tag: null,
  link: null,
  branch: null,
});

export const toEdit = (i: Item): ItemEdit => ({
  title: i.title,
  bodyMd: i.bodyMd,
  dueAt: i.dueAt,
  repeat: i.repeat,
  priority: i.priority,
  tag: i.tag,
  link: i.link,
  branch: i.branch,
});

/** `none`, a free simple value (daily | weekdays | weekly | monthly) or, with Pro, an RRULE-like string (see rrule.ts). */
export type Repeat = string;
/** 0 low, 1 normal, 2 urgent, 3 critical (Pro: keeps re-notifying) */
export type Priority = 0 | 1 | 2 | 3;

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
  repeatAnchorDay: number | null;
  /** Pro: minutes before the due time for a heads-up toast (5, 15, 30, 60, 1440). */
  prealertMin: number | null;
  preFiredAt: number | null;
  lastNagAt: number | null;
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
  prealertMin: number | null;
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
  prealertMin: null,
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
  prealertMin: i.prealertMin,
});

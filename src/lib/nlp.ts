/**
 * Quick-add natural-language parser.
 *
 * Grammar (tokens can appear anywhere in the line):
 *   #tag               tag / project
 *   !!!                critical (priority 3, Pro: free builds fall back to urgent)
 *   !!                 urgent (priority 2)
 *   !                  explicit "normal" (priority 1, which is also the default; the chip confirms it was seen)
 *   !low               low (priority 0)
 *   every day|weekday|week|month   repeat (daily / weekdays / weekly / monthly)
 *   Pro repeats (see nlpRepeat.ts): every 2 weeks, every mon,wed, last friday of the month, ... for 5 times / until dec 31
 *   @branch            branch / repo chip (`@release/2.3`), must start a word so e-mail addresses are left alone
 *   https://…          link
 *   anything chrono-node understands ("tomorrow 5pm", "fri 10am", "in 2h") becomes the due time.
 *   A date without a time is due at 09:00. A repeat without a date starts at the next 09:00.
 * Whatever is left is the title.
 */
import * as chrono from "chrono-node";
import type { ItemEdit, Priority, Repeat } from "./types";
import { formatDue } from "./time";
import { describeRepeat, firstMatch, isSimpleRepeat } from "./rrule";
import { extractRepeat } from "./nlpRepeat";

export interface QuickChip {
  kind: "date" | "tag" | "priority" | "repeat" | "branch" | "link";
  label: string;
}

export interface QuickParse {
  title: string;
  dueAt: number | null;
  repeat: Repeat;
  priority: Priority;
  tag: string | null;
  link: string | null;
  branch: string | null;
  chips: QuickChip[];
}

const PRIORITY_LABEL: Record<Priority, string> = { 0: "Low", 1: "Normal", 2: "Urgent", 3: "Critical" };

const DEFAULT_HOUR = 9;

/** Cut the first match of `re` (group 1 = text to keep before it) out of `s`. */
function take(s: string, re: RegExp): { rest: string; match: RegExpExecArray | null } {
  const m = re.exec(s);
  if (!m) return { rest: s, match: null };
  return { rest: s.slice(0, m.index) + " " + s.slice(m.index + m[0].length), match: m };
}

/** Next 09:00 strictly after `now` (skipping Sat/Sun when `weekdays`). */
function nextDefaultTime(now: Date, weekdays: boolean): number {
  const d = new Date(now);
  d.setHours(DEFAULT_HOUR, 0, 0, 0);
  if (d.getTime() <= now.getTime()) d.setDate(d.getDate() + 1);
  while (weekdays && (d.getDay() === 0 || d.getDay() === 6)) d.setDate(d.getDate() + 1);
  return d.getTime();
}

export function parseQuick(input: string, now: Date = new Date()): QuickParse {
  let text = ` ${input} `;
  let link: string | null = null;
  let branch: string | null = null;
  let tag: string | null = null;
  let repeat: Repeat = "none";
  let priority: Priority = 1;
  let prioritySeen = false;

  let t = take(text, /https?:\/\/[^\s]+/i);
  if (t.match) link = t.match[0].replace(/[.,;)]+$/, "");
  text = t.rest;

  const rep = extractRepeat(text, now);
  if (rep.repeat) repeat = rep.repeat;
  text = rep.rest;

  t = take(text, /(?<=\s)@([\w][\w./-]*)/);
  if (t.match) branch = t.match[1].replace(/[.,;]+$/, "");
  text = t.rest;

  t = take(text, /(?<=\s)#([\w][\w-]*)/);
  if (t.match) tag = t.match[1];
  text = t.rest;

  t = take(text, /(?<=\s)!low(?=\s)/i);
  if (t.match) {
    priority = 0;
    prioritySeen = true;
  } else {
    t = take(text, /(?<=\s)(!{1,3})(?=\s)/);
    if (t.match) {
      priority = t.match[1].length === 3 ? 3 : t.match[1].length === 2 ? 2 : 1;
      prioritySeen = true;
    }
  }
  text = t.rest;

  // Dates. Hours that were not spoken default to 09:00.
  let dueAt: number | null = null;
  let title = text;
  const found = chrono.parse(text, now, { forwardDate: true })[0];
  if (found) {
    const d = found.start.date();
    if (!found.start.isCertain("hour")) d.setHours(DEFAULT_HOUR, 0, 0, 0);
    dueAt = d.getTime();
    const without = text.slice(0, found.index) + " " + text.slice(found.index + found.text.length);
    // "tomorrow 5pm" alone: keep the words as the title rather than saving an empty one.
    if (clean(without)) title = without;
  }
  title = clean(title);
  if (dueAt == null && repeat !== "none") dueAt = firstMatch(repeat, now, DEFAULT_HOUR) ?? nextDefaultTime(now, repeat === "weekdays");

  const chips: QuickChip[] = [];
  if (dueAt != null) chips.push({ kind: "date", label: formatDue(dueAt, now.getTime()) });
  if (tag) chips.push({ kind: "tag", label: `#${tag}` });
  if (prioritySeen) chips.push({ kind: "priority", label: PRIORITY_LABEL[priority] });
  if (repeat !== "none") chips.push({ kind: "repeat", label: describeRepeat(repeat) + (isSimpleRepeat(repeat) ? "" : " · Pro") });
  if (branch) chips.push({ kind: "branch", label: branch });
  if (link) chips.push({ kind: "link", label: hostOf(link) });

  return { title, dueAt, repeat, priority, tag, link, branch, chips };
}

function hostOf(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

/** Collapse whitespace and drop dangling connectives left behind by a removed date ("… by", "… at"). */
function clean(s: string): string {
  return s
    .replace(/\s+/g, " ")
    .trim()
    .replace(/(^|\s)(at|on|by|due|in|from|until|for)$/i, "")
    .trim();
}

export function toItemEdit(p: QuickParse): ItemEdit {
  return {
    title: p.title,
    bodyMd: "",
    dueAt: p.dueAt,
    repeat: p.repeat,
    priority: p.priority,
    tag: p.tag,
    link: p.link,
    branch: p.branch,
    prealertMin: null,
  };
}

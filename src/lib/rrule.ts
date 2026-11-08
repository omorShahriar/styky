/**
 * Repeat rules. `item.repeat` is a legacy simple value (`none|daily|weekdays|weekly|monthly`, free) or, with
 * Styky Pro, an RRULE-like string understood by `src-tauri/src/repeat.rs` (the engine that actually advances items):
 *
 *   FREQ=DAILY|WEEKLY|MONTHLY ; INTERVAL=n ; BYDAY=MO,WE | 2FR | -1FR ; BYMONTHDAY=n|-1 ; COUNT=n ; UNTIL=YYYYMMDD
 *
 * This file only parses, formats and describes rules for the UI and the quick-add parser; it never computes dates
 * except `firstMatch` (the first due date of a rule that names its own days).
 */

export type Freq = "DAILY" | "WEEKLY" | "MONTHLY";

export interface DaySpec {
  /** null for weekly rules; 1..5 or -1 (last) for the monthly "nth weekday". */
  ord: number | null;
  /** 0 = Monday ... 6 = Sunday */
  day: number;
}

export interface Rule {
  freq: Freq;
  interval: number;
  byday: DaySpec[];
  bymonthday: number | null;
  count: number | null;
  /** YYYYMMDD */
  until: string | null;
}

export const DAY_CODES = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"] as const;
export const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"] as const;
const SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;

export const SIMPLE_REPEATS = ["none", "daily", "weekdays", "weekly", "monthly"] as const;

export function isSimpleRepeat(s: string): boolean {
  return (SIMPLE_REPEATS as readonly string[]).includes(s) || s === "";
}

const blank = (freq: Freq): Rule => ({ freq, interval: 1, byday: [], bymonthday: null, count: null, until: null });

export function newRule(freq: Freq = "WEEKLY"): Rule {
  return blank(freq);
}

/** Parse a legacy value or RRULE-like string; null for `none`, empty or anything invalid. */
export function parseRule(input: string): Rule | null {
  const s = input.trim();
  switch (s) {
    case "daily":
      return blank("DAILY");
    case "weekly":
      return blank("WEEKLY");
    case "monthly":
      return blank("MONTHLY");
    case "weekdays":
      return { ...blank("WEEKLY"), byday: [0, 1, 2, 3, 4].map((day) => ({ ord: null, day })) };
    case "":
    case "none":
      return null;
  }
  const r = blank("DAILY");
  let haveFreq = false;
  for (const part of s.split(";")) {
    const eq = part.indexOf("=");
    if (eq < 0) return null;
    const k = part.slice(0, eq).trim().toUpperCase();
    const v = part.slice(eq + 1).trim();
    switch (k) {
      case "FREQ": {
        const f = v.toUpperCase();
        if (f !== "DAILY" && f !== "WEEKLY" && f !== "MONTHLY") return null;
        r.freq = f;
        haveFreq = true;
        break;
      }
      case "INTERVAL": {
        const n = Number(v);
        if (!Number.isInteger(n) || n < 1 || n > 366) return null;
        r.interval = n;
        break;
      }
      case "COUNT": {
        const n = Number(v);
        if (!Number.isInteger(n) || n < 1 || n > 9999) return null;
        r.count = n;
        break;
      }
      case "UNTIL":
        if (!/^\d{8}$/.test(v)) return null;
        r.until = v;
        break;
      case "BYMONTHDAY": {
        const n = Number(v);
        if (!Number.isInteger(n) || !(n === -1 || (n >= 1 && n <= 31))) return null;
        r.bymonthday = n;
        break;
      }
      case "BYDAY":
        for (const raw of v.split(",")) {
          const t = raw.trim().toUpperCase();
          const m = /^([+-]?\d)?(MO|TU|WE|TH|FR|SA|SU)$/.exec(t);
          if (!m) return null;
          const ord = m[1] ? Number(m[1]) : null;
          if (ord !== null && !(ord === -1 || (ord >= 1 && ord <= 5))) return null;
          const day = DAY_CODES.indexOf(m[2] as (typeof DAY_CODES)[number]);
          if (!r.byday.some((d) => d.ord === ord && d.day === day)) r.byday.push({ ord, day });
        }
        break;
      default:
        return null;
    }
  }
  if (!haveFreq) return null;
  if (r.freq === "DAILY" && (r.byday.length || r.bymonthday !== null)) return null;
  if (r.freq === "WEEKLY") {
    if (r.byday.some((d) => d.ord !== null) || r.bymonthday !== null) return null;
    r.byday.sort((a, b) => a.day - b.day);
  }
  if (r.freq === "MONTHLY") {
    if (r.byday.length > 1 || (r.byday.length && r.bymonthday !== null) || r.byday.some((d) => d.ord === null)) return null;
  }
  return r;
}

/** Canonical text, identical to the Rust side (FREQ, INTERVAL, BYDAY, BYMONTHDAY, COUNT, UNTIL). */
export function formatRule(r: Rule): string {
  const p = [`FREQ=${r.freq}`];
  if (r.interval > 1) p.push(`INTERVAL=${r.interval}`);
  if (r.byday.length) p.push(`BYDAY=${r.byday.map((d) => `${d.ord ?? ""}${DAY_CODES[d.day]}`).join(",")}`);
  if (r.bymonthday !== null) p.push(`BYMONTHDAY=${r.bymonthday}`);
  if (r.count !== null) p.push(`COUNT=${r.count}`);
  if (r.until) p.push(`UNTIL=${r.until}`);
  return p.join(";");
}

/** What to store for a rule: the free simple value when it is exactly one, else the RRULE text. */
export function storeRule(r: Rule): string {
  if (r.interval === 1 && r.count === null && r.until === null && r.bymonthday === null) {
    if (r.freq === "DAILY") return "daily";
    if (r.freq === "WEEKLY" && r.byday.length === 0) return "weekly";
    if (r.freq === "MONTHLY" && r.byday.length === 0) return "monthly";
    if (r.freq === "WEEKLY" && r.byday.length === 5 && r.byday.every((d) => d.day < 5)) return "weekdays";
  }
  return formatRule(r);
}

const ORD_WORD: Record<number, string> = { 1: "first", 2: "second", 3: "third", 4: "fourth", 5: "fifth", [-1]: "last" };
const ORD_NUM = (n: number): string => {
  if (n === -1) return "last";
  const t = n % 100;
  const suffix = t >= 11 && t <= 13 ? "th" : ({ 1: "st", 2: "nd", 3: "rd" } as Record<number, string>)[n % 10] ?? "th";
  return `${n}${suffix}`;
};

function days(r: Rule): string {
  return r.byday.map((d) => SHORT[d.day]).join(", ");
}

function untilText(u: string): string {
  const d = new Date(Number(u.slice(0, 4)), Number(u.slice(4, 6)) - 1, Number(u.slice(6, 8)));
  return d.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/** "Every 2 weeks on Mon, Wed · 3 left", "Last Friday of the month", "Every weekday". */
export function describeRepeat(s: string): string {
  const r = parseRule(s);
  if (!r) return "";
  const n = r.interval;
  let text: string;
  if (r.freq === "DAILY") text = n === 1 ? "Every day" : `Every ${n} days`;
  else if (r.freq === "WEEKLY") {
    const weekdays = n === 1 && r.byday.length === 5 && r.byday.every((d) => d.day < 5);
    if (weekdays) text = "Every weekday";
    else {
      const base = n === 1 ? "Every week" : `Every ${n} weeks`;
      text = r.byday.length ? `${base} on ${days(r)}` : base;
    }
  } else {
    const every = n === 1 ? "month" : `${n} months`;
    if (r.byday.length) {
      const { ord, day } = r.byday[0];
      text =
        n === 1
          ? `${ORD_WORD[ord ?? 1][0].toUpperCase()}${ORD_WORD[ord ?? 1].slice(1)} ${DAY_NAMES[day]} of the month`
          : `Every ${every} on the ${ORD_WORD[ord ?? 1]} ${DAY_NAMES[day]}`;
    } else if (r.bymonthday !== null) {
      text = r.bymonthday === -1 ? (n === 1 ? "Last day of the month" : `Every ${every} on the last day`) : `Every ${every} on the ${ORD_NUM(r.bymonthday)}`;
    } else text = n === 1 ? "Every month" : `Every ${every}`;
  }
  if (r.count !== null) text += ` · ${r.count} ${r.count === 1 ? "time" : "times"} left`;
  if (r.until) text += ` · until ${untilText(r.until)}`;
  return text;
}

const pad = (n: number) => String(n).padStart(2, "0");
export const toUntil = (d: Date): string => `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;
export const fromUntil = (u: string): string => `${u.slice(0, 4)}-${u.slice(4, 6)}-${u.slice(6, 8)}`;

function nthWeekday(y: number, m: number, ord: number, day: number): Date | null {
  // day: 0 = Monday. JS getDay(): 0 = Sunday.
  const js = (day + 1) % 7;
  if (ord === -1) {
    const last = new Date(y, m + 1, 0);
    return new Date(y, m, last.getDate() - ((last.getDay() - js + 7) % 7));
  }
  const first = new Date(y, m, 1);
  const d = 1 + ((js - first.getDay() + 7) % 7) + 7 * (ord - 1);
  const out = new Date(y, m, d);
  return out.getMonth() === m ? out : null;
}

/**
 * First due time for a rule that names its own days (every mon,wed / last friday of the month / the 15th) when
 * the user gave no date: the first matching local date from today at `hour`:00 that is still ahead. null when
 * the rule names no days (the caller then uses its default start).
 */
export function firstMatch(repeat: string, now: Date, hour = 9): number | null {
  const r = parseRule(repeat);
  if (!r || (r.byday.length === 0 && r.bymonthday === null)) return null;
  for (let k = 0; k < 800; k++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + k, hour, 0, 0, 0);
    let hit = false;
    if (r.freq === "WEEKLY") hit = r.byday.some((b) => (b.day + 1) % 7 === d.getDay());
    else if (r.freq === "MONTHLY") {
      if (r.byday.length) {
        const n = nthWeekday(d.getFullYear(), d.getMonth(), r.byday[0].ord ?? 1, r.byday[0].day);
        hit = !!n && n.getDate() === d.getDate();
      } else if (r.bymonthday !== null) {
        const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
        hit = (r.bymonthday === -1 ? last : Math.min(r.bymonthday, last)) === d.getDate();
      }
    }
    if (hit && d.getTime() > now.getTime()) return d.getTime();
  }
  return null;
}

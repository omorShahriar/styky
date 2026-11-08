/**
 * Repeat phrases of the quick-add grammar (twin of `src-tauri/src/nlp_repeat.rs`; keep them in step).
 *
 * Free (legacy values): every day | weekday(s) | week | month
 * Pro (RRULE strings):
 *   every 2 days|weeks|months       every other week
 *   every mon,wed / every monday and friday / every mon-fri
 *   every 2 weeks on mon,thu
 *   last friday of the month        first monday of every month    every month on the 2nd tuesday
 *   last day of the month           every month on the 15th
 * optionally followed by an end: `for 5 times` / `5 times`, or `until 2026-12-31` / `until dec 31`.
 */
import * as chrono from "chrono-node";
import { DAY_CODES, type DaySpec, type Rule, newRule, storeRule, toUntil } from "./rrule";

const WD = "mon(?:day)?|tue(?:s(?:day)?)?|wed(?:nesday)?|thu(?:r(?:s(?:day)?)?)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?";
const ORD = "first|second|third|fourth|fifth|last|1st|2nd|3rd|4th|5th";
const LIST = `(?:${WD})(?:\\s*(?:,|and|&|-|\\s)\\s*(?:${WD}))*`;

const NTH_OF_MONTH = new RegExp(`\\b(?:every\\s+)?(${ORD})\\s+(${WD})\\s+(?:of|in)\\s+(?:the|every|each)\\s+month\\b`, "i");
const EVERY_MONTH_ON_NTH = new RegExp(`\\bevery\\s+month\\s+on\\s+the\\s+(${ORD})\\s+(${WD})\\b`, "i");
const LAST_DAY = /\blast\s+day\s+of\s+(?:the|every|each)\s+month\b/i;
const MONTH_ON_DAY = /\bevery\s+month\s+on\s+the\s+(\d{1,2})(?:st|nd|rd|th)\b/i;
const INTERVAL = new RegExp(`\\bevery\\s+(\\d+|other)\\s+(days?|weeks?|months?)(?:\\s+on\\s+(${LIST}))?\\b`, "i");
const DAY_LIST = new RegExp(`\\bevery\\s+(${LIST})\\b`, "i");
const SIMPLE = /\bevery\s+(day|weekdays?|week|month)\b/i;
const COUNT = /\b(?:for\s+)?(\d{1,4})\s+(?:times|occurrences)\b/i;
const UNTIL = /\b(?:until|through|till)\s+/i;

const ORDINALS: Record<string, number> = { first: 1, "1st": 1, second: 2, "2nd": 2, third: 3, "3rd": 3, fourth: 4, "4th": 4, fifth: 5, "5th": 5, last: -1 };

/** Monday = 0 ... Sunday = 6 from a name or abbreviation. */
export function weekdayIndex(name: string): number {
  return ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].indexOf(name.slice(0, 3).toLowerCase());
}

function parseDays(list: string): number[] {
  const toks = [...list.matchAll(new RegExp(`\\b(${WD})\\b`, "gi"))].map((m) => weekdayIndex(m[1]));
  let out: number[];
  if (list.includes("-") && toks.length === 2) {
    out = [];
    for (let d = toks[0]; ; d = (d + 1) % 7) {
      out.push(d);
      if (d === toks[1]) break;
    }
  } else out = toks;
  return [...new Set(out)].sort((a, b) => a - b);
}

const cut = (s: string, re: RegExp): { rest: string; g: string[] } | null => {
  const m = re.exec(s);
  if (!m) return null;
  return { rest: `${s.slice(0, m.index)} ${s.slice(m.index + m[0].length)}`, g: m.map((x) => x ?? "") };
};

const weekly = (days: number[], interval = 1): Rule => ({
  ...newRule("WEEKLY"),
  interval,
  byday: days.map((day): DaySpec => ({ ord: null, day })),
});

type Phrase = { rest: string; simple: string } | { rest: string; rule: Rule };

function phrase(s: string): Phrase | null {
  let c = cut(s, NTH_OF_MONTH) ?? cut(s, EVERY_MONTH_ON_NTH);
  if (c) {
    const r = newRule("MONTHLY");
    r.byday = [{ ord: ORDINALS[c.g[1].toLowerCase()], day: weekdayIndex(c.g[2]) }];
    return { rest: c.rest, rule: r };
  }
  c = cut(s, LAST_DAY);
  if (c) return { rest: c.rest, rule: { ...newRule("MONTHLY"), bymonthday: -1 } };
  c = cut(s, MONTH_ON_DAY);
  if (c) {
    const d = Number(c.g[1]);
    if (d < 1 || d > 31) return null;
    return { rest: c.rest, rule: { ...newRule("MONTHLY"), bymonthday: d } };
  }
  c = cut(s, INTERVAL);
  if (c) {
    const n = c.g[1].toLowerCase() === "other" ? 2 : Number(c.g[1]);
    if (n < 1 || n > 366) return null;
    const u = c.g[2].toLowerCase();
    const freq = u.startsWith("day") ? "DAILY" : u.startsWith("week") ? "WEEKLY" : "MONTHLY";
    const days = c.g[3] ? parseDays(c.g[3]) : [];
    if (days.length && freq !== "WEEKLY") return null;
    if (n === 1 && !days.length) return { rest: c.rest, simple: freq === "DAILY" ? "daily" : freq === "WEEKLY" ? "weekly" : "monthly" };
    return { rest: c.rest, rule: freq === "WEEKLY" ? weekly(days, n) : { ...newRule(freq), interval: n } };
  }
  c = cut(s, DAY_LIST);
  if (c) {
    const days = parseDays(c.g[1]);
    if (days.length) return { rest: c.rest, rule: weekly(days) };
  }
  c = cut(s, SIMPLE);
  if (c) {
    const w = c.g[1].toLowerCase();
    return { rest: c.rest, simple: w === "day" ? "daily" : w === "week" ? "weekly" : w === "month" ? "monthly" : "weekdays" };
  }
  return null;
}

type End = { count: number } | { until: string };

function endClause(s: string, now: Date): { rest: string; end: End | null } {
  const c = cut(s, COUNT);
  if (c && Number(c.g[1]) >= 1) return { rest: c.rest, end: { count: Number(c.g[1]) } };
  const m = UNTIL.exec(s);
  if (m) {
    const tail = s.slice(m.index + m[0].length);
    const found = chrono.parse(tail, now, { forwardDate: true })[0];
    if (found && tail.slice(0, found.index).trim() === "") {
      const d = found.start.date();
      return { rest: `${s.slice(0, m.index)} ${tail.slice(found.index + found.text.length)}`, end: { until: toUntil(d) } };
    }
  }
  return { rest: s, end: null };
}

/** Find a repeat phrase (and end clause). `repeat` is a legacy name for free phrases, an RRULE string for Pro ones. */
export function extractRepeat(s: string, now: Date): { rest: string; repeat: string | null } {
  const p = phrase(s);
  if (!p) return { rest: s, repeat: null };
  const { rest, end } = endClause(p.rest, now);
  const rule = "rule" in p ? p.rule : legacyRule(p.simple);
  if ("simple" in p && !end) return { rest, repeat: p.simple };
  if (end) {
    if ("count" in end) rule.count = end.count;
    else rule.until = end.until;
  }
  return { rest, repeat: storeRule(rule) };
}

function legacyRule(name: string): Rule {
  if (name === "daily") return newRule("DAILY");
  if (name === "weekly") return newRule("WEEKLY");
  if (name === "monthly") return newRule("MONTHLY");
  return weekly([0, 1, 2, 3, 4]);
}

export { DAY_CODES };

const MIN = 60_000;
const HOUR = 60 * MIN;

const timeFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });
const weekdayFmt = new Intl.DateTimeFormat(undefined, { weekday: "short" });
const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });
const dateYearFmt = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

export type DueState = "none" | "overdue" | "soon" | "later";

export function dueState(at: number | null, now: number): DueState {
  if (at == null) return "none";
  if (at < now) return "overdue";
  if (at - now <= HOUR) return "soon";
  return "later";
}

function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function endOfToday(now: number): number {
  const d = new Date(now);
  d.setHours(23, 59, 59, 999);
  return d.getTime();
}

/** Short relative description of a due time, e.g. "in 25 min", "Tomorrow 9:00 AM". */
export function formatDue(at: number, now: number): string {
  const diff = at - now;
  const abs = Math.abs(diff);
  if (abs < MIN) return "now";
  if (abs < HOUR) {
    const m = Math.round(abs / MIN);
    return diff > 0 ? `in ${m} min` : `${m} min ago`;
  }
  const days = Math.round((startOfDay(at) - startOfDay(now)) / 86_400_000);
  const time = timeFmt.format(at);
  if (days === 0) {
    if (abs < 6 * HOUR) {
      const h = Math.round(abs / HOUR);
      return diff > 0 ? `in ${h}h` : `${h}h ago`;
    }
    return `Today ${time}`;
  }
  if (days === 1) return `Tomorrow ${time}`;
  if (days === -1) return `Yesterday ${time}`;
  if (days > 1 && days < 7) return `${weekdayFmt.format(at)} ${time}`;
  const sameYear = new Date(at).getFullYear() === new Date(now).getFullYear();
  return `${(sameYear ? dateFmt : dateYearFmt).format(at)}, ${time}`;
}

const pad = (n: number) => String(n).padStart(2, "0");

/** ms -> value for <input type="datetime-local"> (local time). */
export function toLocalInput(ms: number | null): string {
  if (ms == null) return "";
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Value of <input type="datetime-local"> -> ms, or null if empty/invalid. */
export function fromLocalInput(v: string): number | null {
  if (!v) return null;
  const t = new Date(v).getTime();
  return Number.isNaN(t) ? null : t;
}

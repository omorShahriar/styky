import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { PRO_URL, usePro } from "../lib/pro";
import { DAY_CODES, DAY_NAMES, type Freq, type Rule, describeRepeat, fromUntil, isSimpleRepeat, newRule, parseRule, storeRule, toUntil } from "../lib/rrule";
import { ProPill } from "./ProPill";

const SIMPLE_OPTIONS = [
  { value: "none", label: "Never" },
  { value: "daily", label: "Daily" },
  { value: "weekdays", label: "Weekdays" },
  { value: "weekly", label: "Weekly" },
  { value: "monthly", label: "Monthly" },
];

const ORDINALS = [
  { v: 1, label: "first" },
  { v: 2, label: "second" },
  { v: 3, label: "third" },
  { v: 4, label: "fourth" },
  { v: 5, label: "fifth" },
  { v: -1, label: "last" },
];

type MonthMode = "day" | "nth" | "last";

function monthMode(r: Rule): MonthMode {
  if (r.byday.length) return "nth";
  return r.bymonthday === -1 ? "last" : "day";
}

interface Props {
  value: string;
  /** Needed to anchor "day of month" / default weekday for a new custom rule. */
  dueAt: number | null;
  onChange: (repeat: string) => void;
}

/** Repeat select (free values) plus the Pro "Custom…" builder: every N, weekdays, nth weekday, end after N / on a date. */
export function RepeatField({ value, dueAt, onChange }: Props) {
  const pro = usePro();
  const custom = !isSimpleRepeat(value);
  const [open, setOpen] = useState(custom);
  const rule = parseRule(value);
  const noDue = dueAt == null;

  const startRule = (): Rule => {
    const d = dueAt != null ? new Date(dueAt) : new Date();
    const r = newRule("WEEKLY");
    r.interval = 2;
    r.byday = [{ ord: null, day: (d.getDay() + 6) % 7 }];
    return r;
  };

  const set = (patch: Partial<Rule>) => {
    const base = rule ?? startRule();
    onChange(storeRule({ ...base, ...patch }));
  };

  const onSelect = (v: string) => {
    if (v === "custom") {
      if (!pro) {
        void openUrl(PRO_URL).catch(() => {});
        return;
      }
      setOpen(true);
      if (isSimpleRepeat(value)) onChange(storeRule(rule ?? startRule()));
      return;
    }
    setOpen(false);
    onChange(v);
  };

  const r = rule ?? startRule();
  const locked = !pro;
  const mode = monthMode(r);
  const selectValue = custom || open ? "custom" : value;
  const toggleDay = (day: number) => {
    const has = r.byday.some((b) => b.day === day);
    let byday = has ? r.byday.filter((b) => b.day !== day) : [...r.byday, { ord: null, day }];
    if (byday.length === 0) byday = r.byday; // keep at least one day
    set({ byday: [...byday].sort((a, b) => a.day - b.day) });
  };
  const setFreq = (freq: Freq) => {
    const dow = r.byday[0]?.day ?? (new Date(dueAt ?? Date.now()).getDay() + 6) % 7;
    set({
      freq,
      byday: freq === "WEEKLY" ? [{ ord: null, day: dow }] : [],
      bymonthday: null,
    });
  };
  const setMonthMode = (m: MonthMode) => {
    const dow = r.byday[0]?.day ?? (new Date(dueAt ?? Date.now()).getDay() + 6) % 7;
    if (m === "nth") set({ byday: [{ ord: r.byday[0]?.ord ?? 1, day: dow }], bymonthday: null });
    else if (m === "last") set({ byday: [], bymonthday: -1 });
    else set({ byday: [], bymonthday: null });
  };
  const endKind = r.count !== null ? "count" : r.until ? "until" : "never";

  return (
    <div className="repeat-field">
      <div className="ed-field">
        <span className="ed-label">
          Repeat {!pro && <ProPill title="Custom repeats are part of Styky Pro" />}
        </span>
        <select
          className="ed-input"
          value={selectValue}
          disabled={noDue}
          title={noDue ? "Set a due time to repeat" : undefined}
          onChange={(e) => onSelect(e.target.value)}
          aria-label="Repeat"
        >
          {SIMPLE_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
          <option value="custom">{custom ? `Custom: ${describeRepeat(value)}` : "Custom…"}</option>
        </select>
      </div>

      {(open || custom) && !noDue && (
        <div className={`repeat-builder${locked ? " locked" : ""}`} aria-label="Custom repeat">
          <div className="rb-row">
            <span>Every</span>
            <input
              className="ed-input rb-num"
              type="number"
              min={1}
              max={99}
              value={r.interval}
              disabled={locked}
              aria-label="Repeat interval"
              onChange={(e) => set({ interval: Math.min(99, Math.max(1, Number(e.target.value) || 1)) })}
            />
            <select className="ed-input" value={r.freq} disabled={locked} aria-label="Repeat unit" onChange={(e) => setFreq(e.target.value as Freq)}>
              <option value="DAILY">{r.interval === 1 ? "day" : "days"}</option>
              <option value="WEEKLY">{r.interval === 1 ? "week" : "weeks"}</option>
              <option value="MONTHLY">{r.interval === 1 ? "month" : "months"}</option>
            </select>
          </div>

          {r.freq === "WEEKLY" && (
            <div className="rb-days" role="group" aria-label="Weekdays">
              {DAY_CODES.map((c, i) => {
                const on = r.byday.some((b) => b.day === i);
                return (
                  <button
                    key={c}
                    type="button"
                    className={`rb-day${on ? " on" : ""}`}
                    aria-pressed={on}
                    aria-label={DAY_NAMES[i]}
                    disabled={locked}
                    onClick={() => toggleDay(i)}
                  >
                    {DAY_NAMES[i][0]}
                  </button>
                );
              })}
            </div>
          )}

          {r.freq === "MONTHLY" && (
            <div className="rb-row">
              <select className="ed-input" value={mode} disabled={locked} aria-label="Month repeat style" onChange={(e) => setMonthMode(e.target.value as MonthMode)}>
                <option value="day">Same day of the month</option>
                <option value="nth">On the nth weekday</option>
                <option value="last">On the last day</option>
              </select>
              {mode === "nth" && (
                <>
                  <select
                    className="ed-input"
                    value={r.byday[0]?.ord ?? 1}
                    disabled={locked}
                    aria-label="Which"
                    onChange={(e) => set({ byday: [{ ord: Number(e.target.value), day: r.byday[0]?.day ?? 0 }] })}
                  >
                    {ORDINALS.map((o) => (
                      <option key={o.v} value={o.v}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  <select
                    className="ed-input"
                    value={r.byday[0]?.day ?? 0}
                    disabled={locked}
                    aria-label="Weekday"
                    onChange={(e) => set({ byday: [{ ord: r.byday[0]?.ord ?? 1, day: Number(e.target.value) }] })}
                  >
                    {DAY_NAMES.map((n, i) => (
                      <option key={n} value={i}>
                        {n}
                      </option>
                    ))}
                  </select>
                </>
              )}
            </div>
          )}

          <div className="rb-row">
            <span>Ends</span>
            <select
              className="ed-input"
              value={endKind}
              disabled={locked}
              aria-label="Repeat end"
              onChange={(e) => {
                const k = e.target.value;
                if (k === "never") set({ count: null, until: null });
                else if (k === "count") set({ count: 5, until: null });
                else set({ count: null, until: toUntil(new Date((dueAt ?? Date.now()) + 90 * 86_400_000)) });
              }}
            >
              <option value="never">Never</option>
              <option value="count">After a number of times</option>
              <option value="until">On a date</option>
            </select>
            {endKind === "count" && (
              <input
                className="ed-input rb-num"
                type="number"
                min={1}
                max={999}
                value={r.count ?? 1}
                disabled={locked}
                aria-label="Occurrences"
                onChange={(e) => set({ count: Math.min(999, Math.max(1, Number(e.target.value) || 1)) })}
              />
            )}
            {endKind === "until" && r.until && (
              <input
                className="ed-input"
                type="date"
                value={fromUntil(r.until)}
                disabled={locked}
                aria-label="Repeat until"
                onChange={(e) => e.target.value && set({ until: e.target.value.replace(/-/g, "") })}
              />
            )}
          </div>
          <p className="rb-summary">{describeRepeat(value)}</p>
        </div>
      )}
    </div>
  );
}

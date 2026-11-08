import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { AlarmClock, CalendarClock } from "lucide-react";
import { useStore } from "../lib/store";
import { fromLocalInput, snoozeHint, snoozePresets, toLocalInput } from "../lib/time";

const WIDTH = 276;

/** Snooze popover for the item in `snoozeFor`; one instance lives in the Panel. */
export function SnoozeMenu() {
  const snoozeFor = useStore((s) => s.snoozeFor);
  return snoozeFor ? <Popover key={snoozeFor.id} id={snoozeFor.id} startCustom={snoozeFor.custom} /> : null;
}

function Popover({ id, startCustom }: { id: string; startCustom: boolean }) {
  const { snooze, closeSnooze, showToast } = useStore.getState();
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const [custom, setCustom] = useState(startCustom);
  const [now] = useState(() => Date.now());
  const [value, setValue] = useState(() => {
    const d = new Date(now + 3_600_000);
    d.setMinutes(Math.ceil(d.getMinutes() / 5) * 5, 0, 0);
    return toLocalInput(d.getTime());
  });
  const presets = snoozePresets(now);

  // Anchor to the row (fixed, so the list's overflow never clips it).
  useLayoutEffect(() => {
    const row = document.querySelector(`[data-id="${id}"]`);
    const menu = ref.current;
    if (!row || !menu) return;
    const r = row.getBoundingClientRect();
    const h = menu.offsetHeight;
    const left = Math.max(8, Math.min(window.innerWidth - WIDTH - 8, r.right - WIDTH - 8));
    let top = r.top + 36;
    if (top + h > window.innerHeight - 8) top = Math.max(8, Math.min(r.top - h - 4, window.innerHeight - h - 8));
    setPos({ left, top });
  }, [id, custom]);

  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("button, input")?.focus();
    const down = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) closeSnooze();
    };
    window.addEventListener("mousedown", down);
    window.addEventListener("blur", closeSnooze);
    return () => {
      window.removeEventListener("mousedown", down);
      window.removeEventListener("blur", closeSnooze);
    };
  }, [closeSnooze, custom]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      closeSnooze();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const els = [...(ref.current?.querySelectorAll<HTMLElement>("button") ?? [])];
      const i = els.indexOf(document.activeElement as HTMLElement);
      if (els.length === 0) return;
      e.preventDefault();
      els[(i + (e.key === "ArrowDown" ? 1 : -1) + els.length) % els.length].focus();
    }
    e.stopPropagation();
  };

  const submitCustom = () => {
    const until = fromLocalInput(value);
    if (until == null || until <= Date.now()) {
      showToast("Pick a time in the future");
      return;
    }
    void snooze(id, until);
  };

  return (
    <div
      ref={ref}
      className="menu snooze-menu"
      role="menu"
      aria-label="Snooze"
      style={pos ? { left: pos.left, top: pos.top } : { left: 0, top: 0, visibility: "hidden" }}
      onKeyDown={onKeyDown}
    >
      {custom ? (
        <div className="snooze-custom">
          <label className="snooze-label" htmlFor="snooze-custom-input">
            Snooze until
          </label>
          <input
            id="snooze-custom-input"
            type="datetime-local"
            className="ed-input tnum"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && submitCustom()}
          />
          <div className="snooze-custom-actions">
            <button type="button" className="btn" onClick={() => setCustom(false)}>
              Back
            </button>
            <button type="button" className="btn" onClick={submitCustom}>
              Snooze
            </button>
          </div>
        </div>
      ) : (
        <>
          {presets.map((p) => (
            <button key={p.id} type="button" className="menu-item" role="menuitem" onClick={() => void snooze(id, p.until)}>
              <AlarmClock size={14} strokeWidth={1.75} />
              <span className="grow">{p.label}</span>
              <span className="menu-hint tnum">{p.id === "10m" || p.id === "1h" ? "" : snoozeHint(p.until, now)}</span>
            </button>
          ))}
          <div className="menu-sep" />
          <button type="button" className="menu-item" role="menuitem" onClick={() => setCustom(true)}>
            <CalendarClock size={14} strokeWidth={1.75} />
            <span className="grow">Custom…</span>
          </button>
        </>
      )}
    </div>
  );
}

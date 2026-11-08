import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { useStore } from "@app/lib/store";
import { snoozePresets } from "@app/lib/time";
import { expandSpring } from "@app/lib/motion";

const TARGET = "keys";

/** A Windows-style notification (static markup) that slides in once and drives the real store. */
export function ToastMock({ play, replayKey, base }: { play: boolean; replayKey: number; base: string }) {
  const item = useStore((s) => s.items.find((i) => i.id === TARGET));
  const [show, setShow] = useState(false);
  const [preset, setPreset] = useState("10m");

  useEffect(() => {
    setShow(false);
    if (!play) return;
    const t = setTimeout(() => setShow(true), 1800);
    return () => clearTimeout(t);
  }, [play, replayKey]);

  const live = !!item && item.status === "open" && item.snoozedUntil == null;
  const visible = show && live;
  const now = Date.now();
  const presets = snoozePresets(now);

  const done = () => {
    setShow(false);
    useStore.getState().completeAnimated(TARGET);
  };
  const snooze = () => {
    setShow(false);
    const p = presets.find((x) => x.id === preset) ?? presets[0];
    void useStore.getState().snooze(TARGET, p.until);
  };

  return (
    <motion.div
      className="x-toast"
      role="alert"
      aria-hidden={!visible}
      initial={false}
      animate={visible ? { x: 0, opacity: 1 } : { x: 56, opacity: 0 }}
      transition={expandSpring}
      style={{ pointerEvents: visible ? "auto" : "none" }}
    >
      <div className="x-toast-app">
        <img src={`${base}favicon.svg`} alt="" width={14} height={14} />
        <span>Styky</span>
      </div>
      <div className="x-toast-title">{item?.title ?? "Rotate the production API keys"}</div>
      <div className="x-toast-line tnum">
        Due {new Date(item?.dueAt ?? now).toDateString() === new Date(now).toDateString() ? "Today" : "Yesterday"}{" "}
        {new Date(item?.dueAt ?? now).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}
      </div>
      <div className="x-toast-line x-toast-tag">#{item?.tag ?? "security"}</div>
      <div className="x-toast-actions">
        <select aria-label="Snooze for" value={preset} onChange={(e) => setPreset(e.target.value)} tabIndex={visible ? 0 : -1}>
          {presets.map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
        <button type="button" onClick={snooze} tabIndex={visible ? 0 : -1}>
          Snooze
        </button>
        <button type="button" onClick={done} tabIndex={visible ? 0 : -1}>
          Done
        </button>
      </div>
    </motion.div>
  );
}

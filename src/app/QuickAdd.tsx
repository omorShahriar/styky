import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CalendarClock, GitBranch, Hash, Link2, Plus, Repeat, Flag } from "lucide-react";
import { api } from "../lib/api";
import { parseQuick, toItemEdit } from "../lib/nlp";
import type { QuickChip } from "../lib/nlp";
import { initTheme } from "../lib/theme";
import "./QuickAdd.css";

const WIDTH = 560;
const BAR = 64;
const CHIPS = 36;

const ICONS: Record<QuickChip["kind"], typeof Hash> = {
  date: CalendarClock,
  tag: Hash,
  priority: Flag,
  repeat: Repeat,
  branch: GitBranch,
  link: Link2,
};

export default function QuickAdd() {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const parsed = useMemo(() => parseQuick(text), [text]);

  useEffect(() => {
    let off: (() => void) | undefined;
    let cancelled = false;
    void initTheme().then((u) => (cancelled ? u() : (off = u)));
    // Reset and focus every time the bar is summoned.
    const shown = listen("quickadd-shown", () => {
      setText("");
      setError("");
      setBusy(false);
      input.current?.focus();
    });
    input.current?.focus();
    return () => {
      cancelled = true;
      off?.();
      void shown.then((u) => u());
    };
  }, []);

  // Grow the window under the input when there are chips to show.
  const rows = parsed.chips.length > 0 || error ? BAR + CHIPS : BAR;
  useEffect(() => {
    void getCurrentWindow().setSize(new LogicalSize(WIDTH, rows));
  }, [rows]);

  const submit = async () => {
    if (busy) return;
    if (!parsed.title) {
      setError("Type a title first");
      return;
    }
    setBusy(true);
    try {
      await api.createItem(toItemEdit(parsed));
      await api.hideQuickadd();
      setText("");
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      void submit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      void api.hideQuickadd();
    }
  };

  return (
    <div className="qa">
      <div className="qa-bar">
        <Plus size={20} strokeWidth={1.75} className="qa-icon" aria-hidden />
        <input
          ref={input}
          className="qa-input"
          value={text}
          autoFocus
          spellCheck={false}
          autoComplete="off"
          placeholder="Release v2.3 tomorrow 5pm #api !!"
          aria-label="New reminder"
          onChange={(e) => {
            setText(e.target.value);
            setError("");
          }}
          onKeyDown={onKeyDown}
        />
        <kbd className="qa-kbd">↵</kbd>
      </div>
      {(parsed.chips.length > 0 || error) && (
        <div className="qa-chips" aria-live="polite">
          {error ? (
            <span className="qa-error">{error}</span>
          ) : (
            parsed.chips.map((c) => {
              const Icon = ICONS[c.kind];
              return (
                <span key={c.kind} className={`qa-chip ${c.kind}${c.kind === "priority" && parsed.priority === 2 ? " urgent" : ""}`}>
                  <Icon size={12} strokeWidth={2} />
                  {c.label}
                </span>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}

import { useEffect, useMemo, useRef, useState } from "react";
import { CalendarClock, Flag, GitBranch, Hash, Link2, Plus, Repeat } from "lucide-react";
import { parseQuick } from "@app/lib/nlp";
import type { QuickChip } from "@app/lib/nlp";

const ICONS: Record<QuickChip["kind"], typeof Hash> = {
  date: CalendarClock,
  tag: Hash,
  priority: Flag,
  repeat: Repeat,
  branch: GitBranch,
  link: Link2,
};

export const prefersReducedMotion = () =>
  typeof window !== "undefined" && !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/** Chips for a line of text, produced by the app's real quick-add parser. */
export function Chips({ text }: { text: string }) {
  const parsed = useMemo(() => parseQuick(text), [text]);
  if (parsed.chips.length === 0) return null;
  return (
    <div className="qa-chips" aria-hidden>
      {parsed.chips.map((c) => {
        const Icon = ICONS[c.kind];
        return (
          <span key={c.kind} className={`qa-chip ${c.kind}${c.kind === "priority" && parsed.priority === 2 ? " urgent" : ""}`}>
            <Icon size={12} strokeWidth={2} />
            {c.label}
          </span>
        );
      })}
    </div>
  );
}

interface Props {
  phrases: string[];
  /** index of the phrase being typed (controlled by the parent) */
  index: number;
  /** called when a phrase has been typed and held, so the parent can move on */
  onDone?: () => void;
  paused?: boolean;
  className?: string;
  onActivate?: () => void;
}

/**
 * A quick-add bar that types a phrase out character by character. It renders the same markup
 * and CSS classes as the real QuickAdd and uses the real parser for the chips.
 */
export function TypeBar({ phrases, index, onDone, paused, className = "", onActivate }: Props) {
  const phrase = phrases[index % phrases.length];
  const [n, setN] = useState(0);
  const doneRef = useRef(onDone);
  doneRef.current = onDone;
  const reduced = prefersReducedMotion();

  useEffect(() => {
    if (reduced) {
      setN(phrase.length);
      return;
    }
    setN(0);
  }, [phrase, reduced]);

  useEffect(() => {
    if (paused || reduced) return;
    let t: ReturnType<typeof setTimeout>;
    if (n < phrase.length) {
      t = setTimeout(() => setN((v) => v + 1), n === 0 ? 420 : 34 + ((n * 37) % 38));
    } else {
      t = setTimeout(() => doneRef.current?.(), 2600);
    }
    return () => clearTimeout(t);
  }, [n, phrase, paused, reduced]);

  const text = phrase.slice(0, n);
  const interactive = !!onActivate;

  return (
    <div
      className={`qa x-typebar ${className}`}
      role={interactive ? "button" : "img"}
      tabIndex={interactive ? 0 : undefined}
      aria-label={interactive ? "Try quick add" : `Quick add example: ${phrase}`}
      onClick={onActivate}
      onKeyDown={(e) => interactive && (e.key === "Enter" || e.key === " ") && (e.preventDefault(), onActivate?.())}
      style={interactive ? { cursor: "text" } : undefined}
    >
      <div className="qa-bar">
        <Plus size={20} strokeWidth={1.75} className="qa-icon" aria-hidden />
        <div className="qa-input x-typebar-text" aria-hidden>
          {text ? (
            <>
              {text}
              {!reduced && <span className="x-caret" />}
            </>
          ) : (
            <span className="x-ph">Release v2.3 tomorrow 5pm #api !!</span>
          )}
        </div>
        <kbd className="qa-kbd">↵</kbd>
      </div>
      <Chips text={text} />
    </div>
  );
}

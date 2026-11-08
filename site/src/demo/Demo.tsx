import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { MotionConfig } from "motion/react";
import { RotateCcw } from "lucide-react";
import Panel from "@app/app/Panel";
import QuickAdd from "@app/app/QuickAdd";
import { useStore } from "@app/lib/store";
import { on } from "../mock/events";
import { db } from "../mock/db";
import { TypeBar } from "./TypeBar";
import { ToastMock } from "./ToastMock";
import "@app/app/QuickAdd.css";
import "./demo.css";

const HERO_PHRASES = [
  "Release v2.3 tomorrow 5pm #api !!",
  "Rotate the API keys every month #security",
  "Reply to Dana fri 10am @contract !",
];

const WIDE = { w: 1160, h: 660 };
const NARROW = { w: 440, h: 940 };

/** Keys the real Panel listens for on `window`; they must not act while the visitor is elsewhere on the page. */
function useKeyboardScope(stage: React.RefObject<HTMLElement | null>) {
  const [armed, setArmed] = useState(false);
  const armedRef = useRef(false);
  armedRef.current = armed;

  useEffect(() => {
    const guard = (e: KeyboardEvent) => {
      if (!armedRef.current) e.stopImmediatePropagation();
    };
    const inside = (t: EventTarget | null) => !!stage.current && t instanceof Node && stage.current.contains(t);
    const arm = (e: Event) => setArmed(inside(e.target));
    window.addEventListener("keydown", guard, true);
    document.addEventListener("pointerdown", arm, true);
    document.addEventListener("focusin", arm, true);
    return () => {
      window.removeEventListener("keydown", guard, true);
      document.removeEventListener("pointerdown", arm, true);
      document.removeEventListener("focusin", arm, true);
    };
  }, [stage]);

  return armed;
}

/**
 * The snooze popover is `position: fixed` and positioned from viewport rects. Inside a scaled
 * (transformed) stage that becomes stage-relative, so convert its coordinates back.
 */
function useFixedFix(stage: React.RefObject<HTMLElement | null>, scale: number) {
  useEffect(() => {
    const el = stage.current;
    if (!el || scale === 1) return;
    const fix = (m: HTMLElement) => {
      const raw = `${m.style.left}|${m.style.top}`;
      if (m.dataset.fixed === raw) return;
      const id = useStore.getState().snoozeFor?.id;
      const row = id ? el.querySelector(`[data-id="${id}"]`) : null;
      if (!row) return;
      // Re-place the popover in stage coordinates (the app clamps to the viewport, which is wrong inside a scaled stage).
      const sr = el.getBoundingClientRect();
      const rr = row.getBoundingClientRect();
      const rowRight = (rr.right - sr.left) / scale;
      const rowTop = (rr.top - sr.top) / scale;
      const w = m.offsetWidth;
      const h = m.offsetHeight;
      const left = Math.max(8, Math.min(el.offsetWidth - w - 8, rowRight - w - 8));
      let top = rowTop + 36;
      if (top + h > el.offsetHeight - 8) top = Math.max(8, Math.min(rowTop - h - 4, el.offsetHeight - h - 8));
      m.style.left = `${left}px`;
      m.style.top = `${top}px`;
      m.dataset.fixed = `${m.style.left}|${m.style.top}`; // read back: the browser may round what was written
    };
    const mo = new MutationObserver(() => el.querySelectorAll<HTMLElement>(".snooze-menu").forEach(fix));
    mo.observe(el, { subtree: true, childList: true, attributes: true, attributeFilter: ["style"] });
    return () => mo.disconnect();
  }, [stage, scale]);
}

function useScale(box: React.RefObject<HTMLElement | null>) {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(el.clientWidth));
    ro.observe(el);
    setWidth(el.clientWidth);
    return () => ro.disconnect();
  }, [box]);
  const narrow = width > 0 && width < 760;
  const design = narrow ? NARROW : WIDE;
  const scale = width > 0 ? Math.min(1, width / design.w) : 1;
  return { narrow, design, scale, measured: width > 0 };
}

function Taskbar() {
  const [now] = useState(() => new Date());
  return (
    <div className="x-taskbar" aria-hidden>
      <div className="x-task-icons">
        <span className="x-win" />
        <span className="x-t x-t1" />
        <span className="x-t x-t2" />
        <span className="x-t x-t3" />
        <span className="x-t x-t4" />
      </div>
      <div className="x-clock">
        <span>{now.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</span>
        <span>{now.toLocaleDateString([], { month: "numeric", day: "numeric", year: "numeric" })}</span>
      </div>
    </div>
  );
}

export default function Demo({ base }: { base: string }) {
  const box = useRef<HTMLDivElement>(null);
  const stage = useRef<HTMLDivElement>(null);
  const { narrow, design, scale, measured } = useScale(box);
  const armed = useKeyboardScope(stage);
  useFixedFix(stage, scale);

  const [inView, setInView] = useState(false);
  const [phrase, setPhrase] = useState(0);
  const [qaLive, setQaLive] = useState(false);
  const [qaHeight, setQaHeight] = useState(64);
  const [replay, setReplay] = useState(0);

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const io = new IntersectionObserver(([e]) => setInView(e.isIntersecting), { threshold: 0.35 });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useEffect(() => on("demo-window-size", (e) => setQaHeight((e.payload as { height: number }).height)), []);

  const reset = useCallback(() => {
    db.reset();
    void useStore.getState().refresh();
    useStore.getState().expand(null);
    setReplay((n) => n + 1);
  }, []);

  return (
    <MotionConfig reducedMotion="user">
      <div className="x-demo" ref={box}>
        <div className="x-demo-fit" style={{ height: measured ? design.h * scale : design.h * 0.6 }}>
          <div
            ref={stage}
            className={`x-stage${narrow ? " is-narrow" : ""}${armed ? " is-armed" : ""}`}
            style={{
              width: design.w,
              height: design.h,
              transform: scale === 1 ? undefined : `scale(${scale})`,
              visibility: measured ? "visible" : "hidden",
            }}
          >
            <div className="x-wallpaper" aria-hidden>
              <i className="x-bloom x-b1" />
              <i className="x-bloom x-b2" />
              <i className="x-bloom x-b3" />
              <i className="x-sheen" />
            </div>

            <div className="x-qa-label" aria-hidden>
              <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>N</kbd>
              <span>from anywhere</span>
            </div>
            <div className="x-qa-win" style={qaLive ? { height: qaHeight } : undefined}>
              {qaLive ? (
                <QuickAdd />
              ) : (
                <TypeBar
                  phrases={HERO_PHRASES}
                  index={phrase}
                  onDone={() => setPhrase((p) => (p + 1) % HERO_PHRASES.length)}
                  paused={!inView}
                  onActivate={() => setQaLive(true)}
                />
              )}
            </div>
            {!qaLive && (
              <div className="x-try" aria-hidden>
                Click the bar and type your own
              </div>
            )}

            <div className="x-panel-win">
              <Panel />
            </div>

            <ToastMock play={inView} replayKey={replay} base={base} />
            <Taskbar />
          </div>
        </div>
        <div className="x-demo-foot">
          <p>
            The real Styky UI, running in your browser. Click a reminder to expand it, tick the checklists,
            press <kbd>Space</kbd> to complete, <kbd>S</kbd> to snooze. Nothing is saved.
          </p>
          <button type="button" className="x-link-btn" onClick={reset}>
            <RotateCcw size={13} strokeWidth={2} /> Reset demo
          </button>
        </div>
      </div>
    </MotionConfig>
  );
}

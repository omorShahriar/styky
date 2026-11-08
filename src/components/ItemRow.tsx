import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, Reorder, motion, useDragControls } from "motion/react";
import { AlarmClock, Check, Copy, ExternalLink, GripVertical, Pencil, Repeat as RepeatIcon, Trash2 } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/api";
import { toggleTaskLine } from "../lib/markdown";
import { expandSpring, pressSpring, rowVariants } from "../lib/motion";
import { useStore } from "../lib/store";
import { dueState, formatDue } from "../lib/time";
import { toEdit } from "../lib/types";
import type { Item } from "../lib/types";
import { CheckIcon } from "./CheckIcon";
import { TagChip } from "./Chip";
import { Editor } from "./Editor";
import { MarkdownView } from "./MarkdownView";

interface Props {
  item: Item;
  /** Rendered inside a `Reorder.Group`: shows a drag grip. */
  reorderable?: boolean;
  onDragEnd?: () => void;
}

function hostOf(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

export function ItemRow({ item, reorderable, onDragEnd }: Props) {
  const {
    now,
    items,
    focusedId,
    expandedId,
    editing,
    checking,
    focus,
    expand,
    setEditing,
    completeAnimated,
    restore,
    remove,
    openSnooze,
    showToast,
  } = useStore();
  const controls = useDragControls();
  const ref = useRef<HTMLLIElement>(null);
  const [copied, setCopied] = useState(false);

  const done = item.status === "done";
  const expanded = expandedId === item.id;
  const isEditing = expanded && editing;
  const focused = focusedId === item.id;
  const isChecking = checking.includes(item.id);
  const eff = item.snoozedUntil ?? item.dueAt;
  const state = done ? "none" : dueState(eff, now);

  // Keep DOM focus on the keyboard cursor (so :focus-visible rings appear for keyboard use).
  useEffect(() => {
    if (focused && !isEditing) {
      const el = ref.current;
      if (el && !el.contains(document.activeElement)) {
        el.focus({ preventScroll: true });
        el.scrollIntoView({ block: "nearest" });
      }
    }
  }, [focused, isEditing]);

  // Click outside collapses the expanded row.
  useEffect(() => {
    if (!expanded) return;
    const down = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) expand(null);
    };
    document.addEventListener("mousedown", down);
    return () => document.removeEventListener("mousedown", down);
  }, [expanded, expand]);

  const tags = useMemo(
    () => [...new Set(items.map((i) => i.tag).filter((t): t is string => !!t))].sort(),
    [items],
  );

  const onToggleTask = useCallback(
    (line: number) => {
      const bodyMd = toggleTaskLine(item.bodyMd, line);
      if (bodyMd !== item.bodyMd) {
        api
          .updateItem(item.id, { ...toEdit(item), bodyMd })
          .then(() => useStore.getState().refresh())
          .catch((e) => showToast(String(e)));
      }
    },
    [item, showToast],
  );

  const openLink = () => {
    if (item.link) openUrl(item.link).catch((e) => showToast(`Could not open link: ${String(e)}`));
  };

  const copyBranch = async () => {
    if (!item.branch) return;
    try {
      await writeText(item.branch);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch (e) {
      showToast(`Could not copy: ${String(e)}`);
    }
  };

  const meta: React.ReactNode[] = [];
  if (item.tag) meta.push(<TagChip key="tag" tag={item.tag} />);
  if (done && item.completedAt) {
    meta.push(
      <span key="done" className="meta-text tnum">
        Done {formatDue(item.completedAt, now)}
      </span>,
    );
  } else if (eff != null) {
    meta.push(
      <span key="due" className={`meta-text due ${state} tnum`}>
        {state === "overdue" ? "Overdue · " : ""}
        {formatDue(eff, now)}
      </span>,
    );
  }
  if (item.repeat !== "none") {
    meta.push(
      <span key="rep" className="meta-text" title={`Repeats ${item.repeat}`}>
        <RepeatIcon size={12} strokeWidth={2} aria-label={`Repeats ${item.repeat}`} />
      </span>,
    );
  }
  if (!done && item.priority === 0) {
    meta.push(
      <span key="low" className="meta-text">
        Low
      </span>,
    );
  }

  const cls = [
    "item",
    done ? "done" : "",
    state === "overdue" ? "overdue" : "",
    expanded ? "expanded" : "",
    focused ? "focused" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const body = (
    <>
      <div
        className="row"
        onClick={() => {
          focus(item.id);
          expand(expanded ? null : item.id);
        }}
      >
        {reorderable && (
          <span
            className="grip"
            aria-hidden
            onPointerDown={(e) => {
              e.stopPropagation();
              controls.start(e);
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <GripVertical size={14} strokeWidth={2} />
          </span>
        )}
        <motion.button
          type="button"
          role="checkbox"
          aria-checked={done || isChecking}
          aria-label={done ? "Restore" : "Complete"}
          className={`check${done || isChecking ? " on" : ""}`}
          whileTap={{ scale: 0.88 }}
          transition={pressSpring}
          onClick={(e) => {
            e.stopPropagation();
            if (done) void restore(item.id);
            else completeAnimated(item.id);
          }}
        >
          {(done || isChecking) && <CheckIcon />}
        </motion.button>
        <div className="main">
          <div className="title-line">
            <span className="title">{item.title}</span>
            {item.priority === 2 && !done && <span className="urgent-dot" title="Urgent" />}
          </div>
          {meta.length > 0 && (
            <div className="meta">
              {meta.map((m, i) => (
                <span className="meta-part" key={i}>
                  {m}
                </span>
              ))}
            </div>
          )}
        </div>
        <div className="quick">
          {!done && (
            <button type="button" className="icon-btn sm" title="Snooze (S)" aria-label="Snooze" onClick={(e) => { e.stopPropagation(); openSnooze(item.id); }}>
              <AlarmClock size={13} strokeWidth={2} />
            </button>
          )}
          {item.link && (
            <button type="button" className="icon-btn sm" title="Open link" aria-label="Open link" onClick={(e) => { e.stopPropagation(); openLink(); }}>
              <ExternalLink size={13} strokeWidth={2} />
            </button>
          )}
          {item.branch && (
            <button type="button" className="icon-btn sm" title="Copy branch" aria-label="Copy branch" onClick={(e) => { e.stopPropagation(); void copyBranch(); }}>
              <Copy size={13} strokeWidth={2} />
            </button>
          )}
        </div>
      </div>

      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            key="detail"
            className="detail"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={expandSpring}
          >
            <div className="detail-inner">
              {isEditing ? (
                <Editor
                  initial={toEdit(item)}
                  itemId={item.id}
                  tags={tags}
                  onClose={() => setEditing(false)}
                  onError={showToast}
                />
              ) : (
                <>
                  {item.bodyMd.trim() ? (
                    <MarkdownView source={item.bodyMd} onToggleTask={onToggleTask} onError={showToast} />
                  ) : (
                    <p className="muted">No details.</p>
                  )}
                  {(item.link || item.branch) && (
                    <div className="refs">
                      {item.link && (
                        <button type="button" className="ref-link" onClick={openLink} title={item.link}>
                          <ExternalLink size={12} strokeWidth={2} />
                          <span>{hostOf(item.link)}</span>
                        </button>
                      )}
                      {item.branch && (
                        <button type="button" className="chip branch-chip" onClick={() => void copyBranch()} title="Copy">
                          {copied ? (
                            <>
                              <Check size={11} strokeWidth={2.4} /> Copied
                            </>
                          ) : (
                            item.branch
                          )}
                        </button>
                      )}
                    </div>
                  )}
                  <div className="detail-actions">
                    <button type="button" className="btn" onClick={() => openSnooze(item.id)}>
                      <AlarmClock size={13} strokeWidth={2} /> Snooze
                    </button>
                    <button type="button" className="btn" onClick={() => setEditing(true)}>
                      <Pencil size={13} strokeWidth={2} /> Edit
                    </button>
                    <button type="button" className="btn danger" onClick={() => void remove(item)}>
                      <Trash2 size={13} strokeWidth={2} /> Delete
                    </button>
                  </div>
                </>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );

  if (reorderable) {
    return (
      <Reorder.Item
        as="li"
        ref={ref}
        value={item.id}
        className={cls}
        tabIndex={-1}
        aria-expanded={expanded}
        dragListener={false}
        dragControls={controls}
        onDragEnd={onDragEnd}
        transition={expandSpring}
        variants={rowVariants}
        initial="initial"
        animate="animate"
        exit="exit"
        data-id={item.id}
      >
        {body}
      </Reorder.Item>
    );
  }

  return (
    <motion.li
      ref={ref}
      layout="position"
      className={cls}
      tabIndex={-1}
      aria-expanded={expanded}
      transition={expandSpring}
      variants={rowVariants}
      initial="initial"
      animate="animate"
      exit="exit"
      data-id={item.id}
    >
      {body}
    </motion.li>
  );
}

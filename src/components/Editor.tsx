import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { motion } from "motion/react";
import { X } from "lucide-react";
import { api } from "../lib/api";
import { fromLocalInput, toLocalInput } from "../lib/time";
import type { ItemEdit, Priority } from "../lib/types";
import { pressSpring } from "../lib/motion";
import { usePro } from "../lib/pro";
import { ProPill } from "./ProPill";
import { RepeatField } from "./RepeatField";

const PRIORITIES: { value: Priority; label: string }[] = [
  { value: 0, label: "Low" },
  { value: 1, label: "Normal" },
  { value: 2, label: "Urgent" },
  { value: 3, label: "Critical" },
];

const PREALERTS: { value: number | null; label: string }[] = [
  { value: null, label: "None" },
  { value: 5, label: "5 minutes before" },
  { value: 15, label: "15 minutes before" },
  { value: 30, label: "30 minutes before" },
  { value: 60, label: "1 hour before" },
  { value: 1440, label: "1 day before" },
];

const SAVE_DEBOUNCE_MS = 400;

interface Props {
  /** The values when editing began. */
  initial: ItemEdit;
  /** Existing item id, or null for a new item (created on the first valid save). */
  itemId: string | null;
  /** Existing tags, offered as suggestions. */
  tags: string[];
  /** A new item got an id from its first save. */
  onCreated?: (id: string) => void;
  onClose: () => void;
  onError: (message: string) => void;
}

/**
 * Inline editor. Autosaves 400 ms after the last change; Ctrl+Enter saves and
 * closes; Esc reverts to how things were when editing began and closes.
 */
export function Editor({ initial, itemId, tags, onCreated, onClose, onError }: Props) {
  // `initial` is a fresh object each render; keep what editing began with.
  const original = useRef(initial).current;
  const pro = usePro();
  const [edit, setEdit] = useState<ItemEdit>(original);
  const editRef = useRef(edit);
  const idRef = useRef(itemId);
  const dirty = useRef(false);
  const everSaved = useRef(false);
  const cancelled = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const chain = useRef<Promise<void>>(Promise.resolve());
  const bodyRef = useRef<HTMLTextAreaElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  // Callbacks live in a ref so `flush` (and the unmount cleanup) stay stable.
  const cb = useRef({ onCreated, onError });
  cb.current = { onCreated, onError };

  // Saves are serialised so a create can never race the update that follows it.
  const flush = useCallback(() => {
    clearTimeout(timer.current);
    chain.current = chain.current.then(async () => {
      if (cancelled.current || !dirty.current) return;
      const value = editRef.current;
      if (!value.title.trim()) return; // never save an untitled item
      dirty.current = false;
      try {
        if (idRef.current) {
          await api.updateItem(idRef.current, value);
        } else {
          const created = await api.createItem(value);
          idRef.current = created.id;
          cb.current.onCreated?.(created.id);
        }
        everSaved.current = true;
      } catch (e) {
        dirty.current = true;
        cb.current.onError(`Could not save: ${String(e)}`);
      }
    });
    return chain.current;
  }, []);

  const patch = (p: Partial<ItemEdit>) => {
    const next = { ...editRef.current, ...p };
    editRef.current = next;
    dirty.current = true;
    setEdit(next);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void flush(), SAVE_DEBOUNCE_MS);
  };

  // Persist pending changes if the editor is torn down (outside click, collapse...).
  useEffect(() => {
    titleRef.current?.focus();
    return () => {
      void flush();
    };
  }, [flush]);

  const grow = () => {
    const el = bodyRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  };
  useLayoutEffect(grow, [edit.bodyMd]);

  const cancel = async () => {
    cancelled.current = true;
    clearTimeout(timer.current);
    await chain.current;
    try {
      if (idRef.current && itemId === null) {
        await api.deleteItem(idRef.current); // a draft that was already saved
      } else if (idRef.current && everSaved.current) {
        await api.updateItem(idRef.current, original);
      }
    } catch (e) {
      onError(String(e));
    }
    onClose();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      void cancel();
    } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();
      void flush().then(onClose);
    } else if (e.key !== "Tab") {
      e.stopPropagation(); // typing must never trigger list shortcuts
    }
  };

  return (
    <div className="editor" onKeyDown={onKeyDown} onClick={(e) => e.stopPropagation()}>
      <input
        ref={titleRef}
        className="ed-title"
        placeholder="What needs doing?"
        value={edit.title}
        onChange={(e) => patch({ title: e.target.value })}
        aria-label="Title"
      />
      <textarea
        ref={bodyRef}
        className="ed-body"
        placeholder={"Details (markdown). Use - [ ] for a checklist."}
        value={edit.bodyMd}
        rows={2}
        onChange={(e) => patch({ bodyMd: e.target.value })}
        aria-label="Details"
      />

      <div className="ed-field">
        <span className="ed-label">Due</span>
        <div className="ed-inline">
          <input
            type="datetime-local"
            className="ed-input tnum"
            value={toLocalInput(edit.dueAt)}
            onChange={(e) => patch({ dueAt: fromLocalInput(e.target.value) })}
            aria-label="Due date and time"
          />
          {edit.dueAt != null && (
            <button
              type="button"
              className="icon-btn sm"
              aria-label="Clear due date"
              onClick={() => patch({ dueAt: null, repeat: "none" })}
            >
              <X size={13} strokeWidth={2} />
            </button>
          )}
        </div>
      </div>

      <RepeatField value={edit.repeat} dueAt={edit.dueAt} onChange={(repeat) => patch({ repeat })} />

      <div className="ed-field">
        <span className="ed-label">
          Early alert {!pro && <ProPill />}
        </span>
        <select
          className="ed-input"
          value={edit.prealertMin ?? ""}
          disabled={edit.dueAt == null || !pro}
          title={edit.dueAt == null ? "Set a due time first" : undefined}
          onChange={(e) => patch({ prealertMin: e.target.value === "" ? null : Number(e.target.value) })}
          aria-label="Remind early"
        >
          {PREALERTS.map((r) => (
            <option key={r.label} value={r.value ?? ""}>
              {r.label}
            </option>
          ))}
        </select>
      </div>

      <div className="ed-field">
        <span className="ed-label">
          Nag {!pro && <ProPill />}
        </span>
        <select
          className="ed-input"
          value={edit.nag ? "1" : "0"}
          disabled={edit.dueAt == null || (!pro && !edit.nag)}
          title={edit.dueAt == null ? "Set a due time first" : undefined}
          onChange={(e) => patch({ nag: e.target.value === "1" })}
          aria-label="Nag until done"
        >
          <option value="0">Off</option>
          <option value="1">Until done or snoozed</option>
        </select>
      </div>

      <div className="ed-field">
        <span className="ed-label">
          Priority
        </span>
        <div className="segmented" role="radiogroup" aria-label="Priority">
          {PRIORITIES.map((p) => (
            <button
              key={p.value}
              type="button"
              role="radio"
              aria-checked={edit.priority === p.value}
              className={edit.priority === p.value ? "on" : ""}
              onClick={() => patch({ priority: p.value })}
            >
              {edit.priority === p.value && (
                <motion.span layoutId="prio-thumb" className="thumb" transition={pressSpring} />
              )}
              <span className="seg-label">{p.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="ed-field">
        <span className="ed-label">Tag</span>
        <input
          className="ed-input"
          list="styky-tags"
          placeholder="project"
          value={edit.tag ?? ""}
          onChange={(e) => patch({ tag: e.target.value })}
          aria-label="Tag"
        />
        <datalist id="styky-tags">
          {tags.map((t) => (
            <option key={t} value={t} />
          ))}
        </datalist>
      </div>

      <div className="ed-field">
        <span className="ed-label">Link</span>
        <input
          className="ed-input"
          type="url"
          placeholder="https://"
          value={edit.link ?? ""}
          onChange={(e) => patch({ link: e.target.value })}
          aria-label="Link"
        />
      </div>

      <div className="ed-field">
        <span className="ed-label">Branch</span>
        <input
          className="ed-input mono"
          placeholder="release/2.3"
          value={edit.branch ?? ""}
          onChange={(e) => patch({ branch: e.target.value })}
          aria-label="Branch or repo"
        />
      </div>

      <p className="ed-hint">
        <kbd>Ctrl</kbd> <kbd>Enter</kbd> done · <kbd>Esc</kbd> cancel · saves as you type
      </p>
    </div>
  );
}

import { useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, LayoutGroup, motion } from "motion/react";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { CheckCheck, Download, Ellipsis, Plus, Power, Settings, Upload } from "lucide-react";
import { initTheme } from "../lib/theme";
import { api } from "../lib/api";
import { useStore } from "../lib/store";
import { endOfToday } from "../lib/time";
import { emptyEdit } from "../lib/types";
import type { Item } from "../lib/types";
import { expandSpring } from "../lib/motion";
import { CompletedSection } from "../components/CompletedSection";
import { Editor } from "../components/Editor";
import { SnoozeMenu } from "../components/SnoozeMenu";
import { ItemRow } from "../components/ItemRow";
import { PlainGroup } from "../components/PlainGroup";
import { Toast } from "../components/Toast";
import "./Panel.css";
import "./Items.css";

const TICK_MS = 30_000;

function Menu({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const showToast = useStore((s) => s.showToast);
  const refresh = useStore((s) => s.refresh);

  useEffect(() => {
    const down = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("mousedown", down);
    window.addEventListener("blur", onClose);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", down);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("keydown", key);
    };
  }, [onClose]);

  const doExport = async () => {
    onClose();
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await save({
        defaultPath: `styky-export-${stamp}.json`,
        filters: [{ name: "Styky export", extensions: ["json"] }],
      });
      if (!path) return;
      const n = await api.exportJson(path);
      showToast(`Exported ${n} reminder${n === 1 ? "" : "s"}`);
    } catch (e) {
      showToast(`Export failed: ${String(e)}`);
    }
  };

  const doImport = async (mode: "merge" | "replace") => {
    onClose();
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Styky export", extensions: ["json"] }],
      });
      if (!path || Array.isArray(path)) return;
      if (mode === "replace") {
        const ok = await ask("This replaces all current reminders with the contents of the file.", {
          title: "Replace everything?",
          kind: "warning",
          okLabel: "Replace",
          cancelLabel: "Cancel",
        });
        if (!ok) return;
      }
      const r = await api.importJson(path, mode);
      await refresh();
      showToast(
        mode === "replace"
          ? `Imported ${r.added} reminder${r.added === 1 ? "" : "s"}`
          : `Imported: ${r.added} new, ${r.updated} updated, ${r.skipped} unchanged`,
      );
    } catch (e) {
      showToast(`Import failed: ${String(e)}`);
    }
  };

  return (
    <div className="menu" role="menu" ref={ref}>
      <button className="menu-item" role="menuitem" onClick={() => { onClose(); void api.openSettings(); }}>
        <Settings size={14} strokeWidth={1.75} /> Settings…
      </button>
      <div className="menu-sep" />
      <button className="menu-item" role="menuitem" onClick={() => void doExport()}>
        <Download size={14} strokeWidth={1.75} /> Export…
      </button>
      <button className="menu-item" role="menuitem" onClick={() => void doImport("merge")}>
        <Upload size={14} strokeWidth={1.75} /> Import (merge)…
      </button>
      <button className="menu-item" role="menuitem" onClick={() => void doImport("replace")}>
        <Upload size={14} strokeWidth={1.75} /> Import (replace)…
      </button>
      <div className="menu-sep" />
      <button className="menu-item" role="menuitem" onClick={() => api.quit()}>
        <Power size={14} strokeWidth={1.75} /> Quit Styky
      </button>
    </div>
  );
}

/** A brand-new item being written. It is created on the first save with a title. */
function DraftCard({ tags }: { tags: string[] }) {
  const setDraftId = useStore((s) => s.setDraftId);
  const closeDraft = useStore((s) => s.closeDraft);
  const showToast = useStore((s) => s.showToast);
  const ref = useRef<HTMLDivElement>(null);
  const initial = useMemo(emptyEdit, []);

  useEffect(() => {
    const down = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) closeDraft();
    };
    document.addEventListener("mousedown", down);
    return () => document.removeEventListener("mousedown", down);
  }, [closeDraft]);

  return (
    <motion.div
      ref={ref}
      className="item expanded draft"
      initial={{ opacity: 0, height: 0 }}
      animate={{ opacity: 1, height: "auto" }}
      exit={{ opacity: 0, height: 0 }}
      transition={expandSpring}
    >
      <div className="detail-inner">
        <Editor
          initial={initial}
          itemId={null}
          tags={tags}
          onCreated={setDraftId}
          onClose={closeDraft}
          onError={showToast}
        />
      </div>
    </motion.div>
  );
}

function isTyping(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
}

export default function Panel() {
  const [menuOpen, setMenuOpen] = useState(false);
  const { items, now, loaded, draft, completedOpen, settings } = useStore();
  const hotkey = (settings.quickadd_hotkey || "Ctrl+Alt+N").split("+").map((k) => (k === "Super" ? "Win" : k));

  // theme + data wiring
  useEffect(() => {
    let off: (() => void) | undefined;
    let cancelled = false;
    initTheme().then((u) => (cancelled ? u() : (off = u)));
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  useEffect(() => {
    const { refresh, tick } = useStore.getState();
    void refresh();
    const unlisten = [
      listen("items-changed", () => void refresh()),
      listen("settings-changed", () => void refresh()),
      // A toast body or "Custom..." click: open the panel at that item.
      listen<{ id: string | null; snooze: boolean }>("focus-item", async (e) => {
        const { id, snooze } = e.payload;
        if (!id) return;
        await useStore.getState().refresh();
        const s = useStore.getState();
        const item = s.items.find((i) => i.id === id);
        if (!item) return;
        s.focus(id);
        s.expand(id);
        if (snooze && item.status === "open") s.openSnooze(id, true);
      }),
    ];
    const timer = setInterval(tick, TICK_MS);
    return () => {
      clearInterval(timer);
      unlisten.forEach((p) => void p.then((u) => u()));
    };
  }, []);

  // derived lists (backend already sorted them)
  const { lead, plain, done, open } = useMemo(() => {
    const hidden = draft?.id ?? null;
    const visible = items.filter((i) => i.id !== hidden);
    const openItems = visible.filter((i) => i.status === "open");
    const isPlain = (i: Item) => i.dueAt == null && i.snoozedUntil == null && i.priority !== 2;
    return {
      open: openItems,
      lead: openItems.filter((i) => !isPlain(i)),
      plain: openItems.filter(isPlain),
      done: visible.filter((i) => i.status === "done"),
    };
  }, [items, draft?.id]);

  const dueToday = useMemo(() => {
    const end = endOfToday(now);
    return open.filter((i) => (i.snoozedUntil ?? i.dueAt) != null && (i.snoozedUntil ?? i.dueAt)! <= end).length;
  }, [open, now]);

  const tags = useMemo(
    () => [...new Set(items.map((i) => i.tag).filter((t): t is string => !!t))].sort(),
    [items],
  );

  // keyboard
  const navIds = useMemo(
    () => [...lead, ...plain, ...(completedOpen ? done : [])].map((i) => i.id),
    [lead, plain, done, completedOpen],
  );
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      const s = useStore.getState();
      if (e.ctrlKey && (e.key === "n" || e.key === "N")) {
        e.preventDefault();
        s.openDraft();
        return;
      }
      if (isTyping(e.target) || e.altKey || e.ctrlKey || e.metaKey) return;
      const onButton = (e.target as HTMLElement | null)?.closest("button, a, [role=checkbox]");
      const idx = s.focusedId ? navIds.indexOf(s.focusedId) : -1;
      const focused = s.items.find((i) => i.id === s.focusedId);

      switch (e.key) {
        case "ArrowDown":
        case "ArrowUp": {
          if (navIds.length === 0) return;
          e.preventDefault();
          const next =
            idx === -1
              ? e.key === "ArrowDown" ? 0 : navIds.length - 1
              : Math.min(navIds.length - 1, Math.max(0, idx + (e.key === "ArrowDown" ? 1 : -1)));
          s.focus(navIds[next]);
          break;
        }
        case "Enter":
          if (onButton || !focused) return;
          e.preventDefault();
          s.expand(s.expandedId === focused.id ? null : focused.id);
          break;
        case "e":
        case "E":
          if (!focused || focused.status === "done") return;
          e.preventDefault();
          s.expand(focused.id, true);
          break;
        case " ":
          if (onButton || !focused) return;
          e.preventDefault();
          if (focused.status === "done") void s.restore(focused.id);
          else s.completeAnimated(focused.id);
          break;
        case "s":
        case "S":
          if (!focused || focused.status === "done" || s.snoozeFor) return;
          e.preventDefault();
          s.openSnooze(focused.id);
          break;
        case "Delete":
          if (!focused) return;
          e.preventDefault();
          void s.remove(focused);
          break;
        case "Escape":
          if (s.expandedId) {
            e.preventDefault();
            s.expand(null);
          }
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navIds]);

  const showEmpty = loaded && open.length === 0 && !draft;
  const headline =
    dueToday > 0 ? `${dueToday} due today` : open.length > 0 ? `${open.length} open` : "Nothing due today";

  return (
    <div className="panel">
      <header className="header" data-tauri-drag-region>
        <div className="titles" data-tauri-drag-region>
          <h1>Styky</h1>
          <p className="tnum">{headline}</p>
        </div>
        <div className="actions">
          <button
            className="icon-btn"
            aria-label="Add reminder (Ctrl+N)"
            title="Add reminder (Ctrl+N)"
            onClick={() => useStore.getState().openDraft()}
          >
            <Plus size={16} strokeWidth={2} />
          </button>
          <div className="menu-anchor">
            <button
              className="icon-btn"
              aria-label="More"
              aria-haspopup="menu"
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((v) => !v)}
            >
              <Ellipsis size={16} strokeWidth={2} />
            </button>
            {menuOpen && <Menu onClose={() => setMenuOpen(false)} />}
          </div>
        </div>
      </header>

      <main className="content">
        <LayoutGroup>
          <AnimatePresence initial={false}>{draft && <DraftCard key="draft" tags={tags} />}</AnimatePresence>

          {showEmpty && (
            <div className={`empty${done.length > 0 ? " compact" : ""}`}>
              <div className="empty-icon" aria-hidden>
                <CheckCheck size={22} strokeWidth={1.75} />
              </div>
              <p className="empty-title">Nothing pressing.</p>
              <p className="empty-hint">
                {hotkey.map((k, i) => (
                  <span key={i}>{i > 0 && " "}<kbd>{k}</kbd></span>
                ))}{" "}
                to add.
              </p>
            </div>
          )}

          {lead.length > 0 && (
            <ul className="list">
              <AnimatePresence initial={false}>
                {lead.map((i) => (
                  <ItemRow key={i.id} item={i} />
                ))}
              </AnimatePresence>
            </ul>
          )}
          {plain.length > 0 && <PlainGroup items={plain} />}
          <CompletedSection items={done} />
        </LayoutGroup>
      </main>
      <SnoozeMenu />
      <Toast />
    </div>
  );
}

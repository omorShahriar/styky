import { useEffect, useRef, useState } from "react";
import { CheckCheck, Ellipsis, Plus, Power, Settings } from "lucide-react";
import { initTheme } from "../lib/theme";
import { api } from "../lib/api";
import "./Panel.css";

function Menu({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
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

  return (
    <div className="menu" role="menu" ref={ref}>
      <button className="menu-item" role="menuitem" disabled>
        <Settings size={14} strokeWidth={1.75} /> Settings…
      </button>
      <div className="menu-sep" />
      <button className="menu-item" role="menuitem" onClick={() => api.quit()}>
        <Power size={14} strokeWidth={1.75} /> Quit Styky
      </button>
    </div>
  );
}

export default function Panel() {
  const [menuOpen, setMenuOpen] = useState(false);
  const dueToday = 0;

  useEffect(() => {
    let off: (() => void) | undefined;
    let cancelled = false;
    initTheme().then((u) => (cancelled ? u() : (off = u)));
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  return (
    <div className="panel">
      <header className="header" data-tauri-drag-region>
        <div className="titles" data-tauri-drag-region>
          <h1>Styky</h1>
          <p className="tnum">{dueToday === 0 ? "Nothing due today" : `${dueToday} due today`}</p>
        </div>
        <div className="actions">
          <button className="icon-btn" aria-label="Add reminder" title="Add reminder" disabled>
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
        <div className="empty">
          <div className="empty-icon" aria-hidden>
            <CheckCheck size={22} strokeWidth={1.75} />
          </div>
          <p className="empty-title">Nothing pressing.</p>
          <p className="empty-hint">
            <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>N</kbd> to add.
          </p>
        </div>
      </main>
    </div>
  );
}

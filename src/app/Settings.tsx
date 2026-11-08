import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { motion } from "motion/react";
import { api } from "../lib/api";
import { applyThemeMode, initTheme } from "../lib/theme";
import { pressSpring } from "../lib/motion";
import { keyToAccelerator, prettyHotkey } from "../lib/hotkey";
import { ProCard } from "./ProCard";
import { ConnectAI } from "./ConnectAI";
import { ProPill } from "../components/ProPill";
import { usePro } from "../lib/pro";
import { readProTheme, THEME_KEYS } from "../lib/proTheme";
import "./Settings.css";

const THEMES = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];
const SNOOZES = [
  { id: "10m", label: "10 minutes" },
  { id: "1h", label: "1 hour" },
  { id: "evening", label: "This evening (18:00)" },
  { id: "tomorrow", label: "Tomorrow (09:00)" },
];
const NAG_MINUTES = [
  { id: "5", label: "5 minutes" },
  { id: "10", label: "10 minutes" },
  { id: "15", label: "15 minutes" },
  { id: "30", label: "30 minutes" },
  { id: "60", label: "1 hour" },
];
const CLEAR_DAYS = [
  { id: "0", label: "Never" },
  { id: "1", label: "After 1 day" },
  { id: "3", label: "After 3 days" },
  { id: "7", label: "After 7 days" },
  { id: "14", label: "After 14 days" },
  { id: "30", label: "After 30 days" },
];

function Group({ title, children, footer }: { title?: string; children: React.ReactNode; footer?: React.ReactNode }) {
  return (
    <section className="group">
      {title && <h2>{title}</h2>}
      <div className="card">{children}</div>
      {footer && <p className="footer">{footer}</p>}
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="row">
      <div className="row-label">
        <span>{label}</span>
        {hint && <small>{hint}</small>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

function Segmented({
  value,
  options,
  onChange,
  label,
}: {
  value: string;
  options: { id: string; label: string }[];
  onChange: (v: string) => void;
  label: string;
}) {
  return (
    <div className="seg" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.id}
          role="radio"
          aria-checked={value === o.id}
          className={value === o.id ? "on" : ""}
          onClick={() => onChange(o.id)}
        >
          {value === o.id && <motion.span layoutId={`seg-${label}`} className="seg-pill" transition={pressSpring} />}
          <span className="seg-text">{o.label}</span>
        </button>
      ))}
    </div>
  );
}

function Switch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`switch${checked ? " on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <motion.span className="knob" layout transition={pressSpring} />
    </button>
  );
}

function HotkeyRecorder({
  value,
  startupError,
  onSaved,
}: {
  value: string;
  startupError: string | null;
  onSaved: () => void;
}) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const btn = useRef<HTMLButtonElement>(null);

  const finish = useCallback(
    async (accel: string) => {
      setRecording(false);
      try {
        await api.setQuickaddHotkey(accel);
        setError(null);
        onSaved();
      } catch (e) {
        setError(String(e));
      }
    },
    [onSaved],
  );

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setRecording(false);
      return;
    }
    const r = keyToAccelerator(e.nativeEvent);
    if (r.kind === "partial") return;
    if (r.kind === "invalid") {
      setError(r.reason);
      setRecording(false);
      return;
    }
    void finish(r.accelerator);
  };

  const shown = error ?? startupError;
  return (
    <div className="hotkey">
      <button
        ref={btn}
        className={`hotkey-btn${recording ? " rec" : ""}${shown ? " bad" : ""}`}
        onClick={() => {
          setError(null);
          setRecording((r) => !r);
        }}
        onKeyDown={onKeyDown}
        onBlur={() => setRecording(false)}
        aria-label="Quick add hotkey"
      >
        {recording ? "Press a shortcut…" : prettyHotkey(value)}
      </button>
      {shown && (
        <p className="inline-error" role="alert">
          {shown}
        </p>
      )}
    </div>
  );
}

type UpdateState =
  | { s: "idle" }
  | { s: "checking" }
  | { s: "current" }
  | { s: "available"; update: Update }
  | { s: "installing" }
  | { s: "error"; msg: string };

export default function Settings() {
  const pro = usePro();
  const [accentNow, setAccentNow] = useState("#0a84ff");
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [autostart, setAutostart] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [store, setStore] = useState(false);
  const [hotkeyErr, setHotkeyErr] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [note, setNote] = useState("");
  const [upd, setUpd] = useState<UpdateState>({ s: "idle" });

  const load = useCallback(async () => {
    const [s, a, h, b] = await Promise.all([api.getSettings(), api.getAutostart(), api.quickaddHotkeyError(), api.autostartBlocked()]);
    setSettings(s);
    setAutostart(a);
    setBlocked(b);
    setHotkeyErr(h);
  }, []);

  useEffect(() => {
    let off: (() => void) | undefined;
    let dead = false;
    void initTheme().then((u) => (dead ? u() : (off = u)));
    void load();
    void api.getAccent().then(setAccentNow).catch(() => {});
    void getVersion().then(setVersion);
    void api.isStore().then(setStore);
    const ev = listen("settings-changed", () => void load());
    const pro = listen("open-pro-card", () => {
      const el = document.getElementById("pro-card");
      el?.scrollIntoView({ behavior: "smooth", block: "start" });
      el?.classList.remove("flash");
      void el?.offsetWidth;
      el?.classList.add("flash");
    });
    // The autostart toggle can also change in the tray; refresh when we regain focus.
    const onFocus = () => void load();
    window.addEventListener("focus", onFocus);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) void getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      dead = true;
      off?.();
      void ev.then((u) => u());
      void pro.then((u) => u());
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("keydown", onKey);
    };
  }, [load]);

  const theme = readProTheme(settings);

  const set = async (key: string, value: string) => {
    setSettings((s) => ({ ...s, [key]: value }));
    if (key === "theme") applyThemeMode(value);
    try {
      await api.setSetting(key, value);
    } catch (e) {
      setNote(`Could not save: ${String(e)}`);
    }
  };

  const flash = (msg: string) => {
    setNote(msg);
    window.setTimeout(() => setNote((n) => (n === msg ? "" : n)), 4000);
  };

  const doExport = async () => {
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await save({
        defaultPath: `styky-export-${stamp}.json`,
        filters: [{ name: "Styky export", extensions: ["json"] }],
      });
      if (!path) return;
      const n = await api.exportJson(path);
      flash(`Exported ${n} reminder${n === 1 ? "" : "s"}.`);
    } catch (e) {
      flash(`Export failed: ${String(e)}`);
    }
  };

  const doImport = async (mode: "merge" | "replace") => {
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
      flash(
        mode === "replace"
          ? `Imported ${r.added} reminder${r.added === 1 ? "" : "s"}.`
          : `Imported: ${r.added} new, ${r.updated} updated, ${r.skipped} unchanged.`,
      );
    } catch (e) {
      flash(`Import failed: ${String(e)}`);
    }
  };

  const checkUpdates = async () => {
    setUpd({ s: "checking" });
    try {
      const u = await check();
      setUpd(u ? { s: "available", update: u } : { s: "current" });
    } catch {
      // Placeholder endpoint, offline or a malformed manifest: never show a raw error.
      setUpd({
        s: "error",
        msg: "Couldn't check for updates. The update server isn't set up yet, or you're offline.",
      });
    }
  };

  const install = async () => {
    if (upd.s !== "available") return;
    const u = upd.update;
    setUpd({ s: "installing" });
    try {
      await u.downloadAndInstall();
      await relaunch();
    } catch {
      setUpd({ s: "error", msg: "The update couldn't be installed. Try again later." });
    }
  };

  const updText =
    upd.s === "checking"
      ? "Checking…"
      : upd.s === "current"
        ? "Styky is up to date."
        : upd.s === "available"
          ? `Version ${upd.update.version} is available.`
          : upd.s === "installing"
            ? "Downloading and installing…"
            : upd.s === "error"
              ? upd.msg
              : "";

  return (
    <div className="settings">
      <div className="scroll">
        <ProCard />

        <Group title="Appearance">
          <Row label="Theme">
            <Segmented
              label="Theme"
              value={settings.theme ?? "system"}
              options={THEMES}
              onChange={(v) => void set("theme", v)}
            />
          </Row>
        </Group>

        <Group
          title="Panel look"
          footer={
            pro ? (
              "Tint, transparency, density, accent and font size apply to the panel and quick-add bar."
            ) : (
              <>
                Customise the panel with Styky Pro <ProPill />
              </>
            )
          }
        >
          <Row label="Skin">
            <span className={pro ? "" : "locked"}>
              <Segmented
                label="Skin"
                value={theme.skin}
                options={[
                  { id: "none", label: "Glass" },
                  { id: "sticky", label: "Sticky note" },
                ]}
                onChange={(v) => void set("theme_skin", v)}
              />
            </span>
          </Row>
          <Row label="Tint" hint={theme.skin === "sticky" ? "The Sticky note skin has its own colour" : undefined}>
            <span className={`row-inline${pro && theme.skin !== "sticky" ? "" : " locked"}`}>
              <input
                type="color"
                className="swatch"
                aria-label="Panel tint"
                value={theme.tint || "#ffffff"}
                onChange={(e) => void set("theme_tint", e.target.value)}
              />
              {theme.tint && (
                <button className="btn" onClick={() => void set("theme_tint", "")}>
                  Reset
                </button>
              )}
            </span>
          </Row>
          <Row label="Transparency" hint={theme.opacity === null ? "Default" : `${Math.round(theme.opacity * 100)}% opaque`}>
            <span className={`row-inline${pro ? "" : " locked"}`}>
              <input
                type="range"
                min={20}
                max={100}
                step={5}
                aria-label="Panel opacity"
                value={Math.round((theme.opacity ?? 0.55) * 100)}
                onChange={(e) => void set("theme_opacity", String(Number(e.target.value) / 100))}
              />
              {theme.opacity !== null && (
                <button className="btn" onClick={() => void set("theme_opacity", "")}>
                  Reset
                </button>
              )}
            </span>
          </Row>
          <Row label="Density">
            <span className={pro ? "" : "locked"}>
              <Segmented
                label="Density"
                value={theme.density}
                options={[
                  { id: "comfortable", label: "Comfortable" },
                  { id: "compact", label: "Compact" },
                ]}
                onChange={(v) => void set("theme_density", v)}
              />
            </span>
          </Row>
          <Row label="Font size">
            <span className={pro ? "" : "locked"}>
              <Segmented
                label="Font size"
                value={theme.font}
                options={[
                  { id: "small", label: "Small" },
                  { id: "medium", label: "Medium" },
                  { id: "large", label: "Large" },
                ]}
                onChange={(v) => void set("theme_font", v)}
              />
            </span>
          </Row>
          <Row label="Accent colour" hint={theme.accent ? undefined : "Follows Windows"}>
            <span className={`row-inline${pro ? "" : " locked"}`}>
              <input
                type="color"
                className="swatch"
                aria-label="Accent colour"
                value={theme.accent || accentNow}
                onChange={(e) => void set("theme_accent", e.target.value)}
              />
              {theme.accent && (
                <button className="btn" onClick={() => void set("theme_accent", "")}>
                  Use Windows accent
                </button>
              )}
            </span>
          </Row>
          {pro && THEME_KEYS.some((k) => settings[k]) && (
            <Row label="Reset look">
              <button className="btn" onClick={() => void Promise.all(THEME_KEYS.map((k) => set(k, "")))}>
                Reset all
              </button>
            </Row>
          )}
        </Group>

        <Group title="General" footer="Press the hotkey anywhere in Windows to open the quick-add bar.">
          <Row label="Quick add hotkey">
            <HotkeyRecorder
              value={settings.quickadd_hotkey ?? "Ctrl+Alt+N"}
              startupError={hotkeyErr}
              onSaved={() => void load()}
            />
          </Row>
          <Row label="Launch at login" hint={blocked ? "Turned off in Task Manager > Startup apps. Turn it back on there." : undefined}>
            <Switch label="Launch at login" checked={autostart} onChange={async (v) => { setAutostart(await api.setAutostart(v)); setBlocked(await api.autostartBlocked()); }} />
          </Row>
        </Group>

        <Group title="Reminders">
          <Row label="Default snooze" hint="Preselected in the notification">
            <select
              value={settings.default_snooze ?? "10m"}
              onChange={(e) => void set("default_snooze", e.target.value)}
              aria-label="Default snooze"
            >
              {SNOOZES.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </Row>
          <Row label="Clear completed" hint="Removes done reminders automatically">
            <select
              value={settings.autoclear_days ?? "7"}
              onChange={(e) => void set("autoclear_days", e.target.value)}
              aria-label="Auto-clear completed"
            >
              {CLEAR_DAYS.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </Row>
        </Group>

        <ConnectAI />

        <Group
          title="Escalation"
          footer="Critical reminders (priority !!! or Critical in the editor) notify again until you mark them done or snooze them."
        >
          <Row label="Critical re-notify every" hint={pro ? undefined : "Styky Pro"}>
            <span className="row-inline">
              {!pro && <ProPill />}
              <select
                value={settings.escalation_minutes ?? "10"}
                disabled={!pro}
                onChange={(e) => void set("escalation_minutes", e.target.value)}
                aria-label="Critical re-notify interval"
              >
                {NAG_MINUTES.map((o) => (
                  <option key={o.id} value={o.id}>
                    {o.label}
                  </option>
                ))}
              </select>
            </span>
          </Row>
          <Row label="Bring the panel forward" hint="Raise Styky with each re-notification">
            <span className="row-inline">
              {!pro && <ProPill />}
              <span className={pro ? "" : "locked"}>
                <Switch
                  label="Bring the panel forward on re-notify"
                  checked={pro && settings.escalation_bring_forward === "1"}
                  onChange={(v) => pro && void set("escalation_bring_forward", v ? "1" : "0")}
                />
              </span>
            </span>
          </Row>
        </Group>

        <Group title="Data" footer={note || "Exports are plain JSON. Replace overwrites everything."}>
          <Row label="Export">
            <button className="btn" onClick={() => void doExport()}>
              Export…
            </button>
          </Row>
          <Row label="Import">
            <button className="btn" onClick={() => void doImport("merge")}>
              Merge…
            </button>
          </Row>
          <Row label="Restore from backup">
            <button className="btn" onClick={() => void doImport("replace")}>
              Replace…
            </button>
          </Row>
        </Group>

        <Group title="About" footer={store ? undefined : updText}>
          <Row label="Styky">
            <span className="muted tnum">Version {version}</span>
          </Row>
          {!store && <Row label="Updates">
            {upd.s === "available" ? (
              <button className="btn primary" onClick={() => void install()}>
                Install and restart
              </button>
            ) : (
              <button
                className="btn"
                disabled={upd.s === "checking" || upd.s === "installing"}
                onClick={() => void checkUpdates()}
              >
                Check for updates
              </button>
            )}
          </Row>}
        </Group>
      </div>
    </div>
  );
}

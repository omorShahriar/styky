import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyProTheme, hasAccentOverride } from "./proTheme";

function applyAccent(hex: string) {
  // A Pro accent override wins over the Windows accent.
  if (hasAccentOverride()) return;
  document.documentElement.style.setProperty("--accent", hex);
}

/** "system" removes the override so prefers-color-scheme decides; "light"/"dark" force it. */
export function applyThemeMode(mode: string | undefined) {
  const el = document.documentElement;
  if (mode === "light" || mode === "dark") el.dataset.theme = mode;
  else delete el.dataset.theme;
}

let windowsAccent = "";

/** Read the saved settings (theme mode + Pro theme) and apply them. */
async function refresh() {
  try {
    const [s, status] = await Promise.all([
      invoke<Record<string, string>>("get_settings"),
      invoke<{ pro: boolean }>("license_status"),
    ]);
    applyThemeMode(s.theme);
    applyProTheme(s, status.pro);
    // Clearing an accent override must bring the Windows accent back.
    if (!hasAccentOverride() && windowsAccent) document.documentElement.style.setProperty("--accent", windowsAccent);
  } catch {
    /* keep following the system */
  }
}

/** Read the Windows accent colour into --accent, apply the Theme settings, keep both live. */
export async function initTheme(): Promise<() => void> {
  try {
    windowsAccent = await invoke<string>("get_accent");
    applyAccent(windowsAccent);
  } catch {
    /* keep the CSS fallback */
  }
  void refresh();
  const offs = await Promise.all([
    listen<string>("accent-changed", (e) => {
      windowsAccent = e.payload;
      applyAccent(e.payload);
    }),
    listen("settings-changed", () => void refresh()),
    listen("pro-changed", () => void refresh()),
  ]);
  // Light/dark can flip live; the default tint the opacity slider applies to depends on it.
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const onScheme = () => void refresh();
  mq.addEventListener("change", onScheme);
  return () => {
    offs.forEach((u) => u());
    mq.removeEventListener("change", onScheme);
  };
}

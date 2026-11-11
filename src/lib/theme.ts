import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { retryDelay } from "./retry";
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
let refreshAttempts = 0;
let refreshTimer: ReturnType<typeof setTimeout> | undefined;
const MAX_REFRESH_RETRIES = 8;

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
    refreshAttempts = 0;
  } catch {
    // The backend may still be starting up: try again shortly, then give up and keep following the system.
    if (refreshAttempts < MAX_REFRESH_RETRIES) {
      refreshAttempts += 1;
      clearTimeout(refreshTimer);
      refreshTimer = setTimeout(() => void refresh(), retryDelay(refreshAttempts));
    }
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

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function applyAccent(hex: string) {
  document.documentElement.style.setProperty("--accent", hex);
}

/** "system" removes the override so prefers-color-scheme decides; "light"/"dark" force it. */
export function applyThemeMode(mode: string | undefined) {
  const el = document.documentElement;
  if (mode === "light" || mode === "dark") el.dataset.theme = mode;
  else delete el.dataset.theme;
}

async function loadThemeMode() {
  try {
    const s = await invoke<Record<string, string>>("get_settings");
    applyThemeMode(s.theme);
  } catch {
    /* keep following the system */
  }
}

/** Read the Windows accent colour into --accent, apply the Theme setting, keep both live. */
export async function initTheme(): Promise<() => void> {
  try {
    applyAccent(await invoke<string>("get_accent"));
  } catch {
    /* keep the CSS fallback */
  }
  void loadThemeMode();
  const offs = await Promise.all([
    listen<string>("accent-changed", (e) => applyAccent(e.payload)),
    listen("settings-changed", () => void loadThemeMode()),
  ]);
  return () => offs.forEach((u) => u());
}

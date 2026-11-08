import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function applyAccent(hex: string) {
  document.documentElement.style.setProperty("--accent", hex);
}

/** Read the Windows accent colour into --accent and keep it live. */
export async function initTheme(): Promise<() => void> {
  try {
    applyAccent(await invoke<string>("get_accent"));
  } catch {
    /* keep the CSS fallback */
  }
  return listen<string>("accent-changed", (e) => applyAccent(e.payload));
}

import { invoke } from "@tauri-apps/api/core";

export const api = {
  hidePanel: () => invoke<void>("hide_panel"),
  quit: () => invoke<void>("quit"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
};

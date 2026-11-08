import type { LogicalSize } from "./tauri-dpi";
import { emit } from "./events";

const win = {
  label: "main",
  /** The real quick-add bar resizes its OS window; in the demo the host listens and resizes the frame. */
  setSize: async (s: LogicalSize) => emit("demo-window-size", { width: s.width, height: s.height }),
};

export const getCurrentWindow = () => win;

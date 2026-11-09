import { openUrl } from "@tauri-apps/plugin-opener";
import { PRO_URL } from "../lib/pro";

/** Small "Pro" marker on a locked control; links to the /pro page. No dialogs. */
export function ProPill({ title = "Part of Styky Pro" }: { title?: string }) {
  return (
    <button
      type="button"
      className="pro-pill"
      title={title}
      onClick={(e) => {
        e.stopPropagation();
        void openUrl(PRO_URL).catch(() => {});
      }}
    >
      Pro
    </button>
  );
}

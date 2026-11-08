import { api } from "../lib/api";

/** Small "Pro" marker on a locked control; opens Settings at the Styky Pro card. No dialogs. */
export function ProPill({ title = "Part of Styky Pro" }: { title?: string }) {
  return (
    <button
      type="button"
      className="pro-pill"
      title={title}
      onClick={(e) => {
        e.stopPropagation();
        void api.openSettingsPro();
      }}
    >
      Pro
    </button>
  );
}

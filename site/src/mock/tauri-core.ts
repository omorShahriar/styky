import { db } from "./db";
import { siteTheme } from "./site-theme";

/**
 * `invoke` for the few callers that bypass `lib/api` (lib/theme.ts reads settings and the accent).
 * get_accent rejects on purpose: theme.ts then keeps the site's own CSS accent.
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case "get_settings":
      return { ...db.settings(), theme: siteTheme() } as T;
    case "set_setting":
      db.setSetting(String(args?.key), String(args?.value));
      return undefined as T;
    default:
      throw new Error(`"${cmd}" is not available in the web demo`);
  }
}

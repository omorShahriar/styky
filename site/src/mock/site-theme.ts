/**
 * The app's lib/theme.ts applies the "theme" setting to <html data-theme>. On the site the theme is owned
 * by the page (toggle in the nav, system preference at first paint), so the mocked setting mirrors it.
 */
import { emit } from "./events";

let current: "light" | "dark" =
  typeof document !== "undefined" && document.documentElement.dataset.theme === "dark" ? "dark" : "light";

export const siteTheme = () => current;

export function setSiteTheme(t: "light" | "dark") {
  current = t;
  document.documentElement.dataset.theme = t;
  emit("settings-changed");
}

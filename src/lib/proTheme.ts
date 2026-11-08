/**
 * Pro themes: panel tint, transparency, density, accent override, "Sticky" paper skin and font size.
 * Everything is applied through CSS custom properties / data attributes on <html> (see styles/tokens.css),
 * never by editing components. Without Pro nothing is applied, so a lapsed license simply looks default.
 */

export type Density = "comfortable" | "compact";
export type Skin = "none" | "sticky";
export type FontSize = "small" | "medium" | "large";

export interface ProThemeSettings {
  /** #rrggbb or "" (default tint of the current light/dark mode) */
  tint: string;
  /** 0.2 .. 1, or null for the theme default */
  opacity: number | null;
  density: Density;
  /** #rrggbb or "" (follow Windows) */
  accent: string;
  skin: Skin;
  font: FontSize;
}

export const THEME_KEYS = ["theme_tint", "theme_opacity", "theme_density", "theme_accent", "theme_skin", "theme_font"] as const;

export const DEFAULT_PRO_THEME: ProThemeSettings = { tint: "", opacity: null, density: "comfortable", accent: "", skin: "none", font: "medium" };

const HEX = /^#[0-9a-f]{6}$/i;

export function readProTheme(s: Record<string, string>): ProThemeSettings {
  const op = Number(s.theme_opacity);
  return {
    tint: HEX.test(s.theme_tint ?? "") ? s.theme_tint : "",
    opacity: s.theme_opacity && Number.isFinite(op) ? Math.min(1, Math.max(0.2, op)) : null,
    density: s.theme_density === "compact" ? "compact" : "comfortable",
    accent: HEX.test(s.theme_accent ?? "") ? s.theme_accent : "",
    skin: s.theme_skin === "sticky" ? "sticky" : "none",
    font: s.theme_font === "small" || s.theme_font === "large" ? s.theme_font : "medium",
  };
}

const rgb = (hex: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];

/** White or black text, whichever reads better on `hex` (WCAG relative luminance). */
export function onColor(hex: string): string {
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const [r, g, b] = rgb(hex).map(lin);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? "#000" : "#fff";
}

const FONT: Record<FontSize, { title: string; meta: string; lh: string }> = {
  small: { title: "14px", meta: "11px", lh: "19px" },
  medium: { title: "15px", meta: "12px", lh: "20px" },
  large: { title: "17px", meta: "13px", lh: "23px" },
};

export interface ThemeOutput {
  /** CSS custom properties to set on <html> (all others in `ALL_VARS` are cleared). */
  vars: Record<string, string>;
  skin: Skin;
  density: Density;
}

export const ALL_VARS = ["--panel-tint", "--paper-alpha", "--accent", "--on-accent", "--row-pad-y", "--fs-title", "--fs-meta", "--lh-title"];

/** Pure: the tokens for these settings. `dark` picks the default tint the opacity applies to. */
export function themeVars(t: ProThemeSettings, pro: boolean, dark: boolean): ThemeOutput {
  if (!pro) return { vars: {}, skin: "none", density: "comfortable" };
  const vars: Record<string, string> = {};
  if (t.skin === "sticky") {
    // The paper colour comes from CSS; only the transparency slider reaches it.
    if (t.opacity !== null) vars["--paper-alpha"] = String(t.opacity);
  } else if (t.tint || t.opacity !== null) {
    const [r, g, b] = t.tint ? rgb(t.tint) : dark ? [30, 30, 32] : [255, 255, 255];
    const a = t.opacity ?? (dark ? 0.5 : 0.55);
    vars["--panel-tint"] = `rgba(${r}, ${g}, ${b}, ${a})`;
  }
  if (t.accent) {
    vars["--accent"] = t.accent;
    vars["--on-accent"] = onColor(t.accent);
  }
  if (t.density === "compact") vars["--row-pad-y"] = "6px";
  if (t.font !== "medium") {
    vars["--fs-title"] = FONT[t.font].title;
    vars["--fs-meta"] = FONT[t.font].meta;
    vars["--lh-title"] = FONT[t.font].lh;
  }
  return { vars, skin: t.skin, density: t.density };
}

/** Whether Windows/our Theme setting currently renders dark. */
function isDark(): boolean {
  const el = document.documentElement.dataset.theme;
  if (el === "dark") return true;
  if (el === "light") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** The accent the user chose (so the Windows accent event does not overwrite it). */
let accentOverride = "";
export const hasAccentOverride = (): boolean => accentOverride !== "";

export function applyProTheme(settings: Record<string, string>, pro: boolean): void {
  const el = document.documentElement;
  const out = themeVars(readProTheme(settings), pro, isDark());
  for (const v of ALL_VARS) el.style.removeProperty(v);
  for (const [k, v] of Object.entries(out.vars)) el.style.setProperty(k, v);
  accentOverride = out.vars["--accent"] ?? "";
  if (out.skin === "sticky") el.dataset.skin = "sticky";
  else delete el.dataset.skin;
  if (out.density === "compact") el.dataset.density = "compact";
  else delete el.dataset.density;
}

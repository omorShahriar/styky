import { describe, expect, it } from "vitest";
import { DEFAULT_PRO_THEME, onColor, readProTheme, themeVars } from "./proTheme";

const t = (p: Partial<typeof DEFAULT_PRO_THEME>) => ({ ...DEFAULT_PRO_THEME, ...p });

describe("pro theme tokens", () => {
  it("applies nothing without Pro, whatever is stored", () => {
    const out = themeVars(t({ tint: "#ff0000", opacity: 0.4, accent: "#00ff00", skin: "sticky", density: "compact", font: "large" }), false, false);
    expect(out).toEqual({ vars: {}, skin: "none", density: "comfortable" });
  });

  it("defaults leave the stylesheet alone", () => {
    expect(themeVars(DEFAULT_PRO_THEME, true, false).vars).toEqual({});
  });

  it("tint + transparency become one rgba token", () => {
    expect(themeVars(t({ tint: "#102030", opacity: 0.8 }), true, false).vars["--panel-tint"]).toBe("rgba(16, 32, 48, 0.8)");
    // Transparency alone keeps the mode's own tint colour.
    expect(themeVars(t({ opacity: 0.3 }), true, false).vars["--panel-tint"]).toBe("rgba(255, 255, 255, 0.3)");
    expect(themeVars(t({ opacity: 0.3 }), true, true).vars["--panel-tint"]).toBe("rgba(30, 30, 32, 0.3)");
    // A tint with no slider uses the default alpha of the mode.
    expect(themeVars(t({ tint: "#ff0000" }), true, true).vars["--panel-tint"]).toBe("rgba(255, 0, 0, 0.5)");
  });

  it("the sticky skin owns the panel colour; only transparency reaches it", () => {
    const o = themeVars(t({ skin: "sticky", tint: "#ff0000", opacity: 0.7 }), true, false);
    expect(o.skin).toBe("sticky");
    expect(o.vars["--panel-tint"]).toBeUndefined();
    expect(o.vars["--paper-alpha"]).toBe("0.7");
  });

  it("accent override carries a readable on-accent colour", () => {
    const light = themeVars(t({ accent: "#ffeb3b" }), true, false).vars;
    expect([light["--accent"], light["--on-accent"]]).toEqual(["#ffeb3b", "#000"]);
    expect(themeVars(t({ accent: "#1a237e" }), true, false).vars["--on-accent"]).toBe("#fff");
    expect(onColor("#ffffff")).toBe("#000");
    expect(onColor("#000000")).toBe("#fff");
  });

  it("density and font size map to row padding and type tokens", () => {
    const v = themeVars(t({ density: "compact", font: "large" }), true, false);
    expect(v.density).toBe("compact");
    expect(v.vars).toMatchObject({ "--row-pad-y": "6px", "--fs-title": "17px", "--fs-meta": "13px" });
    expect(themeVars(t({ font: "small" }), true, false).vars["--fs-title"]).toBe("14px");
  });

  it("reads stored settings defensively", () => {
    expect(readProTheme({})).toEqual(DEFAULT_PRO_THEME);
    expect(readProTheme({ theme_tint: "red", theme_accent: "#12345", theme_opacity: "abc", theme_font: "huge", theme_skin: "x", theme_density: "x" })).toEqual(DEFAULT_PRO_THEME);
    expect(readProTheme({ theme_opacity: "5" }).opacity).toBe(1);
    expect(readProTheme({ theme_opacity: "0.01" }).opacity).toBe(0.2);
    expect(readProTheme({ theme_tint: "#AABBCC", theme_skin: "sticky", theme_density: "compact", theme_font: "small" })).toMatchObject({
      tint: "#AABBCC",
      skin: "sticky",
      density: "compact",
      font: "small",
    });
  });
});

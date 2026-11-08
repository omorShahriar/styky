import { describe, expect, it } from "vitest";
import { keyToAccelerator, prettyHotkey } from "./hotkey";

const ev = (
  code: string,
  m: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {},
) => ({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...m });

describe("keyToAccelerator", () => {
  it("builds Ctrl+Alt+N", () => {
    expect(keyToAccelerator(ev("KeyN", { ctrlKey: true, altKey: true }))).toEqual({
      kind: "ok",
      accelerator: "Ctrl+Alt+N",
    });
  });
  it("ignores bare modifiers", () => {
    expect(keyToAccelerator(ev("ControlLeft", { ctrlKey: true }))).toEqual({ kind: "partial" });
  });
  it("rejects a bare letter but allows a bare F-key", () => {
    expect(keyToAccelerator(ev("KeyN")).kind).toBe("invalid");
    expect(keyToAccelerator(ev("F9"))).toEqual({ kind: "ok", accelerator: "F9" });
  });
  it("maps arrows, digits and Win", () => {
    expect(keyToAccelerator(ev("ArrowUp", { metaKey: true, shiftKey: true }))).toEqual({
      kind: "ok",
      accelerator: "Shift+Super+Up",
    });
    expect(keyToAccelerator(ev("Digit5", { ctrlKey: true }))).toEqual({ kind: "ok", accelerator: "Ctrl+5" });
  });
  it("prettifies", () => {
    expect(prettyHotkey("Ctrl+Super+K")).toBe("Ctrl + Win + K");
  });
});

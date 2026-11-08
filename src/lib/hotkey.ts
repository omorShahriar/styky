/** Turns a keydown into a global-shortcut accelerator such as "Ctrl+Alt+N". */

export type Recorded =
  | { kind: "partial" } // only modifiers so far
  | { kind: "invalid"; reason: string }
  | { kind: "ok"; accelerator: string };

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Minus: "-",
  Equal: "=",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
};

export function codeToKey(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return NAMED[code] ?? null;
}

export function keyToAccelerator(
  e: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">,
): Recorded {
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)$/.test(e.code)) return { kind: "partial" };
  const key = codeToKey(e.code);
  if (!key) return { kind: "invalid", reason: "That key can't be used in a shortcut." };
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(
    Boolean,
  ) as string[];
  const isF = /^F\d+$/.test(key);
  if (mods.length === 0 && !isF) {
    return { kind: "invalid", reason: "Include Ctrl, Alt, Shift or Win so it doesn't clash with typing." };
  }
  return { kind: "ok", accelerator: [...mods, key].join("+") };
}

export function prettyHotkey(accel: string): string {
  return accel.split("+").map((p) => (p === "Super" ? "Win" : p)).join(" + ");
}

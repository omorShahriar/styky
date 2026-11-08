import { describe, expect, it } from "vitest";
import { isTaskChecked, toggleTaskLine } from "./markdown";

const doc = [
  "# Release",
  "",
  "- [ ] tag build",
  "- [x] notes",
  "  - [ ] nested",
  "1. [ ] numbered",
  "plain [ ] text",
].join("\n");

describe("toggleTaskLine", () => {
  it("ticks an unticked item and touches only that line", () => {
    const out = toggleTaskLine(doc, 3);
    expect(out.split("\n")[2]).toBe("- [x] tag build");
    expect(out.split("\n").filter((_, i) => i !== 2)).toEqual(
      doc.split("\n").filter((_, i) => i !== 2),
    );
  });
  it("unticks a ticked item (also capital X)", () => {
    expect(toggleTaskLine(doc, 4).split("\n")[3]).toBe("- [ ] notes");
    expect(toggleTaskLine("- [X] a", 1)).toBe("- [ ] a");
  });
  it("handles nested and numbered items", () => {
    expect(toggleTaskLine(doc, 5).split("\n")[4]).toBe("  - [x] nested");
    expect(toggleTaskLine(doc, 6).split("\n")[5]).toBe("1. [x] numbered");
  });
  it("ignores non-task lines and out-of-range lines", () => {
    expect(toggleTaskLine(doc, 1)).toBe(doc);
    expect(toggleTaskLine(doc, 7)).toBe(doc);
    expect(toggleTaskLine(doc, 99)).toBe(doc);
  });
  it("keeps CRLF line endings", () => {
    expect(toggleTaskLine("- [ ] a\r\n- [ ] b\r\n", 2)).toBe("- [ ] a\r\n- [x] b\r\n");
  });
  it("round-trips", () => {
    expect(toggleTaskLine(toggleTaskLine(doc, 3), 3)).toBe(doc);
  });
});

describe("isTaskChecked", () => {
  it("reads the state", () => {
    expect(isTaskChecked(doc, 3)).toBe(false);
    expect(isTaskChecked(doc, 4)).toBe(true);
    expect(isTaskChecked(doc, 1)).toBeNull();
  });
});

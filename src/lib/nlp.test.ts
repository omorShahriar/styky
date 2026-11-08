import { describe, expect, it } from "vitest";
import { parseQuick } from "./nlp";

// Tuesday 2026-03-10 12:00 local.
const now = new Date(2026, 2, 10, 12, 0, 0);
const at = (y: number, m: number, d: number, h = 9, mi = 0) => new Date(y, m - 1, d, h, mi).getTime();

describe("parseQuick", () => {
  it("Release v2.3 tomorrow 5pm #api !!", () => {
    const p = parseQuick("Release v2.3 tomorrow 5pm #api !!", now);
    expect(p.title).toBe("Release v2.3");
    expect(p.dueAt).toBe(at(2026, 3, 11, 17));
    expect(p.tag).toBe("api");
    expect(p.priority).toBe(2);
    expect(p.repeat).toBe("none");
    expect(p.chips.map((c) => c.kind)).toEqual(["date", "tag", "priority"]);
  });

  it("Rotate keys fri 10am #infra every month", () => {
    const p = parseQuick("Rotate keys fri 10am #infra every month", now);
    expect(p.title).toBe("Rotate keys");
    expect(p.dueAt).toBe(at(2026, 3, 13, 10)); // the coming Friday
    expect(p.tag).toBe("infra");
    expect(p.repeat).toBe("monthly");
    expect(p.priority).toBe(1);
  });

  it("Merge @release/2.3 in 2h", () => {
    const p = parseQuick("Merge @release/2.3 in 2h", now);
    expect(p.title).toBe("Merge");
    expect(p.branch).toBe("release/2.3");
    expect(p.dueAt).toBe(now.getTime() + 2 * 3_600_000);
  });

  it("single ! is an explicit normal priority, !! urgent, !low low", () => {
    expect(parseQuick("a !", now)).toMatchObject({ priority: 1, title: "a" });
    expect(parseQuick("a !", now).chips.map((c) => c.label)).toEqual(["Normal"]);
    expect(parseQuick("a !!", now).priority).toBe(2);
    expect(parseQuick("a !low", now).priority).toBe(0);
    expect(parseQuick("plain title", now).chips).toEqual([]);
  });

  it("urls go into link and out of the title", () => {
    const p = parseQuick("Review https://github.com/o/r/pull/12 today 3pm", now);
    expect(p.link).toBe("https://github.com/o/r/pull/12");
    expect(p.title).toBe("Review");
    expect(p.dueAt).toBe(at(2026, 3, 10, 15));
  });

  it("a date without a time defaults to 09:00", () => {
    expect(parseQuick("Call Sam tomorrow", now).dueAt).toBe(at(2026, 3, 11, 9));
  });

  it("a repeat without a date starts at the next 09:00", () => {
    const p = parseQuick("Standup notes every weekday", now);
    expect(p.repeat).toBe("weekdays");
    expect(p.dueAt).toBe(at(2026, 3, 11, 9));
    // Friday afternoon -> Monday morning
    const fri = new Date(2026, 2, 13, 15, 0);
    expect(parseQuick("Standup notes every weekday", fri).dueAt).toBe(at(2026, 3, 16, 9));
  });

  it("does not treat e-mail addresses or mid-word # as tokens", () => {
    const p = parseQuick("Mail bob@example.com about C#", now);
    expect(p.branch).toBeNull();
    expect(p.tag).toBeNull();
    expect(p.title).toBe("Mail bob@example.com about C#");
  });

  it("keeps a title when only a date is typed, and trims dangling connectives", () => {
    expect(parseQuick("tomorrow 5pm", now).title).toBe("tomorrow 5pm");
    expect(parseQuick("Pay rent by tomorrow", now).title).toBe("Pay rent");
  });

  it("empty input parses to nothing", () => {
    expect(parseQuick("   ", now)).toMatchObject({ title: "", dueAt: null, chips: [] });
  });

  it("!!! is critical (priority 3)", () => {
    const p = parseQuick("Deploy hotfix !!!", now);
    expect(p.priority).toBe(3);
    expect(p.title).toBe("Deploy hotfix");
    expect(p.chips.find((c) => c.kind === "priority")?.label).toBe("Critical");
  });
});

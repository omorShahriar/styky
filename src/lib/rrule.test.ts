import { describe, expect, it } from "vitest";
import { describeRepeat, firstMatch, formatRule, isSimpleRepeat, parseRule, storeRule } from "./rrule";
import { extractRepeat } from "./nlpRepeat";
import { parseQuick } from "./nlp";

// Wed 2026-03-11 10:00 local.
const now = new Date(2026, 2, 11, 10, 0, 0, 0);
const at = (m: number, d: number, h = 9) => new Date(2026, m - 1, d, h, 0, 0, 0).getTime();
const rep = (s: string) => extractRepeat(` ${s} `, now);
const rr = (s: string) => rep(s).repeat;

describe("rrule parse / format", () => {
  it("round-trips the canonical form (same strings as repeat.rs)", () => {
    for (const s of [
      "FREQ=DAILY",
      "FREQ=DAILY;INTERVAL=3",
      "FREQ=WEEKLY;BYDAY=MO,WE,FR",
      "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU;COUNT=5",
      "FREQ=MONTHLY;BYDAY=-1FR",
      "FREQ=MONTHLY;INTERVAL=2;BYDAY=2TH;UNTIL=20271231",
      "FREQ=MONTHLY;BYMONTHDAY=-1",
      "FREQ=MONTHLY;BYMONTHDAY=15;COUNT=12",
    ]) {
      expect(formatRule(parseRule(s)!)).toBe(s);
    }
    expect(formatRule(parseRule("freq=weekly;byday=FR,mo,MO;interval=1")!)).toBe("FREQ=WEEKLY;BYDAY=MO,FR");
  });

  it("rejects invalid rules", () => {
    for (const s of [
      "FREQ=YEARLY",
      "INTERVAL=2",
      "FREQ=DAILY;BYDAY=MO",
      "FREQ=WEEKLY;BYDAY=2MO",
      "FREQ=MONTHLY;BYDAY=MO",
      "FREQ=MONTHLY;BYDAY=2MO,3MO",
      "FREQ=MONTHLY;BYDAY=2MO;BYMONTHDAY=3",
      "FREQ=MONTHLY;BYDAY=6MO",
      "FREQ=DAILY;INTERVAL=0",
      "FREQ=DAILY;COUNT=0",
      "FREQ=MONTHLY;BYMONTHDAY=32",
      "FREQ=DAILY;UNTIL=soon",
      "FREQ=DAILY;X=1",
      "banana",
    ]) {
      expect(parseRule(s), s).toBeNull();
    }
    expect(isSimpleRepeat("weekly")).toBe(true);
    expect(isSimpleRepeat("FREQ=DAILY")).toBe(false);
  });

  it("stores simple rules as the free legacy values", () => {
    expect(storeRule(parseRule("FREQ=DAILY")!)).toBe("daily");
    expect(storeRule(parseRule("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR")!)).toBe("weekdays");
    expect(storeRule(parseRule("FREQ=MONTHLY")!)).toBe("monthly");
    expect(storeRule(parseRule("FREQ=WEEKLY;INTERVAL=2")!)).toBe("FREQ=WEEKLY;INTERVAL=2");
    expect(storeRule(parseRule("FREQ=DAILY;COUNT=3")!)).toBe("FREQ=DAILY;COUNT=3");
  });

  it("describes rules in plain words", () => {
    expect(describeRepeat("daily")).toBe("Every day");
    expect(describeRepeat("weekdays")).toBe("Every weekday");
    expect(describeRepeat("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE")).toBe("Every 2 weeks on Mon, Wed");
    expect(describeRepeat("FREQ=MONTHLY;BYDAY=-1FR")).toBe("Last Friday of the month");
    expect(describeRepeat("FREQ=MONTHLY;INTERVAL=2;BYDAY=2TU")).toBe("Every 2 months on the second Tuesday");
    expect(describeRepeat("FREQ=MONTHLY;BYMONTHDAY=-1")).toBe("Last day of the month");
    expect(describeRepeat("FREQ=MONTHLY;BYMONTHDAY=22")).toBe("Every month on the 22nd");
    expect(describeRepeat("FREQ=DAILY;COUNT=3")).toBe("Every day · 3 times left");
    expect(describeRepeat("FREQ=DAILY;COUNT=1")).toBe("Every day · 1 time left");
    expect(describeRepeat("none")).toBe("");
  });
});

describe("firstMatch", () => {
  it("finds the first listed day for rules without a date (same cases as repeat.rs)", () => {
    expect(firstMatch("FREQ=WEEKLY;BYDAY=MO", now)).toBe(at(3, 16));
    expect(firstMatch("FREQ=WEEKLY;BYDAY=MO,FR", now)).toBe(at(3, 13));
    expect(firstMatch("FREQ=WEEKLY;BYDAY=WE", now)).toBe(at(3, 18));
    expect(firstMatch("FREQ=WEEKLY;BYDAY=WE", new Date(2026, 2, 11, 8, 0))).toBe(at(3, 11));
    expect(firstMatch("FREQ=MONTHLY;BYDAY=-1FR", now)).toBe(at(3, 27));
    expect(firstMatch("FREQ=MONTHLY;BYMONTHDAY=-1", now)).toBe(at(3, 31));
    expect(firstMatch("FREQ=DAILY;INTERVAL=2", now)).toBeNull();
    expect(firstMatch("weekly", now)).toBeNull();
  });
});

describe("repeat phrases (same cases as nlp_repeat.rs)", () => {
  it("keeps the free phrases as legacy values", () => {
    expect(rr("water every day")).toBe("daily");
    expect(rr("standup every weekday")).toBe("weekdays");
    expect(rr("standup every weekdays")).toBe("weekdays");
    expect(rr("review every week")).toBe("weekly");
    expect(rr("rent every month")).toBe("monthly");
    expect(rr("x every 1 week")).toBe("weekly");
    expect(rr("plain title")).toBeNull();
  });

  it("every N units", () => {
    expect(rr("sync every 2 weeks")).toBe("FREQ=WEEKLY;INTERVAL=2");
    expect(rr("x every 3 days")).toBe("FREQ=DAILY;INTERVAL=3");
    expect(rr("x every 6 months")).toBe("FREQ=MONTHLY;INTERVAL=6");
    expect(rr("x every other week")).toBe("FREQ=WEEKLY;INTERVAL=2");
    expect(rr("x every other day")).toBe("FREQ=DAILY;INTERVAL=2");
    expect(rep("sync every 2 weeks").rest.trim()).toBe("sync");
  });

  it("chosen weekdays", () => {
    expect(rr("gym every mon,wed")).toBe("FREQ=WEEKLY;BYDAY=MO,WE");
    expect(rr("gym every mon, wed and fri")).toBe("FREQ=WEEKLY;BYDAY=MO,WE,FR");
    expect(rr("gym every monday and friday")).toBe("FREQ=WEEKLY;BYDAY=MO,FR");
    expect(rr("gym every mon wed")).toBe("FREQ=WEEKLY;BYDAY=MO,WE");
    expect(rr("gym every friday")).toBe("FREQ=WEEKLY;BYDAY=FR");
    expect(rr("gym every sat-sun")).toBe("FREQ=WEEKLY;BYDAY=SA,SU");
    expect(rr("gym every mon-fri")).toBe("weekdays");
    expect(rr("gym every 2 weeks on mon,thu")).toBe("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH");
    expect(rr("rent every month")).toBe("monthly");
  });

  it("nth and last weekday of the month", () => {
    expect(rr("report last friday of the month")).toBe("FREQ=MONTHLY;BYDAY=-1FR");
    expect(rr("x first monday of every month")).toBe("FREQ=MONTHLY;BYDAY=1MO");
    expect(rr("x 2nd tuesday of the month")).toBe("FREQ=MONTHLY;BYDAY=2TU");
    expect(rr("x every month on the third thursday")).toBe("FREQ=MONTHLY;BYDAY=3TH");
    expect(rr("x last day of the month")).toBe("FREQ=MONTHLY;BYMONTHDAY=-1");
    expect(rr("x every month on the 15th")).toBe("FREQ=MONTHLY;BYMONTHDAY=15");
  });

  it("end after N times or on a date", () => {
    expect(rr("standup every day for 5 times")).toBe("FREQ=DAILY;COUNT=5");
    expect(rr("x every 2 weeks 6 times")).toBe("FREQ=WEEKLY;INTERVAL=2;COUNT=6");
    expect(rr("x every mon,wed until 2026-12-31")).toBe("FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261231");
    expect(rr("x every week until dec 24")).toBe("FREQ=WEEKLY;UNTIL=20261224");
    expect(rep("pay rent every month until 2027-01-31 please").rest.split(/\s+/).filter(Boolean).join(" ")).toBe("pay rent please");
    expect(rr("ring 5 times")).toBeNull();
  });
});

describe("parseQuick with advanced repeats", () => {
  it("starts on the right day and strips the phrase", () => {
    const q = parseQuick("gym every mon,wed", now);
    expect(q.title).toBe("gym");
    expect(q.repeat).toBe("FREQ=WEEKLY;BYDAY=MO,WE");
    expect(q.dueAt).toBe(at(3, 16));
    const l = parseQuick("send report last friday of the month", now);
    expect(l.title).toBe("send report");
    expect(l.repeat).toBe("FREQ=MONTHLY;BYDAY=-1FR");
    expect(l.dueAt).toBe(at(3, 27));
    const s = parseQuick("sync every 2 weeks", now);
    expect([s.repeat, s.dueAt]).toEqual(["FREQ=WEEKLY;INTERVAL=2", at(3, 12)]);
    const d = parseQuick("sync every 2 weeks fri 3pm for 4 times", now);
    expect([d.title, d.repeat, d.dueAt]).toEqual(["sync", "FREQ=WEEKLY;INTERVAL=2;COUNT=4", at(3, 13, 15)]);
    expect(d.chips.find((c) => c.kind === "repeat")?.label).toContain("Every 2 weeks");
  });
});

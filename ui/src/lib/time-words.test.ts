/**
 * One reset wording everywhere (the 0.2.0 re-render: the limit dialog
 * said "It resets at 08:13" beside its card's "resets Sun 07:13 AM").
 * Every reset — the dialog, the account cards, Home's Runway, Usage, the
 * switch's targets, the notices — is written by `whenWords` and
 * `resetsWords`: the system's own clock, today / tomorrow / the weekday.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clock, dateWords, pastDay, pastWhen, resetsLead, resetsWords, sessionWhen, whenWords } from "./time-words";
import { outOfRoomConsequence, outOfRoomWords } from "./room";
import { resetWords, windowView } from "./quotaWindow";
import { windowDisplay } from "./wire";
import { paceWords } from "../components/BurnDown";
import { digestLine } from "./notice-words";

// Wednesday 28 October 2026, 2:00 PM local.
const NOW = new Date(2026, 9, 28, 14, 0).getTime();
const TODAY = new Date(2026, 9, 28, 15, 10).getTime();
const TOMORROW = new Date(2026, 9, 29, 8, 0).getTime();
const SATURDAY = new Date(2026, 9, 31, 7, 13).getTime();
const NEXT_WEDNESDAY = new Date(2026, 10, 4, 9, 0).getTime();

const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
const weekday = (ms: number) => new Date(ms).toLocaleDateString([], { weekday: "short" });
const twelveHour = new Intl.DateTimeFormat([], { hour: "numeric" }).resolvedOptions().hourCycle?.startsWith("h1");

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});
afterEach(() => {
  vi.useRealTimers();
});

describe("the clock", () => {
  it("is the system's own style, never a hard-coded 12 or 24 hours", () => {
    expect(clock(SATURDAY)).toBe(time(SATURDAY));
  });

  it.runIf(twelveHour)("on a 12-hour system: no leading zero beside AM", () => {
    expect(clock(SATURDAY)).toMatch(/^7:13\s?AM$/);
  });
});

describe("a reset, by its day", () => {
  it("today: the time", () => {
    expect(whenWords(TODAY, NOW)).toBe(time(TODAY));
    expect(resetsWords(TODAY, NOW)).toBe(`resets at ${time(TODAY)}`);
    expect(resetsLead(TODAY, NOW)).toBe(`Resets at ${time(TODAY)}`);
  });

  it("tomorrow: the time and tomorrow — by the calendar, not 24 hours", () => {
    expect(whenWords(TOMORROW, NOW)).toBe(`${time(TOMORROW)} tomorrow`);
    expect(resetsWords(TOMORROW, NOW)).toBe(`resets at ${time(TOMORROW)} tomorrow`);
    const late = new Date(2026, 9, 28, 23, 50).getTime();
    const early = new Date(2026, 9, 29, 0, 20).getTime();
    expect(whenWords(early, late)).toBe(`${time(early)} tomorrow`);
  });

  it("later this week: the weekday and the time", () => {
    expect(whenWords(SATURDAY, NOW)).toBe(`${weekday(SATURDAY)} ${time(SATURDAY)}`);
    expect(resetsWords(SATURDAY, NOW)).toBe(`resets ${weekday(SATURDAY)} ${time(SATURDAY)}`);
  });

  it("a week on: its date too, so the weekday never reads as today's", () => {
    const day = new Date(NEXT_WEDNESDAY).toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
    expect(resetsWords(NEXT_WEDNESDAY, NOW)).toBe(`resets ${day} ${time(NEXT_WEDNESDAY)}`);
  });

  it("defaults now to the clock", () => {
    expect(resetsWords(TOMORROW)).toBe(resetsWords(TOMORROW, NOW));
  });
});

describe("a time gone by, by its day (P3: \"Finished 4:22 PM\" on one that ended days ago)", () => {
  const YESTERDAY = new Date(2026, 9, 27, 16, 22).getTime();
  const MONDAY = new Date(2026, 9, 26, 16, 22).getTime();
  const LAST_MONTH = new Date(2026, 8, 30, 16, 22).getTime();
  const LAST_YEAR = new Date(2025, 9, 28, 16, 22).getTime();
  it("today: the time alone", () => {
    expect(pastWhen(new Date(2026, 9, 28, 9, 5).getTime(), NOW)).toBe(time(new Date(2026, 9, 28, 9, 5).getTime()));
  });
  it("yesterday, then the weekday within the week, then its date, then its year", () => {
    expect(pastWhen(YESTERDAY, NOW)).toBe(`yesterday ${time(YESTERDAY)}`);
    expect(pastWhen(MONDAY, NOW)).toBe(`${weekday(MONDAY)} ${time(MONDAY)}`);
    expect(pastWhen(LAST_MONTH, NOW)).toBe(`${new Date(LAST_MONTH).toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })} ${time(LAST_MONTH)}`);
    expect(pastWhen(LAST_YEAR, NOW)).toContain("2025");
  });
  it("over a day old, never the time alone", () => {
    for (const at of [YESTERDAY, MONDAY, LAST_MONTH, LAST_YEAR]) expect(pastWhen(at, NOW)).not.toBe(clock(at));
  });
  it("where room is short, its day alone (today its time): the header's state chip gives way to it (0.4.4)", () => {
    expect(pastDay(new Date(2026, 9, 28, 9, 5).getTime(), NOW)).toBe(time(new Date(2026, 9, 28, 9, 5).getTime()));
    expect(pastDay(YESTERDAY, NOW)).toBe("yesterday");
    expect(pastDay(MONDAY, NOW)).toBe(weekday(MONDAY));
    expect(pastDay(LAST_MONTH, NOW)).toBe(new Date(LAST_MONTH).toLocaleDateString([], { month: "short", day: "numeric" }));
    expect(pastDay(LAST_YEAR, NOW)).toContain("2025");
    // The same day as the full form says, its clock left out.
    for (const at of [YESTERDAY, MONDAY, LAST_MONTH, LAST_YEAR]) expect(pastWhen(at, NOW)).toContain(pastDay(at, NOW));
  });
});

describe("every caller says it the one way", () => {
  it("a window's reset and its used-up line", () => {
    expect(resetWords(windowDisplay("tracking", { percentage: 40, resets_at_ms: TOMORROW }))).toBe(`resets at ${time(TOMORROW)} tomorrow`);
    const out = windowView(windowDisplay("exhausted", { percentage: 100, resets_at_ms: SATURDAY }));
    expect(out.words).toBe(`Used up until ${weekday(SATURDAY)} ${time(SATURDAY)}`);
    expect(out.resets).toBe(`resets ${weekday(SATURDAY)} ${time(SATURDAY)}`);
  });

  it("an account out of room", () => {
    expect(outOfRoomWords(TODAY, NOW)).toBe(`Out of room until ${time(TODAY)}`);
    expect(outOfRoomWords(TOMORROW, NOW)).toBe(`Out of room until ${time(TOMORROW)} tomorrow`);
    expect(outOfRoomConsequence("Work Max", SATURDAY, NOW)).toBe(
      `You can still pick it, but the conversation stops at Work Max's limit straight away and waits there until ${weekday(SATURDAY)} ${time(SATURDAY)}.`,
    );
  });

  it("Usage's pace", () => {
    const runway = { exhausts_at_ms: TODAY, resets_at_ms: TOMORROW, pct_per_hour: 9 } as Parameters<typeof paceWords>[0];
    expect(paceWords(runway, NOW)).toBe(`At this pace it runs out about ${time(TODAY)}, before the reset at ${time(TOMORROW)} tomorrow`);
    expect(paceWords({ ...runway, exhausts_at_ms: null, pct_per_hour: null }, NOW)).toBe(`Resets at ${time(TOMORROW)} tomorrow`);
  });

  it("the digest", () => {
    const digest = { finished: 0, restarted: 0, resets: 0, updates: 0, next_reset_account: "Work Max", next_reset_ms: TODAY } as unknown as Parameters<typeof digestLine>[0];
    expect(digestLine(digest, NOW)).toBe(`Today: Work Max resets at ${time(TODAY)}`);
  });
});

describe("a session row's when", () => {
  it("says \"ago\" for every ended session, one stopped with an error too", () => {
    const at = NOW - 7 * 3600_000;
    const row = (status: string) => sessionWhen({ status, started_at_ms: at - 3600_000, last_activity_ms: at, attention: { since_ms: at } }, NOW);
    expect(row("finished")).toBe("7 h ago");
    expect(row("failed")).toBe("7 h ago");
    // A live one: how long it has been at what it's doing.
    expect(row("waiting_user")).toBe("7 h");
  });
});

describe("a calendar date in words (renewals, the Account's choice)", () => {
  const now = new Date("2026-11-02T12:00:00").getTime();
  it("says the day and month, with the year only when it isn't this one", () => {
    expect(dateWords("2026-12-03", now)).toBe("3 Dec");
    expect(dateWords("2027-01-05", now)).toBe("5 Jan 2027");
  });
  it("keeps the day entered, whatever the time zone, and leaves a non-date alone", () => {
    expect(dateWords("2026-11-01", now)).toBe("1 Nov");
    expect(dateWords("soon", now)).toBe("soon");
  });
});

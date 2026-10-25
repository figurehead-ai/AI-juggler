/**
 * D-25 vitest — an account's runway in words: both windows, used up,
 * learning, unmeasured, a session's own reading as the fallback, and the
 * pace line only when it will run out; Accounts' activity and best pick.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AccountRunway, paceWords, runwayLines, windowLine } from "./AccountRunway";
import { placementWords } from "../screens/Profiles";
import { newWorkAdvice } from "./AccountRunway";
import { activityWords } from "../lib/account-words";
import type { AccountInfo, InstanceInfo, QuotaDisplayData } from "../lib/bridge";
import { forecastDisplay, quotaDisplay, windowDisplay } from "../lib/wire";

afterEach(cleanup);

const NOW = 1_728_800_000_000;
const display = (five: number | null, week: number | null, extra: Partial<QuotaDisplayData> = {}): QuotaDisplayData =>
  ({
    ...quotaDisplay("work"),
    five_hour: five === null ? null : windowDisplay("tracking", { percentage: five }),
    weekly: week === null ? null : windowDisplay("tracking", { percentage: week }),
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: forecastDisplay("learning", { badge: "" }),
    ...extra,
  }) as QuotaDisplayData;
const session = (id: string, status: string, account = "work", used: number | null = null): InstanceInfo =>
  ({
    id,
    status,
    account_id: account,
    quota: used === null ? null : { used_percentage: used, resets_at_ms: null, confidence: "computed" },
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000,
  }) as InstanceInfo;

describe("D-25 runway words", () => {
  it("a measured window says what's left; low and used up are named", () => {
    expect(windowLine("5-hour window", windowDisplay("tracking", { percentage: 42 }))).toMatchObject({ left: 58, words: "58% left", tone: "ok" });
    expect(windowLine("This week", windowDisplay("tracking", { percentage: 80 })).tone).toBe("low");
    expect(windowLine("This week", windowDisplay("exhausted", { percentage: 100 }))).toMatchObject({ left: 0, tone: "out" });
    expect(windowLine("x", windowDisplay("learning")).words).toBe("Measuring — shows after a little use");
    expect(windowLine("x", undefined).words).toBe("Not measured yet");
  });

  it("with no account reading, the newest session's own line stands in, named as such", () => {
    const lines = runwayLines(undefined, [session("a", "running", "work", 30), session("b", "finished", "work", 90)]);
    expect(lines).toHaveLength(1);
    expect(lines[0]!.label).toBe("Last read in a session");
    expect(lines[0]!.words).toBe("70% left");
    expect(runwayLines(undefined, [])).toHaveLength(2);
  });

  it("the pace line shows only when the window will run out", () => {
    expect(paceWords(display(50, 10))).toBeNull();
    expect(paceWords(display(50, 10, { forecast: forecastDisplay("exhausts", { tte_min: 40, badge: "computed" }) }))).toBe("At this pace it runs out in about 40 min");
  });

  it("renders both windows", () => {
    const { getByTestId } = render(<AccountRunway display={display(42, 10)} />);
    expect(getByTestId("account-runway").textContent).toContain("5-hour window");
    expect(getByTestId("account-runway").textContent).toContain("90% left");
  });
});

describe("D-25 Accounts — activity and the best pick", () => {
  it("says what runs on an account", () => {
    expect(activityWords([])).toBe("Nothing running");
    expect(activityWords([session("a", "running"), session("b", "waiting_user")])).toBe("2 running · 1 needs you");
  });

  it("picks the signed-in account with the most left in its tighter window", () => {
    const accounts = [
      { id: "work", label: "Work Max", login_state: "logged_in" },
      { id: "personal", label: "Personal", login_state: "logged_in" },
      { id: "out", label: "Signed out", login_state: "logged_out" },
    ] as AccountInfo[];
    const displays = [display(10, 90), { ...display(40, 30), account: "personal" }, { ...display(0, 0), account: "out" }];
    expect(newWorkAdvice(accounts, displays, [])).toMatchObject({
      best: { account: { id: "personal" }, left: 60 },
      words: "Best for new work: Personal, 60% left.",
    });
    expect(newWorkAdvice(accounts, [], [])).toEqual({ best: null, words: null });
  });

  it("never recommends a low account, and says when it can't tell (QA batch 8)", () => {
    const accounts = [
      { id: "g", label: "Account G", login_state: "logged_in" },
      { id: "a", label: "Account A", login_state: "logged_in" },
    ] as AccountInfo[];
    // Only G is measured, at 6% left: the others aren't, so nothing is "best".
    const onlyG = [{ ...display(94, 10), account: "g" }];
    expect(newWorkAdvice(accounts, onlyG, [])).toEqual({
      best: null,
      words: "Account G has 6% left, and Account A isn't measured yet, so AI Juggler can't tell which has room.",
    });
    // One with room, the other unmeasured: it has room, and the words say what's unknown.
    expect(newWorkAdvice(accounts, [{ ...display(40, 10), account: "g" }], [])).toEqual({
      best: { account: accounts[0], left: 60 },
      words: "Account G has 60% left; Account A isn't measured yet.",
    });
    // Every one measured and low: the most, said as low.
    expect(newWorkAdvice(accounts, [onlyG[0]!, { ...display(80, 10), account: "a" }], []).words).toBe(
      "Every account is low: Account A has the most, 20% left.",
    );
  });
});

describe("ADR-0018 where an account lives", () => {
  const account = (extra: Partial<AccountInfo>): AccountInfo =>
    ({ id: "a", provider: "claude-code", label: "Work", in_place: false, ...extra }) as AccountInfo;
  it("says whether it uses your folder or is a copy; nothing for a fresh account", () => {
    expect(placementWords(account({ in_place: true, import_source_path: "/Users/you/.claude-work" }))).toBe(
      "uses ~/.claude-work where it is",
    );
    expect(placementWords(account({ import_source_path: "/Users/you/.claude" }))).toBe("a copy of ~/.claude");
    expect(placementWords(account({}))).toBeNull();
  });
});

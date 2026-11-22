/**
 * F-rc3c-1 (the 0.4.6 rc3c walk): two days after the Dev build last ran,
 * Home's "Runway by account", the Your apps cards and the rail's Out
 * marks showed two-day-old readings as current: the Lab account read
 * "34% left this week" though it was out, two read "Out · resets …" with
 * room, until the apps' own records came in minutes later. The ruling:
 * stale usage at start shows its age ("as of 2 days ago, refreshing…")
 * until a fresh reading lands; a reading whose window has reset since is
 * history, never Out.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { runwayDetail, runwayLines, runwayOf } from "../components/AccountRunway";
import { HomeRunway } from "../components/HomeRunway";
import { ContinueOn } from "../pages/session/ContinueOn";
import { fromInstance } from "../pages/session/model";
import * as targets from "./ipc/switch-targets";
import * as setup from "./ipc/switchSetup";
import { roomCounts, roomMarkOf } from "./room-mark";
import { READING_OLD_MS, oldReadingWords, windowView } from "./quotaWindow";
import { resetsWords, whenWords } from "./time-words";
import { quotaDisplay, windowDisplay } from "./wire";
import { instance, subscription } from "../test/fixtures";
import type { QuotaDisplayData, WindowDisplay } from "./ipc/quota";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { SwitchTargetData } from "./ipc/switch-targets";
import type { SubscriptionInfo } from "./subscriptions";

const NOW = new Date(2026, 10, 19, 10, 0).getTime();
const MIN = 60_000;
const DAY = 86_400_000;
const RESET = NOW + 2 * DAY;

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOW);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

/** A week read from the Claude app `ago` ms before NOW, `used`% used. */
const week = (used: number, ago: number, resetsAt: number = RESET): WindowDisplay =>
  windowDisplay(used >= 100 ? "exhausted" : "stale", {
    percentage: used,
    badge: "measured",
    measured_at_ms: NOW - ago,
    resets_at_ms: resetsAt,
    reset_basis: "measured",
    source: "desktop_app",
    source_words: "the Claude app",
  });
const lab = (weekly: WindowDisplay | null, over: Partial<QuotaDisplayData> = {}): QuotaDisplayData => quotaDisplay("lab", { weekly, ...over });

describe("an old reading says how old", () => {
  it("on the runway line, and after the reset on the line under Home's figure", () => {
    const run = runwayOf(lab(week(66, 2 * DAY)));
    expect(run.left).toBe(34);
    expect(run.words).toBe("34% left this week · as of 2 days ago");
    expect(run.age).toBe("as of 2 days ago");
    expect(runwayDetail(run)).toBe(`Week ${resetsWords(RESET, NOW)} · as of 2 days ago`);
  });

  it("a fresh one reads as before: no age, nothing refreshing", () => {
    for (const ago of [20 * MIN, READING_OLD_MS - MIN]) {
      const run = runwayOf(lab(week(66, ago), { refreshing: true }));
      expect(run.words).toBe("34% left this week");
      expect(run.age).toBeNull();
      expect(runwayDetail(run)).toBe(`Week ${resetsWords(RESET, NOW)}`);
    }
    // From the hour on, it says so, in the app's words.
    expect(oldReadingWords(NOW - READING_OLD_MS, false, NOW)).toBe("as of 1 h ago");
    expect(oldReadingWords(NOW - 3 * DAY, false, NOW)).toBe("as of 3 days ago");
    expect(oldReadingWords(null, true, NOW)).toBeNull();
  });

  it("while this start's first read runs, it adds refreshing…; when a fresh reading lands, both go", () => {
    const starting = runwayOf(lab(week(66, 2 * DAY), { refreshing: true }));
    expect(starting.words).toBe("34% left this week · as of 2 days ago, refreshing…");
    expect(runwayDetail(starting)).toBe(`Week ${resetsWords(RESET, NOW)} · as of 2 days ago, refreshing…`);
    // The window lines (the Accounts and Usage cards) say it after their own age.
    const line = runwayLines(lab(week(66, 2 * DAY), { refreshing: true }), [], NOW).find((one) => one.label === "This week")!;
    expect(line.words).toBe(`34% left · ${resetsWords(RESET, NOW)} · from the Claude app, 2 days ago, refreshing…`);
    // The first read done, no newer reading: the age stays, refreshing goes.
    expect(runwayOf(lab(week(66, 2 * DAY), { refreshing: false })).words).toBe("34% left this week · as of 2 days ago");
    // A fresh reading: both go.
    const fresh = runwayOf(lab(week(100, 2 * MIN), { refreshing: false }));
    expect(fresh.age).toBeNull();
    expect(fresh.words).toBe("0% left this week");
  });

  it("on an Out or Low mark: its label and its title (the rail shows the title)", () => {
    const out = roomMarkOf(lab(week(100, 2 * DAY)), [], NOW)!;
    expect(out.kind).toBe("out");
    expect(out.label).toBe(`Out · resets ${whenWords(RESET, NOW)} · as of 2 days ago`);
    expect(out.short).toBe("Out");
    expect(out.title).toBe(`Out of room until ${whenWords(RESET, NOW)} · as of 2 days ago`);
    const starting = roomMarkOf(lab(week(100, 2 * DAY), { refreshing: true }), [], NOW)!;
    expect(starting.label).toBe(`Out · resets ${whenWords(RESET, NOW)} · as of 2 days ago, refreshing…`);
    const low = roomMarkOf(lab(week(88, 5 * 3_600_000)), [], NOW)!;
    expect(low.label).toBe("Low · 12% left · as of 5 h ago");
    // Fresh: the mark as before.
    expect(roomMarkOf(lab(week(100, 5 * MIN)), [], NOW)!.label).toBe(`Out · resets ${whenWords(RESET, NOW)}`);
  });

  it("on Home's Runway by account, under the figure", async () => {
    const folder = { id: "lab", provider: "claude-code", label: "Lab", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false } as AccountInfo;
    render(<HomeRunway accounts={[folder]} quota={[lab(week(66, 2 * DAY), { refreshing: true })]} sessions={[]} />);
    const row = (await screen.findAllByTestId("home-runway-account"))[0]!;
    expect(row.textContent).toContain("34% left");
    expect(within(row).getByTestId("home-runway-facts").textContent).toBe(`Week ${resetsWords(RESET, NOW)} · as of 2 days ago, refreshing…`);
  });
});

describe("Continue on another account", () => {
  it("the compact mark beside a target's name says the age in its title and to a screen reader, as the row does", async () => {
    const target: SwitchTargetData = {
      recipe_id: "r-lab",
      provider: "claude-code",
      account_id: "lab",
      account_label: "Lab",
      badge: "Same conversation, another app · Beta",
      validated: false,
      cross_provider: false,
      drops: [],
      score: null,
      filtered_reason: null,
      top_factors: [],
      factors: [],
      confidence: "Measured",
      tte_min: null,
      suggested: false,
    };
    vi.spyOn(setup, "switchSetupCheck").mockResolvedValue({ from: "Work", to: "Lab", already_there: 0, offers: [] });
    vi.spyOn(targets, "switchTargets").mockResolvedValue([target]);
    const model = fromInstance(instance({ id: "s-1", account_id: "src", account_label: "Source", conversation_id: "c", cwd: "/w" }));
    render(<ContinueOn model={model} accounts={[]} displays={[lab(week(100, 2 * DAY), { refreshing: true })]} onPick={vi.fn()} />);
    await waitFor(() => expect(screen.getAllByTestId("continue-account")).toHaveLength(1));
    const account = screen.getByTestId("continue-account");
    const title = `Out of room until ${whenWords(RESET, NOW)} · as of 2 days ago, refreshing…`;
    // The compact chip ("Out", beside the name) and the row's own chip.
    const chips = within(account).getAllByTestId("switch-target-room");
    expect(chips.map((chip) => chip.textContent)).toContain(`Out${title}`);
    for (const chip of chips) {
      expect(chip.getAttribute("title")).toBe(title);
      expect(chip.querySelector(".sr-only")?.textContent).toBe(title);
    }
  });
});

describe("a reading whose window has reset since", () => {
  const passed = NOW - 3 * 3_600_000;

  it("is history, never Out: no mark, no 0% left, the last reading named as such", () => {
    const display = lab(week(100, 2 * DAY, passed));
    const run = runwayOf(display);
    expect(run.left).toBeNull();
    expect(run.words).toBe("Last reading 2 days ago (the Claude app): 100% used; that window has reset since");
    expect(roomMarkOf(display, [], NOW)).toBeNull();
    const view = windowView(display.weekly, null, NOW);
    expect(view.left).toBeNull();
    expect(view.tone).toBe("none");
    // A fuller reading rounded to 0% left, too.
    expect(roomMarkOf(lab(week(99.6, 2 * DAY, passed)), [], NOW)).toBeNull();
  });

  it("isn't counted out, and the window with room left decides", () => {
    // The five hours used up, reset since; the week has room.
    const five = windowDisplay("exhausted", { percentage: 100, badge: "measured", measured_at_ms: NOW - 6 * 3_600_000, resets_at_ms: passed, reset_basis: "measured" });
    const display = lab(week(40, 10 * MIN), { five_hour: five, room_back: { window: "five_hour", at_ms: passed } });
    const run = runwayOf(display);
    expect(run.left).toBe(60);
    expect(run.words).toBe("60% left this week");
    expect(roomMarkOf(display, [], NOW)).toBeNull();
    const account: SubscriptionInfo = {
      ...subscription({ folder: "lab" }),
      key: "acct-lab",
      account_id: "acct-lab",
      state: "signed_in",
      profiles: [{ id: "lab", label: "Lab", folder: "lab", plan: null, renewal_date: null, fee_usd: null }],
    };
    expect(roomCounts([account], [display, lab(week(100, 2 * DAY, passed))], NOW)).toEqual({ out: 0, low: 0 });
  });
});

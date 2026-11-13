/**
 * 0.4.4 (a persona lab): an Account out on both windows read "Out · resets
 * 7:06 PM" on its tiles (the five hours' reset), "resets Sun 5:06 PM" in
 * Needs you and "5-hour window resets at 7:06 PM" in Home's Runway. It has
 * room again only when the later window resets: every surface gives the
 * backend's one answer (`room_back`), so they all say the week's.
 */
import { describe, expect, it } from "vitest";
import { runwayOf } from "../components/AccountRunway";
import { roomMarkOf, roomMarkOfProfile } from "./room-mark";
import { launchRoom } from "./launch-pick";
import { roomOfProfile } from "./room-targets";
import { resetsWords, whenWords } from "./time-words";
import type { QuotaDisplayData, RoomBack, WindowDisplay } from "./ipc/quota";
import { subscription } from "../test/fixtures";
import type { SubscriptionInfo } from "./subscriptions";

const NOW = 1_794_000_000_000;
const FIVE = NOW + 2 * 3_600_000;
const WEEK = NOW + 3 * 86_400_000;
const window = (percentage: number, resets: number | null, badge = "measured"): WindowDisplay => ({
  state: percentage >= 100 ? "exhausted" : "tracking",
  percentage,
  badge,
  stale_note_ms: null,
  resets_at_ms: resets,
  reset_basis: resets === null ? null : "measured",
  measured_at_ms: NOW - 60_000,
  burned_tokens: 0,
  burned_credits: null,
  headroom_tokens: null,
});
const display = (five: WindowDisplay, week: WindowDisplay, back: RoomBack | null | undefined): QuotaDisplayData => ({
  account: "cc-work",
  five_hour: five,
  five_hour_omitted_reason: null,
  weekly: week,
  weekly_omitted_reason: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
  unavailable_reason: null,
  credits: null,
  room_back: back,
});
// The lab's shape, as the backend sends it.
const bothOut = display(window(100, FIVE), window(100, WEEK), { window: "weekly", at_ms: WEEK });
const work: SubscriptionInfo = {
  ...subscription({ folder: "cc-work" }),
  key: "acct-work",
  account_id: "acct-work",
  state: "signed_in",
  profiles: ["cc-work", "app-work"].map((p) => ({ id: p, label: p, folder: p, plan: null, renewal_date: null, fee_usd: null })),
};

describe("an Account out on both windows", () => {
  it("says the later reset on every surface", () => {
    const run = runwayOf(bothOut);
    expect(run.left).toBe(0);
    expect(run.resetsAtMs).toBe(WEEK);
    // Home's Runway: the week, never the five hours.
    expect(run.resets).toBe(`Week ${resetsWords(WEEK)}`);
    expect(run.words).toBe("0% left this week");
    // The rail's and Home's App tiles, the Account page (one mark).
    expect(roomMarkOf(bothOut, [], NOW)!.label).toBe(`Out · resets ${whenWords(WEEK, NOW)}`);
    expect(roomMarkOfProfile("app-work", [work], [bothOut], [], NOW)!.title).toBe(`Out of room until ${whenWords(WEEK, NOW)}`);
    // New session's picker and the switch's targets.
    expect(launchRoom(bothOut).resetsAtMs).toBe(WEEK);
    expect(roomOfProfile("app-work", [work], [bothOut]).resetsAtMs).toBe(WEEK);
  });

  it("goes by the window the backend names, whichever is fuller", () => {
    // The week rounds to 0% left; the five hours are at 100%.
    const rounded = display(window(100, FIVE), window(99.6, WEEK), { window: "weekly", at_ms: WEEK });
    expect(runwayOf(rounded).resetsAtMs).toBe(WEEK);
    expect(roomMarkOf(rounded, [], NOW)!.label).toBe(`Out · resets ${whenWords(WEEK, NOW)}`);
  });

  it("says no time when a needed reset truly isn't known", () => {
    const unknown = display(window(100, FIVE), window(100, null), { window: "weekly", at_ms: null });
    const run = runwayOf(unknown);
    expect([run.left, run.resetsAtMs, run.resets]).toEqual([0, null, null]);
    const mark = roomMarkOf(unknown, [], NOW)!;
    expect([mark.label, mark.title]).toEqual(["Out", "Out of room; when it resets isn't known yet"]);
  });

  it("out on one window only, that window's reset", () => {
    const fiveOnly = display(window(100, FIVE), window(60, WEEK), { window: "five_hour", at_ms: FIVE });
    expect(runwayOf(fiveOnly).resets).toBe(`5-hour window ${resetsWords(FIVE)}`);
    expect(roomMarkOf(fiveOnly, [], NOW)!.label).toBe(`Out · resets ${whenWords(FIVE, NOW)}`);
    // The estimate's "about" follows the window that keeps it out.
    const estimated = display(window(100, FIVE), window(100, WEEK, "estimated"), { window: "weekly", at_ms: WEEK });
    expect(roomMarkOf(estimated, [], NOW)!.label).toBe(`Out · resets about ${whenWords(WEEK, NOW)}`);
  });
});

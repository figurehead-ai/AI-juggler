/**
 * F-rc3c-3 (the 0.4.6 rc3c walk, part 4): a folder moved from one Account
 * to another while the window was open (a Claude app profile's own
 * sign-in written into the folder it shares with its CLI). The rail's
 * header for the default Claude app, whose Account has room, then wore
 * the Out mark: its mark read the Accounts once, when it was drawn, and
 * the readings every half minute, so it still counted the moved folder's
 * new reading (the other Account's, out) as its own Account's newest. An
 * Out mark reads the Accounts as fresh as the readings it is read with;
 * the top bar's count too.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as subs from "../lib/ipc/subscriptions";
import * as quota from "../lib/ipc/quota";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { ProfileRoomChip, RoomCount } from "./RoomChip";

const NOW = Date.UTC(2026, 10, 18, 21, 0, 0);
const RESET = NOW + 10 * 3_600_000;

const account = (key: string, ids: string[]): SubscriptionInfo => ({
  key,
  account_id: `acct:anthropic:${key}`,
  name: null,
  display_name: key,
  colour_slot: null,
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: ids.map((id) => ({ id, label: id, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null })),
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
});

/** Folder `folder`'s weekly reading: `used`%, measured at `at`. */
const reading = (folder: string, used: number, at: number): QuotaDisplayData => ({
  account: folder,
  weekly: {
    state: used >= 100 ? "exhausted" : "tracking",
    percentage: used,
    badge: "measured",
    stale_note_ms: null,
    resets_at_ms: RESET,
    reset_basis: "measured",
    measured_at_ms: at,
    burned_tokens: 0,
    burned_credits: null,
    headroom_tokens: null,
  },
  weekly_omitted_reason: null,
  five_hour: null,
  five_hour_omitted_reason: null,
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
  unavailable_reason: null,
  credits: null,
});

// Before: the shared folder ("shared") on the Team seat with the default
// app's folder; the personal plan (out) on its own folders.
const BEFORE = [account("team", ["default", "shared"]), account("personal", ["personal", "lab"])];
// After: the shared folder on the personal plan.
const AFTER = [account("team", ["default"]), account("personal", ["shared", "personal", "lab"])];

const READ_BEFORE = [reading("default", 7, NOW - 60_000), reading("shared", 7, NOW - 60_000), reading("personal", 100, NOW - 120_000), reading("lab", 100, NOW - 120_000)];
// The shared folder's newest reading is the personal plan's now (out).
const READ_AFTER = [reading("default", 7, NOW - 60_000), reading("shared", 100, NOW + 60_000), reading("personal", 100, NOW + 60_000), reading("lab", 100, NOW - 120_000)];

let accounts = BEFORE;
let readings = READ_BEFORE;

beforeEach(() => {
  vi.useFakeTimers({ now: NOW, toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "Date"] });
  accounts = BEFORE;
  readings = READ_BEFORE;
  vi.spyOn(subs, "subscriptionsList").mockImplementation(() => Promise.resolve(accounts));
  vi.spyOn(quota, "quotaDisplayAll").mockImplementation(() => Promise.resolve(readings));
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

/** Lets the reads in flight answer, after `ms` of the window's time. */
async function pass(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

describe("an Out mark reads the Accounts as fresh as its readings", () => {
  it("a folder that moved to an Account out of room doesn't mark the Account it left", async () => {
    render(
      <>
        <ProfileRoomChip profileId="default" compact testId="default-mark" />
        <ProfileRoomChip profileId="shared" compact testId="shared-mark" />
        <RoomCount />
      </>,
    );
    await pass(0);
    expect(screen.queryByTestId("default-mark")).toBeNull();
    expect(screen.queryByTestId("shared-mark")).toBeNull();
    expect(screen.getByTestId("header-room-count").textContent).toContain("1 account out");

    // The folder moves (its sign-in read as the personal plan's); its
    // next reading is that plan's. Two of the window's half-minute polls
    // pass (four while another app is in front).
    accounts = AFTER;
    readings = READ_AFTER;
    await pass(4 * 30_000 + 1_000);

    expect(screen.queryByTestId("default-mark"), "the default app's Account (the Team seat) has room").toBeNull();
    expect(screen.getByTestId("shared-mark").dataset.kind, "the moved folder's Account is out").toBe("out");
    expect(screen.getByTestId("header-room-count").textContent, "one Account is out, not two").toContain("1 account out");
  });
});

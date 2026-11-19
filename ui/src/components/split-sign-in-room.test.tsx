/**
 * AJ-E3210 (F-rc3c-3, D-135): a Claude app profile and its CLI share one
 * folder that holds two sign-ins. Each App is on its own Account, so each
 * wears its own Account's mark: the default Claude app (the Team seat) has
 * no Out; the aij-test profile's desktop App (the personal plan, out) is
 * Out; its CLI App (the Team seat) isn't. The top bar counts one Account
 * out. Resume reads by App: a terminal run its folder's CLI App's room, a
 * session launched in the desktop App that App's (acceptance tests 6–7).
 */
import { act, cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as subs from "../lib/ipc/subscriptions";
import * as quota from "../lib/ipc/quota";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { roomCounts, roomMarkOfApp } from "../lib/room-mark";
import { roomOfApp } from "../lib/room-targets";
import { ProfileRoomChip, RoomCount } from "./RoomChip";
import { useTargetRoom } from "./switch/TargetRoom";
import { ACCOUNT_NOT_KNOWN_YET } from "../lib/account-words";
import { accountGroups } from "./HomeRunway";

const NOW = Date.UTC(2026, 10, 18, 21, 0, 0);
const RESET = NOW + 10 * 3_600_000;
const TEAM = "acct:anthropic:team";
const PERSONAL = "acct:anthropic:personal";
const DEFAULT_APP = "app:claude-app:/u/Claude";
const PROFILE_APP = "app:claude-app:/u/ClaudeProfiles/aij-test";
const SHARED_CLI = "cli:shared";

const account = (id: string, folders: string[], apps: string[]): SubscriptionInfo => ({
  key: id,
  account_id: id,
  name: null,
  display_name: id === TEAM ? "Team" : "Personal Max",
  colour_slot: null,
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: folders.map((folder) => ({ id: folder, label: folder, folder: `~/p/${folder}`, plan: null, renewal_date: null, fee_usd: null })),
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
  apps,
});

/** Folder `folder`'s weekly reading under Account `under`: `used`%. */
const reading = (folder: string, under: string | null, used: number): QuotaDisplayData => ({
  account: folder,
  signed_in_as: under,
  weekly: {
    state: used >= 100 ? "exhausted" : "tracking",
    percentage: used,
    badge: "measured",
    stale_note_ms: null,
    resets_at_ms: RESET,
    reset_basis: "measured",
    measured_at_ms: NOW - 60_000,
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

// The rc3c setup, as the backend lists it after AJ-E3210: the shared
// folder stays the Team seat's (its CLI's), with the default app's; the
// aij-test desktop App is on the personal plan, as the personal folder.
const ACCOUNTS = [account(TEAM, ["default", "shared"], ["cli:default", DEFAULT_APP, SHARED_CLI]), account(PERSONAL, ["personal"], ["cli:personal", PROFILE_APP])];
// The shared folder's own readings are the Team seat's (7% used); the
// personal plan's taken on it (100%, the app's meter) are tagged as its.
const READINGS = [reading("default", TEAM, 7), reading("shared", TEAM, 7), reading("personal", PERSONAL, 100), reading("shared", PERSONAL, 100)];

beforeEach(() => {
  vi.useFakeTimers({ now: NOW, toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "Date"] });
  vi.spyOn(subs, "subscriptionsList").mockImplementation(() => Promise.resolve(ACCOUNTS));
  vi.spyOn(quota, "quotaDisplayAll").mockImplementation(() => Promise.resolve(READINGS));
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

async function pass(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

describe("each App wears its own Account's mark (acceptance test 6)", () => {
  it("rules: the desktop App is out, its CLI App and the default app aren't", () => {
    expect(roomMarkOfApp(PROFILE_APP, "shared", ACCOUNTS, READINGS, [], NOW)?.kind).toBe("out");
    expect(roomMarkOfApp(SHARED_CLI, "shared", ACCOUNTS, READINGS, [], NOW)).toBeNull();
    expect(roomMarkOfApp(DEFAULT_APP, "default", ACCOUNTS, READINGS, [], NOW)).toBeNull();
    expect(roomCounts(ACCOUNTS, READINGS, NOW)).toEqual({ out: 1, low: 0 });
    // Before its first Code record a desktop App is on no Account: no
    // mark, never its folder's.
    const unknown = [account(TEAM, ["default", "shared"], ["cli:default", SHARED_CLI]), ACCOUNTS[1]!];
    expect(roomMarkOfApp("app:claude-app:/u/ClaudeProfiles/new", "shared", unknown, READINGS, [], NOW)).toBeNull();
  });

  it("drawn: the rail's marks and the top bar's count", async () => {
    render(
      <>
        <ProfileRoomChip profileId="default" appId={DEFAULT_APP} compact testId="default-mark" />
        <ProfileRoomChip profileId="shared" appId={PROFILE_APP} compact testId="profile-mark" />
        <ProfileRoomChip profileId="shared" appId={SHARED_CLI} compact testId="cli-mark" />
        <RoomCount />
      </>,
    );
    await pass(0);
    expect(screen.queryByTestId("default-mark"), "the default app's Account (the Team seat) has room").toBeNull();
    expect(screen.getByTestId("profile-mark").dataset.kind, "the profile's own Account is out").toBe("out");
    expect(screen.queryByTestId("cli-mark"), "its CLI runs on the Team seat").toBeNull();
    expect(screen.getByTestId("header-room-count").textContent).toContain("1 account out");
  });

  it("an Account only a Claude app profile is on is a Runway row of its own", () => {
    const onlyApp = { ...account(PERSONAL, [], [PROFILE_APP]) };
    const groups = accountGroups([], [ACCOUNTS[0]!, onlyApp]);
    expect(groups.map((group) => group.subscription?.account_id)).toContain(PERSONAL);
    expect(accountGroups([], [ACCOUNTS[0]!, onlyApp], true).map((group) => group.subscription?.account_id)).not.toContain(PERSONAL);
  });
});

describe("Resume reads the room of the session's App (acceptance test 7)", () => {
  it("a terminal session in the shared folder has its CLI App's room; a desktop one is out", () => {
    const terminal = roomOfApp(null, "shared", ACCOUNTS, READINGS);
    expect(terminal.left).toBe(93);
    expect(terminal.account).toBe("Team");
    const desktop = roomOfApp(PROFILE_APP, "shared", ACCOUNTS, READINGS);
    expect(desktop.left).toBe(0);
    expect(desktop.account).toBe("Personal Max");
    const unknown = roomOfApp("app:claude-app:/u/ClaudeProfiles/new", "shared", ACCOUNTS, READINGS);
    expect(unknown.words).toBe(ACCOUNT_NOT_KNOWN_YET);
  });

  it("the hook Resume reads", async () => {
    const terminal = renderHook(() => useTargetRoom("shared", true, null));
    const desktop = renderHook(() => useTargetRoom("shared", true, PROFILE_APP));
    await pass(0);
    terminal.rerender();
    desktop.rerender();
    expect(terminal.result.current?.kind, "a terminal run there has room").not.toBe("out");
    expect(desktop.result.current?.kind, "the Code tab session's Account is out until it resets").toBe("out");
  });
});

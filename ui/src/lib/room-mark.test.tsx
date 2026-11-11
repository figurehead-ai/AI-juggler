/**
 * 0.4.1 (the maintainer's finding): one unmistakable mark on every
 * Account and App out of room, the same words everywhere; low the calmer
 * amber; measured or computed readings only, an estimate says "about";
 * credits that carry work on aren't out.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { roomCountWords, roomCounts, roomMarkFrom, roomMarkOf, roomMarkOfProfile } from "./room-mark";
import { RoomChip } from "../components/RoomChip";
import { whenWords } from "./time-words";
import type { QuotaDisplayData, WindowDisplay } from "./ipc/quota";
import { subscription } from "../test/fixtures";
import type { SubscriptionInfo } from "./subscriptions";

afterEach(cleanup);

const NOW = 1_794_000_000_000;
const RESET = NOW + 90 * 60_000;
const window = (percentage: number | null, badge = "measured"): WindowDisplay => ({
  state: "tracking",
  percentage,
  badge,
  stale_note_ms: null,
  resets_at_ms: RESET,
  reset_basis: "measured",
  measured_at_ms: NOW - 60_000,
  burned_tokens: 0,
  burned_credits: null,
  headroom_tokens: null,
});
const display = (account: string, used: number | null, badge = "measured"): QuotaDisplayData => ({
  account,
  five_hour: window(used, badge),
  five_hour_omitted_reason: null,
  weekly: window(10),
  weekly_omitted_reason: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
  unavailable_reason: null,
  credits: null,
});
const sub = (id: string, profiles: string[], state: SubscriptionInfo["state"] = "signed_in"): SubscriptionInfo => ({
  ...subscription({ folder: profiles[0] }),
  key: id,
  account_id: id,
  state,
  profiles: profiles.map((p) => ({ id: p, label: p, folder: p, plan: null, renewal_date: null, fee_usd: null })),
});

describe("the room mark", () => {
  it("out: an empty gauge's words with when it resets; low: the calmer words; room: none", () => {
    const out = roomMarkOf(display("w", 100), [], NOW)!;
    expect(out.kind).toBe("out");
    expect(out.label).toBe(`Out · resets ${whenWords(RESET, NOW)}`);
    expect(out.short).toBe("Out");
    expect(out.title).toBe(`Out of room until ${whenWords(RESET, NOW)}`);
    const low = roomMarkOf(display("w", 88), [], NOW)!;
    expect(low.kind).toBe("low");
    expect(low.label).toBe("Low · 12% left");
    expect(roomMarkOf(display("w", 40), [], NOW)).toBeNull();
    expect(roomMarkOf(display("w", null), [], NOW)).toBeNull();
  });

  it("an estimate says about", () => {
    expect(roomMarkOf(display("w", 100, "estimated"), [], NOW)!.label).toBe(`Out · resets about ${whenWords(RESET, NOW)}`);
    expect(roomMarkFrom(10, null, true, NOW)!.label).toBe("Low · about 10% left");
    expect(roomMarkFrom(0, null, false, NOW)!.label).toBe("Out");
  });

  it("an App reads its Account's room: two folders, one Account, one mark", () => {
    const accounts = [sub("acct-work", ["cc-work", "app-work-max"])];
    const displays = [display("cc-work", 100)];
    expect(roomMarkOfProfile("app-work-max", accounts, displays, [], NOW)?.kind).toBe("out");
    expect(roomMarkOfProfile(null, accounts, displays, [], NOW)).toBeNull();
  });

  it("the top bar counts each signed-in Account once", () => {
    const accounts = [sub("acct-work", ["a", "b"]), sub("acct-home", ["c"]), sub("acct-side", ["d"]), sub("acct-off", ["e"], "signed_out")];
    const displays = [display("a", 100), display("b", 100), display("c", 90), display("d", 20), display("e", 100)];
    const counts = roomCounts(accounts, displays, NOW);
    expect(counts).toEqual({ out: 1, low: 1 });
    expect(roomCountWords(counts)).toBe("1 account out · 1 low");
    expect(roomCountWords({ out: 2, low: 0 })).toBe("2 accounts out");
    expect(roomCountWords({ out: 0, low: 1 })).toBe("1 account low");
    expect(roomCountWords({ out: 0, low: 0 })).toBeNull();
  });
});

describe("the chip", () => {
  it("draws colour, icon and words together, and says the whole in its name", () => {
    render(<RoomChip mark={roomMarkFrom(0, RESET, false, NOW)} />);
    const chip = screen.getByTestId("room-chip");
    expect(chip.getAttribute("data-kind")).toBe("out");
    expect(chip.className).toContain("text-danger");
    expect(chip.querySelector("svg")).not.toBeNull();
    // Seen: the short words; heard: the whole title (hidden text, as a
    // plain span may not carry an aria-label).
    expect(chip.querySelector("[aria-hidden=true]:not(svg)")?.textContent).toBe(`Out · resets ${whenWords(RESET, NOW)}`);
    expect(chip.querySelector(".sr-only")?.textContent).toBe(`Out of room until ${whenWords(RESET, NOW)}`);
    cleanup();
    render(<RoomChip mark={roomMarkFrom(12, RESET, false, NOW)} compact />);
    expect(screen.getByTestId("room-chip").className).toContain("text-warning");
    expect(screen.getByTestId("room-chip").querySelector("[aria-hidden=true]:not(svg)")?.textContent).toBe("Low");
    cleanup();
    const { container } = render(<RoomChip mark={null} />);
    expect(container.textContent).toBe("");
  });
});

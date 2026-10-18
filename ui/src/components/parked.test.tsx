/**
 * AJ-E1003 vitest — park until reset: the countdown is text and only for
 * a Measured reset (Estimated says so, with "Check now"); Park is offered
 * only to a quota-blocked Instance; at the reset one offer appears with
 * Resume and a distinct snooze ("Remind me in 30 min"), then — on its
 * last appearance — "Stop reminding me". Nothing resumes by itself.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ParkCard } from "./ParkCard";
import { NotificationRegion } from "./NotificationRegion";
import { parkedWords } from "../lib/parked";
import type { InstanceInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_PARK_CALLS__ as string[] | undefined) ?? [];
const NOW = 1_800_000_000_000;

const session = (over: Partial<InstanceInfo>): InstanceInfo =>
  ({ id: "s1", status: "quota_blocked", ...over }) as InstanceInfo;

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_PARK_CALLS__;
  delete w().__AIJ_PARKED_OFFERS__;
});

describe("AJ-E1003 parked words", () => {
  it("counts down only a Measured reset", () => {
    expect(parkedWords({ parked: "measured", parked_resets_at_ms: NOW + 2 * 3_600_000 + 13 * 60_000 }, NOW)!.short).toBe(
      "Parked · resets in 2h 13m",
    );
    expect(parkedWords({ parked: "measured", parked_resets_at_ms: NOW - 1 }, NOW)!.due).toBe(true);
    const estimated = parkedWords({ parked: "estimated", parked_resets_at_ms: NOW + 3_600_000 }, NOW)!;
    expect(estimated.short).toBe("Parked · reset estimated");
    expect(estimated.long).not.toMatch(/resets in/);
    expect(parkedWords({}, NOW)).toBeNull();
  });
});

describe("AJ-E1003 park card", () => {
  it("offers Park only to a quota-blocked Instance", async () => {
    const { container, rerender } = render(<ParkCard session={session({})} now={NOW} />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='park-go']")!);
    });
    expect(calls()).toEqual(["park:s1"]);
    rerender(<ParkCard session={session({ status: "running" })} now={NOW} />);
    expect(container.querySelector("[data-testid='park-card']")).toBeNull();
  });

  it("Estimated offers Check now; a reached reset offers the same conversation", async () => {
    const { container, rerender } = render(
      <ParkCard session={session({ parked: "estimated", parked_resets_at_ms: NOW + 1 })} now={NOW} />,
    );
    expect(container.querySelector("[data-testid='park-check']")).not.toBeNull();
    expect(container.querySelector("[data-testid='park-resume']")).toBeNull();
    rerender(<ParkCard session={session({ parked: "measured", parked_resets_at_ms: NOW - 1 })} now={NOW} />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='park-resume']")!);
    });
    expect(calls()).toEqual(["resume:s1"]);
  });
});

describe("AJ-E1003 the reset offer", () => {
  it("one offer with Resume and a distinct snooze; the last one says stop", async () => {
    w().__AIJ_PARKED_OFFERS__ = [
      { session_id: "s1", account_label: "Work Max", text: "Work Max has reset. Resume the work you parked.", last: false },
    ];
    const { container, unmount } = render(<NotificationRegion />);
    await waitFor(() => expect(container.querySelector("[data-testid='parked-offer']")).not.toBeNull());
    expect(container.querySelector("[data-testid='parked-offer-snooze']")!.textContent).toBe("Remind me in 30 min");
    expect(calls()).toEqual([]);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='parked-offer-snooze']")!);
    });
    expect(calls()).toEqual(["dismiss:s1"]);
    unmount();
    w().__AIJ_PARKED_OFFERS__ = [
      { session_id: "s1", account_label: "Work Max", text: "Work Max has reset.", last: true },
    ];
    const second = render(<NotificationRegion />);
    await waitFor(() => expect(second.container.querySelector("[data-testid='parked-offer-stop']")).not.toBeNull());
    expect(second.container.querySelector("[data-testid='parked-offer-stop']")!.textContent).toBe("Stop reminding me");
    await act(async () => {
      fireEvent.click(second.container.querySelector("[data-testid='parked-offer-resume']")!);
    });
    expect(calls()).toContain("resume:s1");
  });
});

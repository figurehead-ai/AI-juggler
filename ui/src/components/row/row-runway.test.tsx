/**
 * AJ-E2104 R2: a session row shows its account's runway only when it is
 * low (25 % left or less), muted (a fact; the need's loud copy is its
 * Needs you row, D-118) and in what a screen reader hears; otherwise the row says nothing of it. Other apps' rows
 * never do: AI Juggler reads room for the sessions it runs.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SessionRow } from "./SessionRow";
import { sessionItem } from "../../lib/rail-items";
import { instance } from "../../test/fixtures";

afterEach(cleanup);

const row = (used: number) =>
  render(
    <ul>
      <li>
        <SessionRow item={sessionItem(instance({ id: `s-${used}`, status: "running", quota: { used_percentage: used, resets_at_ms: null, confidence: "exact" } }), () => "shop", false)} density="cozy" now={Date.now()} onSelect={vi.fn()} />
      </li>
    </ul>,
  );

it("says how much is left only when it's low, in its row's colour and in its name", () => {
  // D-117 (the 0.5 design call): the figure follows the row's own level,
  // not the room's: a row that is working (L0) shows it muted however
  // low; the Account's own runway line carries the Account's level.
  row(80);
  const low = screen.getByText("20% left");
  expect(low.className).toContain("text-muted");
  expect(low.getAttribute("data-level")).toBe("0");
  // What a screen reader hears of the row says it too.
  expect([...document.querySelectorAll(".sr-only")].some((words) => (words.textContent ?? "").includes("20% left"))).toBe(true);
  cleanup();
  for (const used of [92, 100]) {
    row(used);
    const figure = screen.getByText(`${100 - used}% left`);
    expect(figure.className).toContain("text-muted");
    expect(figure.className).not.toMatch(/text-(warning|danger)/);
    expect(figure.getAttribute("data-level")).toBe("0");
    cleanup();
  }
  row(50);
  expect(screen.queryByText(/% left/)).toBeNull();
});

// D-118 (one quiet mark per need, the 0.5 review of the rail): a row's
// low room is a fact, muted whatever the row's level; the need's loud copy
// is its Needs you row (an asking row's "0% left" and its limit row's were
// amber beside it).
it("a row's figure stays muted when it asks (L2), when it broke (L3) and when its limit stopped it", () => {
  const out = (over: Parameters<typeof instance>[0]) =>
    render(
      <SessionRow item={sessionItem(instance({ id: "s-out", quota: { used_percentage: 100, resets_at_ms: null, confidence: "exact" }, ...over }), () => "shop", false)} density="cozy" now={Date.now()} onSelect={vi.fn()} />,
    );
  for (const over of [{ status: "waiting_user", needs_attention: "waiting_user" }, { status: "stuck", needs_attention: "stuck" }, { status: "running", needs_attention: "failed" }] as const) {
    out(over);
    const figure = screen.getByText("0% left");
    expect(figure.className).toContain("text-muted");
    expect(figure.className).not.toMatch(/text-(warning|danger)/);
    expect(figure.getAttribute("data-level")).toBe("0");
    cleanup();
  }
  // AJ-E3603: its limit stopped it: "Limit reached" says it, once (no
  // "0% left" beside it, which cut its title).
  out({ status: "quota_blocked", needs_attention: "quota_blocked" });
  expect(screen.queryByText("0% left")).toBeNull();
  expect(screen.getByTestId("state-word").textContent).toBe("Limit reached");
  cleanup();
  // Parked, its reset reached: calm too, its word as its page's header
  // says it ("Parked · ready", AJ-E2312 1, 0.4.9).
  out({ status: "quota_blocked", needs_attention: "quota_blocked", parked: "measured", parked_resets_at_ms: Date.now() - 60_000 });
  expect(screen.getByText("0% left").className).toContain("text-muted");
  expect(screen.getByText("Parked · ready")).toBeTruthy();
});

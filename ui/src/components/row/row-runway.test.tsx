/**
 * AJ-E2104 R2: a session row shows its account's runway only when it is
 * low (25 % left or less), in its row's own level's colour (D-117, the
 * 0.5 design call) and in what a screen reader hears; otherwise the row says nothing of it. Other apps' rows
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

// D-117 (the 0.5 design call): a row's figures follow the row's own level.
// An asking or stuck session (L2) on an Account at 0 % shows "0% left" in
// amber, never red; red only when the row itself is broken (L3).
it("a row's figure takes the row's level: amber when it asks (L2), red only when it broke (L3)", () => {
  const out = (over: Parameters<typeof instance>[0]) =>
    render(
      <SessionRow item={sessionItem(instance({ id: "s-out", quota: { used_percentage: 100, resets_at_ms: null, confidence: "exact" }, ...over }), () => "shop", false)} density="cozy" now={Date.now()} onSelect={vi.fn()} />,
    );
  for (const status of ["waiting_user", "stuck"] as const) {
    out({ status, needs_attention: status });
    const figure = screen.getByText("0% left");
    expect(figure.className).toContain("text-warning");
    expect(figure.className).not.toContain("text-danger");
    expect(figure.getAttribute("data-level")).toBe("2");
    cleanup();
  }
  // Broken (L3): the one place its figure is red.
  out({ status: "running", needs_attention: "failed" });
  const broken = screen.getByText("0% left");
  expect(broken.className).toContain("text-danger");
  expect(broken.getAttribute("data-level")).toBe("3");
});

// D-117 (AJ-E3602), the inspector-folded captures: a "Limit reached" row
// (L2) read "0% left" in red. A session its limit stopped: its figure is
// that limit, at the row's own level (attentionLevel(), the one rule):
// amber at L2, never red; calm while it is parked until its reset.
it("a row its limit stopped shows its figure at the row's level: amber at L2, never red", () => {
  const limited = (over: Parameters<typeof instance>[0] = {}) =>
    render(
      <SessionRow item={sessionItem(instance({ id: "s-limit", status: "quota_blocked", needs_attention: "quota_blocked", quota: { used_percentage: 100, resets_at_ms: null, confidence: "exact" }, ...over }), () => "shop", false)} density="cozy" now={Date.now()} onSelect={vi.fn()} />,
    );
  for (const used of [100, 95, 80]) {
    limited({ quota: { used_percentage: used, resets_at_ms: null, confidence: "exact" } });
    expect(document.querySelector("[data-need='limit']")?.getAttribute("data-level")).toBe("2");
    const shown = screen.getByText(`${100 - used}% left`);
    expect(shown.className).not.toContain("text-danger");
    expect(shown.className).toContain("text-warning");
    expect(shown.getAttribute("data-level")).toBe("2");
    cleanup();
  }
  // Parked until its reset: calm (L0), its figure muted, its word "Parked".
  limited({ parked: "measured", parked_resets_at_ms: Date.now() - 60_000 });
  const figure = screen.getByText("0% left");
  expect(figure.className).toContain("text-muted");
  expect(figure.getAttribute("data-level")).toBe("0");
  expect(screen.getByText("Parked")).toBeTruthy();
});

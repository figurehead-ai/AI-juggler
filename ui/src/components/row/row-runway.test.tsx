/**
 * AJ-E2104 R2: a session row shows its account's runway only when it is
 * low (25 % left or less), in the warning colour and in what a screen
 * reader hears; otherwise the row says nothing of it. Other apps' rows
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

it("says how much is left only when it's low, in the warning colour and in its name", () => {
  row(80);
  const low = screen.getByText("20% left");
  expect(low.className).toContain("text-warning");
  // What a screen reader hears of the row says it too.
  expect([...document.querySelectorAll(".sr-only")].some((words) => (words.textContent ?? "").includes("20% left"))).toBe(true);
  cleanup();
  row(50);
  expect(screen.queryByText(/% left/)).toBeNull();
});

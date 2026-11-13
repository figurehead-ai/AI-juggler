/**
 * ADR-0032 amendment 2 (0.4.4): by Account, a pinned session that ended
 * shows pinned under its Account, as by App: after the open rows of an
 * Account with something open, and under its Account in "Pinned" when
 * nothing of it is open (pins never mean open). It showed nowhere.
 */
import { cleanup, render, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { resetSeen } from "../lib/seen";
import { needsFor } from "../test/needs-kit";
import { instance, setSeam } from "../test/fixtures";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
const H = 3_600_000;
const ended = { status: "finished", pinned: true, last_activity_ms: NOW - 5 * H } as const;

/** Work has one open and one pinned that ended; Side only a pinned one that ended. */
const SESSIONS: InstanceInfo[] = [
  instance({ id: "work-open", account_id: "a-work", account_label: "Work", status: "running", last_activity_ms: NOW - 60_000 }),
  instance({ id: "work-pinned", account_id: "a-work", account_label: "Work", ...ended }),
  instance({ id: "side-pinned", account_id: "a-side", account_label: "Side", ...ended }),
];

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  for (const seam of Object.keys(window).filter((found) => found.startsWith("__AIJ_"))) Reflect.deleteProperty(window, seam);
  resetRailViewCache();
});

it("by Account, an ended pinned session is under its Account: in Open now after its open rows, else in Pinned", async () => {
  setRailView({ ...DEFAULT_VIEW, group: "account" });
  setSeam("__AIJ_FIXTURE__", { conversations: [] });
  setSeam("__AIJ_PLACES__", { places: [] });
  needsFor({ sessions: SESSIONS });
  const view = render(
    <ConversationsProvider>
      <InstanceBoard sessions={SESSIONS} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={[]} snooze={{}} onSnooze={() => {}} />
    </ConversationsProvider>,
  );
  const list = await view.findByTestId("place-board");
  const lines = [...list.querySelectorAll<HTMLElement>("[data-rail-line]")];
  const at = (selector: string) => lines.findIndex((line) => line.querySelector(selector) !== null);
  const rowAt = (id: string) => at(`[data-rail-item="s:${id}"]`);
  // Work: Open now, its open row, then its pinned one.
  expect(within(view.getByTestId("rail-open-now")).getByRole("button").textContent).toContain("Open now1");
  expect(rowAt("work-open")).toBeGreaterThan(at("[data-testid='rail-open-now']"));
  expect(rowAt("work-pinned")).toBe(rowAt("work-open") + 1);
  // Side: never in Open now; under its own header in Pinned, which says nothing is open.
  const pinned = at("[data-testid='rail-pinned']");
  expect(view.getByTestId("rail-pinned").textContent).toContain("Pinned1");
  expect(pinned).toBeGreaterThan(rowAt("work-pinned"));
  const side = lines[pinned + 1]!;
  expect(within(side).getByTestId("place-page").textContent).toContain("Side");
  expect(within(side).getByTestId("place-nothing-open")).toBeTruthy();
  expect(rowAt("side-pinned")).toBe(pinned + 2);
});

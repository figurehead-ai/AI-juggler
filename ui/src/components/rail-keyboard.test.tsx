/**
 * The 0.2 walk, F11 — 84 Tabs to get past the rail: every row was four
 * stops (the row, its project, Go to it, ⋯). deep/11 §9: the list is one
 * stop on the Tab path (roving tabindex), its row's own controls after
 * it, and ↑ ↓ Home End move between the lines inside.
 */
import { needsFor } from "../test/needs-kit";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { linesOf } from "../lib/roving";
import { placesConversations } from "../preview/places";
import type { ProjectInfo } from "../lib/bridge";

const NOW = new Date(2026, 9, 28, 10, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown };
const seams = () => window as unknown as Seams;
const projects = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler" }] as ProjectInfo[];

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  resetRailViewCache();
  resetRailOrderCache();
});

function board(selectedId: string | null = null) {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  needsFor({ sessions: [], conversations: placesConversations(NOW), projects });
  const view = render(
    <ConversationsProvider>
      <InstanceBoard sessions={[]} loginTerminals={[]} selectedId={selectedId} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} />
    </ConversationsProvider>,
  );
  const list = view.getByTestId("rail-list");
  /** The controls on the Tab path inside the list. */
  const tabbable = () => [...list.querySelectorAll<HTMLElement>("button, [tabindex]")].filter((element) => element.tabIndex >= 0 && !(element as HTMLButtonElement).disabled);
  return { view, list, tabbable };
}

describe("the rail's list on the Tab path", () => {
  it("is one line: the first, its row's controls after it; every other row is out of the Tab path", async () => {
    const { list, tabbable } = board();
    await waitFor(() => expect(list.querySelectorAll("[data-rail-item]").length).toBeGreaterThan(5));
    await waitFor(() => expect(new Set(tabbable().map((element) => element.closest("[data-rail-row], [data-rail-line]"))).size).toBe(1));
    const lines = linesOf(list);
    expect(lines.length).toBeGreaterThan(8);
    // All of them in the first line, and it is the first line.
    expect(tabbable().every((element) => lines[0]!.line.contains(element))).toBe(true);
    // Before: a row and its project, Go to it and ⋯ each a stop; now the
    // whole list is at most one line's few.
    expect(tabbable().length).toBeLessThanOrEqual(4);
  });

  it("↓ and ↑ move between lines, and the line on the Tab path follows", async () => {
    const { list, tabbable } = board();
    await waitFor(() => expect(list.querySelectorAll("[data-rail-item]").length).toBeGreaterThan(5));
    const firstRow = list.querySelector<HTMLElement>("[data-rail-item]")!;
    firstRow.focus();
    fireEvent.keyDown(firstRow, { key: "ArrowDown" });
    const next = document.activeElement as HTMLElement;
    expect(next).not.toBe(firstRow);
    expect(list.contains(next)).toBe(true);
    await waitFor(() => expect(tabbable().every((element) => (next.closest("[data-rail-row], [data-rail-line]") ?? next).contains(element))).toBe(true));
    fireEvent.keyDown(next, { key: "ArrowUp" });
    expect(document.activeElement).toBe(firstRow);
    // From a row's own control (its ⋯), ↓ goes on to the next line too.
    const menu = firstRow.closest("[data-rail-row]")!.querySelector<HTMLElement>("button[aria-haspopup]");
    if (menu) {
      menu.focus();
      fireEvent.keyDown(menu, { key: "ArrowDown" });
      expect(document.activeElement).toBe(next);
    }
    fireEvent.keyDown(document.activeElement!, { key: "End" });
    const lines = linesOf(list);
    expect(lines[lines.length - 1]!.line.contains(document.activeElement)).toBe(true);
  });

  it("the open page's row is the one on the Tab path", async () => {
    const { list, tabbable } = board("outside-3");
    await waitFor(() => expect(list.querySelector("[data-rail-selected='true']")).not.toBeNull());
    await waitFor(() => expect(tabbable()[0]?.getAttribute("data-rail-selected")).toBe("true"));
  });
});

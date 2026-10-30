/**
 * The 0.2 walk, F18 — the View's words: "Sub-agents" where everything
 * else says "Subagents"; "Quiet ones" and "Inactive" with nothing on the
 * difference; and "An app" offered as an app to pick.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { ViewPopover } from "./ViewPopover";
import { onlyOptions } from "./RailToolbar";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { placesConversations } from "../preview/places";

afterEach(() => {
  cleanup();
  localStorage.clear();
  resetRailViewCache();
});

it("says Subagents, and what Inactive, Quiet ones and Background runs are", () => {
  setRailView(DEFAULT_VIEW);
  const view = render(<ViewPopover options={{ app: [], profile: [], account: [], project: [], mac: [] }} />);
  fireEvent.click(view.getByTestId("rail-view-button"));
  const panel = view.getByTestId("rail-view");
  const note = (key: string) => document.getElementById(view.getByTestId(`rail-view-show-${key}`).getAttribute("aria-describedby") ?? "")?.textContent;
  // Named by its label alone; what it is, as its description.
  expect(within(panel).getByRole("menuitemcheckbox", { name: "Subagents" })).toBeTruthy();
  expect(note("subagents")).toBe("a count on each session");
  expect(note("inactive")).toBe("in an app's list, not running");
  expect(note("quiet")).toBe("idle ones, folded away");
  expect(note("background")).toBe("from scripts");
  expect(within(panel).queryByText("Sub-agents")).toBeNull();
});

it("draws every Show line one height: a hint sits on its label's line (0.3 polish)", () => {
  setRailView(DEFAULT_VIEW);
  const view = render(<ViewPopover options={{ app: [], profile: [], account: [], project: [], mac: [] }} />);
  fireEvent.click(view.getByTestId("rail-view-button"));
  for (const key of ["inactive", "quiet", "background", "subagents"]) {
    const line = view.getByTestId(`rail-view-show-${key}`);
    const label = line.querySelector("[data-choice-label]")!;
    // Label and hint side by side in one row, never a column.
    expect(label.parentElement!.className).toContain("items-baseline");
    expect(label.parentElement!.className).not.toContain("flex-col");
  }
  // A line without a hint has the same one row.
  expect(view.getByTestId("rail-view-show-ended").querySelectorAll("[data-choice-label] ~ span")).toHaveLength(0);
});

it("never offers \"An app\" as an app to pick", () => {
  const conversations = placesConversations(Date.now()).map((c, index) => (index === 0 ? { ...c, places: [], live: true } : c));
  const apps = onlyOptions([], [], conversations, []).app;
  expect(apps.map((option) => option.label)).not.toContain("An app");
  expect(apps.some((option) => option.label === "Not open in an app")).toBe(true);
});

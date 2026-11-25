/**
 * The 0.2 walk, F18 — the View's words: "Sub-agents" where everything
 * else says "Subagents"; "Quiet ones" and "Inactive" with nothing on the
 * difference; and "An app" offered as an app to pick.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { cliAppName } from "../lib/app-name";
import { afterEach, expect, it } from "vitest";
import { ViewPopover } from "./ViewPopover";
import { onlyOptions } from "./RailToolbar";
import { DEFAULT_VIEW, parseView, resetRailViewCache, setRailView, SHOW_KEYS, viewChanged } from "../lib/rail-view";
import { placesConversations } from "../preview/places";

afterEach(() => {
  cleanup();
  localStorage.clear();
  resetRailViewCache();
});

it("says what Inactive, Quiet ones and Background runs are; no Subagents switch (0.5.1, F10)", () => {
  setRailView(DEFAULT_VIEW);
  const view = render(<ViewPopover options={{ app: [], profile: [], account: [], project: [], pool: [], mac: [] }} />);
  fireEvent.click(view.getByTestId("rail-view-button"));
  const panel = view.getByTestId("rail-view");
  const note = (key: string) => document.getElementById(view.getByTestId(`rail-view-show-${key}`).getAttribute("aria-describedby") ?? "")?.textContent;
  // Named by its label alone; what it is, as its description.
  // Every row shows its running subagents now: a switch would change nothing (D-127).
  expect(within(panel).queryByRole("menuitemcheckbox", { name: "Subagents" })).toBeNull();
  expect(view.queryByTestId("rail-view-show-subagents")).toBeNull();
  expect(note("inactive")).toBe("in an app's list, not running");
  expect(note("quiet")).toBe("idle ones, folded away");
  expect(note("background")).toBe("from scripts");
  expect(within(panel).queryByText("Sub-agents")).toBeNull();
});

it("ignores a Subagents switch kept by an older version: the rest of the view is read as it was (F10)", () => {
  const view = parseView(JSON.stringify({ group: "project", show: { subagents: true, quiet: true }, only: {} }));
  expect(view.group).toBe("project");
  expect(view.show).toEqual({ ...DEFAULT_VIEW.show, quiet: true });
  expect("subagents" in view.show).toBe(false);
  // Nothing of it counts as a changed view (the View's dot, its Reset).
  expect(viewChanged(parseView(JSON.stringify({ show: { subagents: true } })))).toBe(false);
});

it("draws every Show line one height: a hint sits on its label's line (0.3 polish)", () => {
  setRailView(DEFAULT_VIEW);
  const view = render(<ViewPopover options={{ app: [], profile: [], account: [], project: [], pool: [], mac: [] }} />);
  fireEvent.click(view.getByTestId("rail-view-button"));
  for (const key of SHOW_KEYS) {
    const line = view.getByTestId(`rail-view-show-${key}`);
    const label = line.querySelector("[data-choice-label]")!;
    // Label and hint side by side in one row, never a column.
    expect(label.parentElement!.className).toContain("items-baseline");
    expect(label.parentElement!.className).not.toContain("flex-col");
  }
});

it("never offers \"An app\" as an app to pick: a session no app shows is in its App (D-62)", () => {
  const conversations = placesConversations(Date.now()).map((c, index) => (index === 0 ? { ...c, places: [], live: true } : c));
  const apps = onlyOptions([], [], conversations, []).app;
  expect(apps.map((option) => option.label)).not.toContain("An app");
  expect(apps.map((option) => option.label)).toContain(cliAppName(conversations[0]!.tool, conversations[0]!.account_label));
});

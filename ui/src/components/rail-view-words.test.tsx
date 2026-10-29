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
  expect(within(panel).getByRole("checkbox", { name: "Subagents" })).toBeTruthy();
  expect(note("subagents")).toBe("how many each session runs");
  expect(note("inactive")).toBe("in an app's list, not running");
  expect(note("quiet")).toBe("idle rows a place folds away");
  expect(note("background")).toBe("headless runs, from scripts");
  expect(within(panel).queryByText("Sub-agents")).toBeNull();
});

it("never offers \"An app\" as an app to pick", () => {
  const conversations = placesConversations(Date.now()).map((c, index) => (index === 0 ? { ...c, places: [], live: true } : c));
  const apps = onlyOptions([], [], conversations, []).app;
  expect(apps.map((option) => option.label)).not.toContain("An app");
  expect(apps.some((option) => option.label === "Not open in an app")).toBe(true);
});

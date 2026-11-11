/**
 * 0.3 — the rail's grouping as a visible switch (App · Project ·
 * Account, Pool once a pool of yours exists), and the View rebuilt as a
 * real menu: a View header with Reset only when something changed,
 * Show lines that turn on and off, Only lines that open a submenu of
 * values — never a system select, never a radio for the grouping.
 */
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { GroupSwitch } from "./RailToolbar";
import { ViewPopover } from "./ViewPopover";
import { DEFAULT_VIEW, railView, resetRailViewCache, setRailView, SHOW_KEYS, SHOW_LABELS, SHOW_NOTES } from "../lib/rail-view";
import { resetPools, setPools } from "../lib/pools";
import { scopeOf } from "../lib/ipc/pools";
import { resetToasts, toastEntries, undoLatest } from "./Toast";
import type { PoolInfo } from "../types/generated/PoolInfo";

const pool = (id: string, kind: string, members = 0): PoolInfo => ({
  id,
  name: id,
  kind,
  project_id: kind === "default" ? "p1" : null,
  access: { level: "work_together" } as PoolInfo["access"],
  rules: [],
  spans_macs: false,
  scopes: 1,
  members,
  ...scopeOf(kind, kind === "default" ? "p1" : null),
});

beforeEach(() => {
  localStorage.clear();
  resetRailViewCache();
  resetPools();
  setRailView(DEFAULT_VIEW);
});
afterEach(cleanup);

describe("the grouping switch", () => {
  it("offers App, Project and Account; one click groups, kept per Mac", () => {
    render(<GroupSwitch />);
    const group = screen.getByRole("radiogroup", { name: "Group sessions by" });
    const names = within(group).getAllByRole("radio").map((radio) => radio.textContent);
    expect(names).toEqual(["App", "Project", "Account"]);
    expect(within(group).getByRole("radio", { name: "App" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(within(group).getByRole("radio", { name: "Project" }));
    expect(railView().group).toBe("project");
    expect(JSON.parse(localStorage.getItem("aij.rail.view.v1")!).group).toBe("project");
    // The arrows move it too.
    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(railView().group).toBe("account");
  });

  it("offers Pool once a pool of yours has members (a project's own pool, or an empty one, doesn't count)", () => {
    act(() => setPools({ pools: [pool("p1-default", "default", 3)], placements: [] }));
    render(<GroupSwitch />);
    expect(screen.queryByRole("radio", { name: "Pool" })).toBeNull();
    act(() => setPools({ pools: [pool("p1-default", "default", 3), pool("checkout", "user")], placements: [] }));
    expect(screen.queryByRole("radio", { name: "Pool" })).toBeNull();
    act(() => setPools({ pools: [pool("p1-default", "default", 3), pool("checkout", "user", 2)], placements: [] }));
    fireEvent.click(screen.getByRole("radio", { name: "Pool" }));
    expect(railView().group).toBe("pool");
  });

  it("shows App while Pool is kept with no pool of yours", () => {
    setRailView({ ...DEFAULT_VIEW, group: "pool" });
    render(<GroupSwitch />);
    expect(screen.getByRole("radio", { name: "App" }).getAttribute("aria-checked")).toBe("true");
  });
});

describe("the View menu", () => {
  const options = {
    app: [
      { value: "Claude", label: "Claude" },
      { value: "Terminal", label: "Terminal" },
    ],
    profile: [],
    account: [
      { value: "Work", label: "Work" },
      { value: "Personal", label: "Personal" },
    ],
    project: [],
    pool: [],
    mac: [],
  };

  it("is a menu: Show lines turn on and off, Only opens a submenu, Reset shows only when changed", () => {
    render(<ViewPopover options={options} />);
    fireEvent.click(screen.getByTestId("rail-view-button"));
    const menu = screen.getByRole("menu", { name: "View" });
    expect(menu.querySelector("select, input")).toBeNull();
    expect(within(menu).queryByTestId("rail-view-reset")).toBeNull();
    // Focus opens on the first Show line, not on Reset or the header.
    expect(document.activeElement?.getAttribute("data-testid")).toBe("rail-view-show-inactive");
    const quiet = within(menu).getByRole("menuitemcheckbox", { name: "Quiet ones" });
    fireEvent.click(quiet);
    expect(railView().show.quiet).toBe(true);
    expect(quiet.getAttribute("aria-checked")).toBe("true");
    // The menu stays open for the next one.
    expect(screen.getByRole("menu", { name: "View" })).toBeTruthy();
    // Only › Account › Work.
    fireEvent.click(within(menu).getByTestId("rail-view-only-account"));
    const sub = screen.getByRole("menu", { name: "Only this account" });
    expect(within(sub).getByRole("menuitemradio", { name: "All" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(within(sub).getByRole("menuitemradio", { name: "Work" }));
    expect(railView().only.account).toBe("Work");
    expect(within(menu).getByTestId("rail-view-only-account").textContent).toContain("Work");
    // Reset puts Show and Only back, and keeps the grouping.
    act(() => setRailView((view) => ({ ...view, group: "project" })));
    resetToasts();
    fireEvent.click(within(menu).getByTestId("rail-view-reset"));
    expect(railView()).toEqual({ ...DEFAULT_VIEW, group: "project" });
    expect(within(menu).queryByTestId("rail-view-reset")).toBeNull();
    // with Undo, as Reset my order has (AJ-E2802 R5)
    expect(toastEntries().map((t) => t.options.message)).toEqual(["The View is reset."]);
    act(() => void undoLatest());
    expect(railView().show.quiet).toBe(true);
    expect(railView().only.account).toBe("Work");
  });

  it("gives every Show line its hint (AJ-E2802 R4)", () => {
    render(<ViewPopover options={options} />);
    fireEvent.click(screen.getByTestId("rail-view-button"));
    const lines = within(screen.getByRole("menu", { name: "View" })).getAllByRole("menuitemcheckbox");
    expect(lines.length).toBe(SHOW_KEYS.length);
    for (const key of SHOW_KEYS) {
      const note = SHOW_NOTES[key];
      expect(note, key).toBeTruthy();
      expect(within(screen.getByRole("menu", { name: "View" })).getByRole("menuitemcheckbox", { name: new RegExp(SHOW_LABELS[key]) }).textContent).toContain(note);
    }
  });

  it("no Only line for a list with one value, and Escape closes it back to its button", () => {
    render(<ViewPopover options={{ ...options, account: [{ value: "Work", label: "Work" }] }} />);
    const button = screen.getByTestId("rail-view-button");
    fireEvent.click(button);
    expect(screen.queryByTestId("rail-view-only-account")).toBeNull();
    expect(screen.getByTestId("rail-view-only-app")).toBeTruthy();
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(button);
  });
});

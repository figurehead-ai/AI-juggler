/**
 * AJ-E2901 vitest — ⌘K as a real dialog: named, modal, Tab kept inside,
 * Esc closes, focus back to the invoker; nothing typed shows Recent and
 * the top actions; typed, the groups in order, loosely matched, capped
 * at 5 with "n more", the matched letters bold; → previews and ← goes
 * back; See all results searches History; no matches leaves Search
 * History active; offline says so.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GoToPalette } from "./GoToPalette";
import { ActionHostProvider } from "./ItemMenu";
import type { ActionHost } from "../lib/actions";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { AppDoings } from "../lib/palette-commands";
import { rememberRecent } from "../lib/palette-recents";
import { resetBackendHealth } from "../lib/backend-health";

const NOW = Date.now();
const session = (id: string, title: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({ id, provider: "claude-code", account_id: "a1", account_label: "Work Max", status: "running", kind: "terminal", git_branch: null, cwd: "/w/alpha", project_id: null, conversation_id: null, quota: null, label: title, last_activity_ms: NOW, ...overrides }) as InstanceInfo;

const SESSIONS = [session("s-bill", "Fix billing export"), session("s-dash", "Billing dashboard", { account_label: "Personal" }), session("s-notes", "Write release notes")];
const PROJECTS = ["alpha", "alpine", "alps", "altair", "alto", "alamo"].map((name, index) => ({ id: `p${index}`, name, root: `/w/${name}`, missing: false }) as ProjectInfo);

function doings() {
  return { openScreen: vi.fn(), newSession: vi.fn(), newProject: vi.fn(), notifications: vi.fn(), shortcuts: vi.fn(), toggleRail: vi.fn(), searchHistory: vi.fn<(words: string) => void>(), fleet: false } satisfies AppDoings;
}

function openPalette({ host = {} as ActionHost, onClose = vi.fn(), app = doings() } = {}) {
  const view = render(
    <ActionHostProvider host={host}>
      <button type="button" data-testid="invoker">⌘K</button>
      <GoToPalette open={false} onClose={onClose} sessions={SESSIONS} onFocusWindow={vi.fn()} onOpenSession={vi.fn()} hotkey="" onHotkeyChange={async () => {}} projects={PROJECTS} app={app} />
    </ActionHostProvider>,
  );
  const invoker = view.getByTestId("invoker");
  invoker.focus();
  const rerender = (open: boolean) =>
    view.rerender(
      <ActionHostProvider host={host}>
        <button type="button" data-testid="invoker">⌘K</button>
        <GoToPalette open={open} onClose={onClose} sessions={SESSIONS} onFocusWindow={vi.fn()} onOpenSession={vi.fn()} hotkey="" onHotkeyChange={async () => {}} projects={PROJECTS} app={app} />
      </ActionHostProvider>,
    );
  rerender(true);
  const input = () => view.container.querySelector<HTMLInputElement>("input[role='combobox']")!;
  const groups = () => [...view.container.querySelectorAll<HTMLElement>("ul[role='group']")].map((group) => group.getAttribute("aria-label"));
  return { view, input, groups, rerender, invoker, onClose, app };
}

beforeEach(() => localStorage.clear());
afterEach(() => {
  cleanup();
  localStorage.clear();
  resetBackendHealth();
});

describe("AJ-E2901 the dialog", () => {
  it("is a named modal dialog; focus goes in, Tab stays inside, and closing returns focus to the invoker", () => {
    const { view, input, rerender, invoker } = openPalette();
    const dialog = view.getByRole("dialog", { name: "Search, act or go" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(document.activeElement).toBe(input());
    const buttons = within(dialog).getAllByRole("button");
    buttons.at(-1)!.focus();
    fireEvent.keyDown(buttons.at(-1)!, { key: "Tab" });
    expect(document.activeElement).toBe(input());
    rerender(false);
    expect(document.activeElement).toBe(invoker);
  });

  it("Esc closes it", () => {
    const { input, onClose } = openPalette();
    fireEvent.keyDown(input(), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("AJ-E2901 before typing", () => {
  it("shows Recent (ids only, still here) and the top actions, then what's open", () => {
    rememberRecent({ kind: "session", id: "s-notes" });
    rememberRecent({ kind: "session", id: "gone" });
    rememberRecent({ kind: "project", id: "p1" });
    const { view, groups } = openPalette();
    expect(groups()).toEqual(["Recent", "Actions", "Open now"]);
    const recent = view.getByTestId("goto-group-recent");
    expect([...recent.querySelectorAll("[role='option']")].map((row) => row.getAttribute("data-testid"))).toEqual(["goto-row-project::p1", "goto-row-s-notes"]);
    const actions = view.getByTestId("goto-group-actions");
    expect(actions.textContent).toContain("New session…");
    expect(actions.textContent).toContain("Settings");
  });

  it("a first run (no recents) shows the top actions only, before what's open", () => {
    const { groups } = openPalette();
    expect(groups()).toEqual(["Actions", "Open now"]);
  });
});

describe("AJ-E2901 typing", () => {
  it("groups in order, loosely matched, the matched letters bold", () => {
    const { view, input, groups } = openPalette();
    fireEvent.change(input(), { target: { value: "wrk bil" } });
    expect(groups()[0]).toBe("Sessions");
    const first = view.getByTestId("goto-group-sessions").querySelector("[role='option']")!;
    expect(first.getAttribute("data-testid")).toBe("goto-row-s-bill");
    expect([...first.querySelectorAll("b[data-hit]")].map((bold) => bold.textContent)).toEqual(["bil"]);
    expect(groups().at(-1)).toBe("Search");
  });

  it("a row named by what's typed starts active, even after a looser match in an earlier group", () => {
    const { input } = openPalette();
    fireEvent.change(input(), { target: { value: "alt" } });
    expect(input().getAttribute("aria-activedescendant")).toBe("goto-option-project__p3");
  });

  it("caps a group at 5 and says how many more", () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "al" } });
    const projects = view.getByTestId("goto-group-projects");
    expect(projects.querySelectorAll("[role='option']")).toHaveLength(5);
    expect(view.getByTestId("goto-more-projects").textContent).toBe("· 1 more");
  });

  it("finds settings too, and opens one at its section", () => {
    const opened = vi.fn();
    window.addEventListener("aij:open-settings", opened);
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "quiet hours" } });
    const row = view.getByTestId("goto-group-settings").querySelector("[role='option']")!;
    fireEvent.click(row);
    expect((opened.mock.calls[0]![0] as CustomEvent).detail).toEqual({ section: "notifications" });
    window.removeEventListener("aij:open-settings", opened);
  });

  it("→ previews the active session beside the list; ← goes back", () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "billing export" } });
    input().setSelectionRange(14, 14);
    fireEvent.keyDown(input(), { key: "ArrowRight" });
    const preview = view.getByTestId("goto-preview");
    expect(preview.getAttribute("aria-label")).toBe("Preview of Fix billing export");
    expect(preview.textContent).toContain("Work Max");
    expect(input().getAttribute("aria-controls")).toBe("goto-listbox goto-preview");
    fireEvent.keyDown(input(), { key: "ArrowLeft" });
    expect(view.queryByTestId("goto-preview")).toBeNull();
  });

  it("See all results searches History for the words", () => {
    const { view, input, app } = openPalette();
    fireEvent.change(input(), { target: { value: "billing" } });
    fireEvent.click(view.getByTestId("goto-see-all"));
    expect(app.searchHistory).toHaveBeenCalledWith("billing");
  });

  it("no matches: says so, with Search History for it active", () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "qqqzz" } });
    expect(view.getByTestId("goto-no-matches").textContent).toBe("No matches for “qqqzz”.");
    expect(input().getAttribute("aria-activedescendant")).toBe("goto-option-command__search-history");
  });

  it("choosing remembers it (ids only)", () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "alpha" } });
    fireEvent.click(view.getByTestId("goto-row-project::p0"));
    expect(JSON.parse(localStorage.getItem("aij.palette.recents")!)).toEqual([{ kind: "project", id: "p0" }]);
  });

  it("offline: says it shows what was open last", () => {
    (window as unknown as Record<string, unknown>).__AIJ_OFFLINE__ = { since_ms: NOW };
    try {
      const { view } = openPalette();
      expect(view.getByTestId("goto-offline").textContent).toBe("AI Juggler's service isn't answering — showing what was open last.");
    } finally {
      delete (window as unknown as Record<string, unknown>).__AIJ_OFFLINE__;
    }
  });
});

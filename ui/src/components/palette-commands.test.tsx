/**
 * The 0.2 walk, F7 — ⌘K had no actions: "Go to session…" and live
 * sessions; typing "switch" found only folders with the word; a session
 * another app runs had none; screen readers heard "terminal" before every
 * app's session. deep/11 §4.17: actions first (the selected item's, then
 * App's: screens, New session…, notifications…), filtered by what's
 * typed, then sessions and projects, and a search when nothing matches.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GoToPalette, paletteSelection } from "./GoToPalette";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { appCommands, commandMatches, commandsFor, type AppDoings } from "../lib/palette-commands";
import { placesConversations } from "../preview/places";
import type { ActionHost } from "../lib/actions";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

type Seams = { __AIJ_FIXTURE__?: unknown };
const seams = () => window as unknown as Seams;
afterEach(() => {
  cleanup();
  delete seams().__AIJ_FIXTURE__;
});

const NOW = Date.now();
const live = (overrides: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({ id: "s1", provider: "claude-code", account_id: "a1", account_label: "Work Max", status: "running", kind: "terminal", git_branch: "login-fix", cwd: "/w/alpha", project_id: "p1", conversation_id: "conv-1", quota: null, label: null, ...overrides }) as InstanceInfo;
const PROJECTS = [{ id: "p1", name: "alpha", root: "/w/alpha", missing: false } as ProjectInfo];

function doings(fleet = false): AppDoings {
  return { openScreen: vi.fn(), newSession: vi.fn(), newProject: vi.fn(), notifications: vi.fn(), shortcuts: vi.fn(), toggleRail: vi.fn(), searchHistory: vi.fn(), fleet };
}

function open({ selected = null as InstanceInfo | null, conversation = null as { accountId: string; id: string } | null, app = doings(), host = {} as ActionHost } = {}) {
  const onClose = vi.fn();
  const view = render(
    <ActionHostProvider host={host}>
      <ConversationsProvider>
        <GoToPalette open onClose={onClose} sessions={[live({ id: "s2", git_branch: "billing" })]} onFocusWindow={() => {}} onOpenSession={() => {}} hotkey="" onHotkeyChange={async () => {}} selected={paletteSelection(selected, null, null, null)} selectedConversation={conversation} projects={PROJECTS} app={app} />
      </ConversationsProvider>
    </ActionHostProvider>,
  );
  const input = view.container.querySelector("input")!;
  const options = () => [...view.container.querySelectorAll<HTMLElement>("li[role='option']")];
  const type = (text: string) => fireEvent.change(input, { target: { value: text } });
  return { view, input, options, type, app, onClose };
}

describe("⌘K's own actions", () => {
  it("before any typing: New session… among the actions, then the sessions", () => {
    const { options } = open();
    const ids = options().map((option) => option.getAttribute("data-testid"));
    expect(ids[0]).toBe("goto-row-command::new-session");
    expect(ids).toContain("goto-row-s2");
  });

  it("typing names a screen: it comes first, with its key, and Enter goes there", () => {
    const { options, type, input, app, onClose } = open();
    type("usage");
    expect(options()[0]!.textContent).toContain("Go to Usage");
    expect(options()[0]!.querySelector("kbd")?.textContent).toBe("⌘⇧U");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(app.openScreen).toHaveBeenCalledWith("usage");
    expect(onClose).toHaveBeenCalled();
  });

  it("\"switch\" finds the selected session's Continue on another account…, and Accounts", () => {
    const host: ActionHost = { continueSession: vi.fn(), openSession: vi.fn() };
    const { options, type } = open({ selected: live(), host });
    type("switch");
    const names = options().map((option) => option.textContent ?? "");
    expect(names[0]).toContain("Continue on another account…");
    expect(names.some((name) => name.includes("Go to Accounts"))).toBe(true);
    fireEvent.click(options()[0]!);
    expect(host.continueSession).toHaveBeenCalled();
  });

  it("anything typed can be searched for in History, last", () => {
    const { options, type, app } = open();
    type("webhook retries");
    const last = options().at(-1)!;
    expect(last.textContent).toContain("Search History for “webhook retries”");
    fireEvent.click(last);
    expect(app.searchHistory).toHaveBeenCalledWith("webhook retries");
  });

  it("a session another app runs has its actions too", () => {
    const conversations = placesConversations(NOW);
    seams().__AIJ_FIXTURE__ = { conversations };
    const host: ActionHost = { goToConversation: vi.fn(), openConversation: vi.fn() };
    const { view, input } = open({ conversation: { accountId: "claude-work", id: "outside-1" }, host });
    return vi.waitFor(() => {
      expect(view.container.querySelector("[data-testid='goto-row-action::go-to']")).not.toBeNull();
      expect(input.getAttribute("aria-label")).toBe("Actions for Refactor the settings screen, or search and go");
    });
  });

  it("an option's name is what it is and where, never \"terminal\" first", () => {
    const { options } = open();
    const session = options().find((option) => option.getAttribute("data-testid") === "goto-row-s2")!;
    expect(session.textContent!.startsWith("billing")).toBe(true);
    expect(session.querySelector(".sr-only")).toBeNull();
  });
});

describe("App's actions as data", () => {
  it("lists every screen once, Fleet only while it shows, with the sheet's keys", () => {
    const without = appCommands(doings(false)).map((command) => command.label);
    expect(without).toEqual(expect.arrayContaining(["Go to Home", "Go to Accounts", "Go to Usage", "Go to History", "Go to Processes", "Go to This Mac", "Go to Settings", "Go to Privacy", "New session…", "New project…", "Notifications", "Keyboard shortcuts", "Show or hide the list"]));
    expect(without).not.toContain("Go to Fleet");
    expect(appCommands(doings(true)).map((command) => command.label)).toContain("Go to Fleet");
    const keys = Object.fromEntries(appCommands(doings(false)).map((command) => [command.id, command.keys]));
    expect(keys).toMatchObject({ "go:home": "⌘0", "go:settings": "⌘,", notifications: "⌥⌘N", "new-session": "⌘N", "new-project": "⌘⇧N", "toggle-rail": "⌘B", shortcuts: "?" });
  });

  it("matches every word typed, in its label or its other words", () => {
    const [accounts] = appCommands(doings()).filter((command) => command.id === "go:accounts");
    expect(commandMatches(accounts!, "switch")).toBe(true);
    expect(commandMatches(accounts!, "switch usage")).toBe(false);
    expect(commandsFor(appCommands(doings()), "", doings()).map((command) => command.id)).toEqual(["new-session"]);
  });
});

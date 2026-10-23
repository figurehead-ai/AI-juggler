/**
 * AJ-E1537 — one action registry, three triggers. The ⋯ button, a
 * right-click and Shift+F10 open the same items in the same order (the
 * native menu gets that list too); destructive items come last in their
 * own group; a disabled item says why and never runs; each type's menu
 * is pinned here; the menu is a role=menu that gives focus back.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX } from "react";
import { actionSpecs, resolveActions, type ActionHost, type ItemKind, type ItemOf } from "../lib/actions";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import { PREVIEW_TOOLS } from "../preview/tools";
import { ActionHostProvider, useItemMenu } from "./ItemMenu";

afterEach(cleanup);

const session = (overrides: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id: "s1",
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    status: "running",
    kind: "terminal",
    git_branch: "main",
    cwd: "/w/alpha",
    project_id: "p1",
    conversation_id: "c-1",
    quota: null,
    ...overrides,
  }) as InstanceInfo;
const project: ProjectInfo = { id: "p1", root: "/w/alpha", name: "alpha", missing: false } as ProjectInfo;
const account: AccountInfo = { id: "a1", label: "Work Max", is_default: false } as AccountInfo;
const tree = { session_id: null, place: "desktop" } as ItemOf["this-mac"];

/** A host where every capability is there (spies), so every applicable
 * action shows. */
function fullHost(): Required<ActionHost> {
  const host: Record<string, unknown> = {};
  for (const name of [
    "openSession", "focusSession", "continueSession", "renameSession", "pinSession", "copyResumeCommand", "revealSessionProject", "stopSession",
    "snoozeSession", "muteSession", "unsnoozeSession", "openProject", "newSessionIn", "openProjectIn", "copyProjectPath", "renameProject", "pinProject",
    "relocateProject", "forgetProject", "setProjectAccount", "openAccount", "startSessionOn", "signIn", "editLaunchSettings", "renameAccount",
    "toggleDefaultAccount", "removeAccount", "openTool", "copyUpdateCommand", "toolAccounts", "openHistory", "replay", "continueHistory", "archive",
    "unarchive", "deleteHistory", "openSessionById",
  ]) {
    host[name] = vi.fn();
  }
  host.isSnoozed = () => false;
  host.isArchived = () => false;
  host.editorName = () => "Zed";
  host.startBlockedWhy = () => null;
  return host as Required<ActionHost>;
}

const labels = <K extends ItemKind>(kind: K, item: ItemOf[K], host: ActionHost = fullHost()) =>
  resolveActions(kind, item, host).map((action) => (action.reason ? `${action.label} (${action.reason})` : action.label));

describe("AJ-E1537 the registry: each type's menu", () => {
  it("a live session", () => {
    expect(labels("session", session())).toEqual([
      "Open", "Show its terminal", "Continue on another account…", "Rename…", "Pin to top", "Copy resume command", "Show its project",
      "Snooze 15m", "Snooze 1h", "Until reset (No known reset time for this quota window)", "Mute this session",
      "Stop",
    ]);
  });

  it("an ended desktop session with no conversation: no focus, no Stop, and why it can't carry over", () => {
    const ended = labels("session", session({ status: "finished", kind: "desktop", conversation_id: null }));
    expect(ended).toContain("Continue on another account… (It has no conversation to carry over yet)");
    expect(ended).toContain("Copy resume command (It has no conversation to resume yet)");
    expect(ended).not.toContain("Stop");
    expect(ended.some((label) => label.startsWith("Focus"))).toBe(false);
  });

  it("a project, and a moved one", () => {
    expect(labels("project", project)).toEqual([
      "Open", "New session here", "Open in Zed", "Show in Finder", "Copy path", "Rename…", "Pin to top", "Set its account…", "Forget…",
    ]);
    expect(labels("project", { ...project, missing: true })).toContain("Relocate…");
    expect(labels("project", { ...project, missing: true })).toContain("New session here (Its folder isn't there any more)");
  });

  it("an account, a tool, a history entry and This Mac's rows", () => {
    expect(labels("account", account)).toEqual([
      "Open", "Start a session", "Sign in…", "Edit launch settings", "Rename…", "Make the default", "Remove…",
    ]);
    const tool = PREVIEW_TOOLS.find((spec) => spec.install_command !== null)!;
    expect(labels("tool", tool)).toEqual(["Open", "Copy update command", "Its accounts"]);
    expect(labels("history", session({ status: "finished" }))).toEqual([
      "Open", "Replay", "Continue on another account…", "Archive", "Delete…",
    ]);
    expect(labels("this-mac", { ...tree, session_id: "s1" })).toEqual(["Go to the session"]);
    // D-08: a tool AI Juggler didn't start says so, and offers nothing.
    expect(labels("this-mac", tree)).toEqual(["Not started by AI Juggler (AI Juggler only acts on sessions it started)"]);
  });

  it("destructive actions come last, in their own group, for every type", () => {
    for (const kind of ["session", "project", "account", "tool", "history", "this-mac"] as const) {
      const groups = actionSpecs(kind).map((spec) => spec.group);
      const firstDestructive = groups.indexOf("destructive");
      if (firstDestructive >= 0) expect(groups.slice(firstDestructive).every((group) => group === "destructive"), kind).toBe(true);
    }
  });

  it("an action whose capability isn't there doesn't show", () => {
    expect(labels("session", session(), { openSession: vi.fn() })).toEqual(["Open"]);
  });

  it("a disabled action never runs", () => {
    const host = fullHost();
    const blocked = resolveActions("session", session({ conversation_id: null }), host).find((action) => action.id === "copy-resume")!;
    expect(blocked.reason).toBeTruthy();
    void blocked.run();
    expect(host.copyResumeCommand).not.toHaveBeenCalled();
  });
});

function Row({ host }: { host: ActionHost }): JSX.Element {
  return (
    <ActionHostProvider host={host}>
      <Item />
    </ActionHostProvider>
  );
}
function Item(): JSX.Element {
  const menu = useItemMenu("session", session(), "main");
  return (
    <div data-testid="row" tabIndex={0} {...menu.triggerProps}>
      main
      {menu.button({ label: "Session actions for main" })}
      {menu.element}
    </div>
  );
}

const shown = () => screen.getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);

describe("AJ-E1537 three triggers, one menu", () => {
  it("⋯, right-click and Shift+F10 show the same items in the same order", () => {
    render(<Row host={fullHost()} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    const fromButton = shown();
    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.contextMenu(screen.getByTestId("row"));
    const fromRightClick = shown();
    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.keyDown(screen.getByTestId("row"), { key: "F10", shiftKey: true });
    const fromKeys = shown();
    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.keyDown(screen.getByTestId("row"), { key: "ContextMenu" });
    expect(shown()).toEqual(fromKeys);
    expect(fromButton).toEqual(fromRightClick);
    expect(fromButton).toEqual(fromKeys);
    expect(fromButton).toEqual(resolveActions("session", session(), fullHost()).map((action) => action.label));
  });

  it("is a role=menu that takes focus and gives it back on Escape", () => {
    render(<Row host={fullHost()} />);
    const button = screen.getByRole("button", { name: "Session actions for main" });
    button.focus();
    fireEvent.click(button);
    expect(screen.getByRole("menu", { name: "Actions for main" })).toBeTruthy();
    expect(document.activeElement?.querySelector("[data-label]")?.textContent).toBe("Open");
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(document.activeElement?.querySelector("[data-label]")?.textContent).toBe("Show its terminal");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(button);
  });

  it("destructive items sit after a separator; the shortcut shows beside Stop", () => {
    render(<Row host={fullHost()} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    const stop = screen.getByRole("menuitem", { name: "Stop" });
    expect(stop.previousElementSibling?.getAttribute("role")).toBe("separator");
    expect(stop.textContent).toContain("⌘.");
  });

  it("choosing an item runs it and closes the menu; a disabled one does nothing", () => {
    const host = fullHost();
    render(<Row host={host} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename…" }));
    expect(host.renameSession).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("menu")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    const until = screen.getByRole("menuitem", { name: "Until reset" }) as HTMLButtonElement;
    expect(until.disabled).toBe(true);
    expect(until.title).toBe("No known reset time for this quota window");
    fireEvent.click(until);
    expect(host.snoozeSession).not.toHaveBeenCalled();
  });
});

describe("AJ-E1537 the native menu", () => {
  it("gets the same items, groups and reasons as the in-app one", async () => {
    const made: { text?: string; enabled?: boolean; item?: string }[] = [];
    vi.doMock("@tauri-apps/api/menu", () => ({
      MenuItem: { new: async (options: { text: string; enabled: boolean }) => (made.push(options), options) },
      PredefinedMenuItem: { new: async (options: { item: string }) => (made.push(options), options) },
      Menu: { new: async () => ({ popup: async () => {} }) },
    }));
    vi.doMock("@tauri-apps/api/dpi", () => ({ LogicalPosition: class {} }));
    const { popupNativeMenu } = await import("../lib/native-menu");
    const actions = resolveActions("session", session(), fullHost());
    await popupNativeMenu("main", actions, { x: 1, y: 2 });
    const enabled = made.filter((entry) => entry.text && !entry.text.startsWith("   ") && entry.text !== "main").map((entry) => entry.text);
    expect(enabled).toEqual(actions.map((action) => action.label));
    expect(made.filter((entry) => entry.item === "Separator")).toHaveLength(3); // heading, attention, destructive
    expect(made.find((entry) => entry.text === "Until reset")?.enabled).toBe(false);
    expect(made.some((entry) => entry.text === "   No known reset time for this quota window" && entry.enabled === false)).toBe(true);
    vi.doUnmock("@tauri-apps/api/menu");
    vi.doUnmock("@tauri-apps/api/dpi");
  });
});

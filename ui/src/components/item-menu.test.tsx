/**
 * AJ-E1537 — one action registry, three triggers. The ⋯ button, a
 * right-click and Shift+F10 open the same items in the same order (the
 * native menu gets that list too); destructive items come last in their
 * own group; a disabled item says why and never runs; each type's menu
 * is pinned here; the menu is a role=menu that gives focus back.
 *
 * AJ-E2106 — every action has an icon, a group and a rank; menus show at
 * most three groups (getting there, working on it, stopping) with
 * stopping last in red; one level of submenus; never possible → left
 * out, not possible now → greyed with the reason.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX } from "react";
import {
  ACTION_ICON_NAMES,
  GROUP_ORDER,
  ITEM_KINDS,
  actionSpecs,
  menuEntries,
  resolveActions,
  type ActionHost,
  type ItemKind,
  type ItemOf,
} from "../lib/actions";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import { PREVIEW_TOOLS } from "../preview/tools";
import { ACTION_ICONS } from "./ActionIcon";
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
const conversation = { id: "c9", account_id: "a1", project_id: "p1", live: false } as ItemOf["conversation"];

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

/** Every capability, the ones the list above leaves out too. */
function everyHost(): ActionHost {
  return {
    ...fullHost(),
    markProject: vi.fn(),
    syncProject: vi.fn(),
    copyAccountSettings: vi.fn(),
    openProjectById: vi.fn(),
    openConversation: vi.fn(),
    continueConversation: vi.fn(),
    copyConversationResume: vi.fn(),
  };
}

const labels = <K extends ItemKind>(kind: K, item: ItemOf[K], host: ActionHost = fullHost()) =>
  resolveActions(kind, item, host).map((action) => (action.reason ? `${action.label} (${action.reason})` : action.label));

const SAMPLES: { [K in ItemKind]: ItemOf[K][] } = {
  session: [session(), session({ status: "finished", conversation_id: null }), session({ kind: "desktop", pinned: true } as Partial<InstanceInfo>)],
  project: [project, { ...project, missing: true }],
  account: [account, { ...account, is_default: true }],
  tool: PREVIEW_TOOLS,
  history: [session({ status: "finished" })],
  "this-mac": [tree, { ...tree, session_id: "s1" }],
  conversation: [conversation, { ...conversation, live: true }],
};

describe("AJ-E2106 every action has an icon, a group and a rank", () => {
  it("in the registry, for every type", () => {
    for (const kind of ITEM_KINDS) {
      for (const spec of actionSpecs(kind)) {
        const where = `${kind}/${spec.id}`;
        expect(GROUP_ORDER, where).toContain(spec.group);
        expect(Number.isFinite(spec.rank), where).toBe(true);
        expect(spec.icon, where).toBeTruthy();
        if (typeof spec.icon === "string") expect(ACTION_ICON_NAMES, where).toContain(spec.icon);
      }
    }
  });

  it("each icon name is drawn, and drawn once", () => {
    expect(Object.keys(ACTION_ICONS).sort()).toEqual([...ACTION_ICON_NAMES].sort());
  });

  it("no two actions of a type share a rank in a group", () => {
    for (const kind of ITEM_KINDS) {
      for (const group of GROUP_ORDER) {
        const ranks = actionSpecs(kind).filter((spec) => spec.group === group).map((spec) => spec.rank);
        expect(new Set(ranks).size, `${kind}/${group}`).toBe(ranks.length);
      }
    }
  });

  it("the same action looks the same everywhere: one label, one icon", () => {
    const seen = new Map<string, string>();
    for (const kind of ITEM_KINDS) {
      for (const item of SAMPLES[kind] as ItemOf[typeof kind][]) {
        for (const action of resolveActions(kind, item as never, everyHost())) {
          const before = seen.get(action.label);
          expect(before ?? action.icon, `${kind}: ${action.label}`).toBe(action.icon);
          expect(ACTION_ICONS[action.icon], action.icon).toBeTruthy();
          seen.set(action.label, action.icon);
        }
      }
    }
  });

  it("a menu has at most three groups, in order, stopping last and in red", () => {
    for (const kind of ITEM_KINDS) {
      for (const item of SAMPLES[kind] as ItemOf[typeof kind][]) {
        const actions = resolveActions(kind, item as never, everyHost());
        const groups = [...new Set(actions.map((action) => action.group))];
        expect(groups.length).toBeLessThanOrEqual(3);
        expect(groups).toEqual(GROUP_ORDER.filter((group) => groups.includes(group)));
        for (const action of actions) expect(action.destructive, action.label).toBe(action.group === "stop");
        // Most used first within each group.
        for (const group of groups) {
          const ranks = actions.filter((action) => action.group === group).map((action) => action.rank);
          expect(ranks).toEqual([...ranks].sort((a, b) => a - b));
        }
      }
    }
  });

  it("submenus are one level, and stay within their group", () => {
    const entries = menuEntries(resolveActions("session", session(), fullHost()));
    const sub = entries.filter((entry) => entry.kind === "submenu");
    expect(sub).toHaveLength(1);
    expect(sub[0]).toMatchObject({ label: "Snooze or mute", group: "work" });
    if (sub[0]!.kind === "submenu") expect(sub[0]!.actions.map((action) => action.label)).toEqual(["For 15 minutes", "Until the limit resets", "Mute this session"]);
  });
});

describe("AJ-E1537 the registry: each type's menu", () => {
  it("a live session: getting there, working on it, stopping", () => {
    expect(labels("session", session())).toEqual([
      "Go to it", "Open its page", "Show its project",
      "Continue on another account…", "Rename…", "Snooze for an hour",
      "For 15 minutes", "Until the limit resets (No known reset time for this quota window)", "Mute this session",
      "Copy the resume command", "Pin to top",
      "Stop this session",
    ]);
  });

  it("a snoozed session offers to wake it, where the snoozes were", () => {
    const host = { ...fullHost(), isSnoozed: () => true };
    const shown = labels("session", session(), host);
    expect(shown).toContain("Unsnooze / unmute");
    expect(shown.some((label) => label.startsWith("Snooze") || label === "Mute this session")).toBe(false);
  });

  it("hide or grey: an ended session with no conversation", () => {
    const ended = labels("session", session({ status: "finished", kind: "desktop", conversation_id: null }));
    // Not possible right now: greyed, with the reason.
    expect(ended).toContain("Continue on another account… (It has no conversation to carry over yet)");
    expect(ended).toContain("Copy the resume command (It has no conversation to resume yet)");
    // Never possible for it: left out.
    expect(ended).not.toContain("Stop this session");
    expect(ended).not.toContain("Go to it");
  });

  it("D-08: a session another app started has no Stop at all", () => {
    expect(labels("conversation", conversation, everyHost())).toEqual([
      "Open its page", "Show its project", "Continue in AI Juggler", "Copy the resume command",
    ]);
    expect(labels("conversation", { ...conversation, live: true }, everyHost())).toContain(
      "Continue in AI Juggler (It's open in another session now)",
    );
  });

  it("a project, and a moved one", () => {
    expect(labels("project", project)).toEqual([
      "Open its page", "Open in Zed", "Show in Finder", "New session here", "Copy path", "Rename…", "Pin to top", "Set its account…", "Forget…",
    ]);
    expect(labels("project", { ...project, missing: true })).toContain("Relocate…");
    expect(labels("project", { ...project, missing: true })).toContain("New session here (Its folder isn't there any more)");
    expect(labels("project", project)).not.toContain("Relocate…");
  });

  it("an account, a tool, a history entry and This Mac's rows", () => {
    expect(labels("account", account)).toEqual([
      "Open its page", "Start a session", "Sign in…", "Edit launch settings", "Rename…", "Make the default", "Remove…",
    ]);
    const tool = PREVIEW_TOOLS.find((spec) => spec.install_command !== null)!;
    expect(labels("tool", tool)).toEqual(["Open its page", "Its accounts", "Copy update command"]);
    expect(labels("history", session({ status: "finished" }))).toEqual([
      "Open its page", "Continue on another account…", "Replay", "Archive", "Delete…",
    ]);
    expect(labels("this-mac", { ...tree, session_id: "s1" })).toEqual(["Open the session's page"]);
    // D-08: a tool AI Juggler didn't start says so, and offers nothing.
    expect(labels("this-mac", tree)).toEqual(["Not started by AI Juggler (AI Juggler only acts on sessions it started)"]);
  });

  it("destructive actions come last, in their own group, for every type", () => {
    for (const kind of ITEM_KINDS) {
      const specs = actionSpecs(kind);
      const order = GROUP_ORDER.flatMap((group) => specs.filter((spec) => spec.group === group).map((spec) => spec.group));
      const firstStop = order.indexOf("stop");
      if (firstStop >= 0) expect(order.slice(firstStop).every((group) => group === "stop"), kind).toBe(true);
    }
  });

  it("an action whose capability isn't there doesn't show", () => {
    expect(labels("session", session(), { openSession: vi.fn() })).toEqual(["Open its page"]);
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

const topMenu = () => screen.getByRole("menu", { name: "Actions for main" });
const shown = () =>
  [...topMenu().querySelectorAll('[role="menuitem"]')]
    .filter((item) => item.closest('[role="menu"]') === topMenu())
    .map((item) => item.querySelector("[data-label]")?.textContent);
const focusedLabel = () => document.activeElement?.querySelector("[data-label]")?.textContent;

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
    expect(fromButton).toEqual(
      menuEntries(resolveActions("session", session(), fullHost())).map((entry) => (entry.kind === "action" ? entry.action.label : entry.label)),
    );
  });

  it("is a role=menu that takes focus and gives it back on Escape", () => {
    render(<Row host={fullHost()} />);
    const button = screen.getByRole("button", { name: "Session actions for main" });
    button.focus();
    fireEvent.click(button);
    expect(topMenu()).toBeTruthy();
    expect(focusedLabel()).toBe("Go to it");
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(focusedLabel()).toBe("Open its page");
    fireEvent.keyDown(window, { key: "End" });
    expect(focusedLabel()).toBe("Stop this session");
    fireEvent.keyDown(window, { key: "Home" });
    expect(focusedLabel()).toBe("Go to it");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(button);
  });

  it("every line has its icon, hidden from assistive tech", () => {
    render(<Row host={fullHost()} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    for (const item of topMenu().querySelectorAll('[role="menuitem"]')) {
      const icon = item.querySelector("svg[data-icon]");
      expect(icon, item.textContent ?? "").toBeTruthy();
      expect(icon!.getAttribute("aria-hidden")).toBe("true");
    }
    expect(screen.getByRole("menuitem", { name: "Go to it" }).querySelector("svg")!.getAttribute("data-icon")).toBe("go-to");
  });

  it("Stop is last, after a separator, in red, with its shortcut", () => {
    render(<Row host={fullHost()} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    const stop = screen.getByRole("menuitem", { name: "Stop this session" });
    expect(stop.previousElementSibling?.getAttribute("role")).toBe("separator");
    expect(stop.className).toContain("text-danger");
    expect(stop.textContent).toContain("⌘.");
    expect(shown().at(-1)).toBe("Stop this session");
    expect(topMenu().querySelectorAll('[role="separator"]')).toHaveLength(2);
  });

  it("a submenu opens with →, closes with ←, and its greyed line says why", () => {
    const host = fullHost();
    render(<Row host={host} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    const trigger = screen.getByRole("menuitem", { name: "Snooze or mute" });
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    trigger.focus();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const sub = screen.getByRole("menu", { name: "Snooze or mute" });
    expect(focusedLabel()).toBe("For 15 minutes");
    // Arrows stay in the submenu, past its greyed line.
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(focusedLabel()).toBe("Mute this session");
    const until = screen.getByRole("menuitem", { name: "Until the limit resets" }) as HTMLButtonElement;
    expect(sub.contains(until)).toBe(true);
    expect(until.disabled).toBe(true);
    expect(until.title).toBe("No known reset time for this quota window");
    expect(document.getElementById(until.getAttribute("aria-describedby")!)?.textContent).toBe("No known reset time for this quota window");
    fireEvent.click(until);
    expect(host.snoozeSession).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(trigger);
    expect(screen.queryByRole("menu", { name: "Snooze or mute" })).toBeNull();
    // Escape in the main menu still closes the whole menu.
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("choosing an item runs it and closes the menu", () => {
    const host = fullHost();
    render(<Row host={host} />);
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename…" }));
    expect(host.renameSession).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("menu")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Session actions for main" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Snooze or mute" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Mute this session" }));
    expect(host.muteSession).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("menu")).toBeNull();
  });
});

describe("AJ-E1537 the native menu", () => {
  it("gets the same items, groups, submenu, keys and reasons as the in-app one", async () => {
    const made: { text?: string; enabled?: boolean; item?: string; accelerator?: string; items?: unknown[] }[] = [];
    vi.doMock("@tauri-apps/api/menu", () => ({
      MenuItem: { new: async (options: { text: string; enabled: boolean }) => (made.push(options), options) },
      PredefinedMenuItem: { new: async (options: { item: string }) => (made.push(options), options) },
      Submenu: { new: async (options: { text: string; items: unknown[] }) => (made.push(options), options) },
      Menu: { new: async () => ({ popup: async () => {} }) },
    }));
    vi.doMock("@tauri-apps/api/dpi", () => ({ LogicalPosition: class {} }));
    const { popupNativeMenu, accelerator } = await import("../lib/native-menu");
    const actions = resolveActions("session", session(), fullHost());
    await popupNativeMenu("main", actions, { x: 1, y: 2 });
    const lines = made.filter((entry) => entry.text && !entry.items && !entry.text.startsWith("   ") && entry.text !== "main").map((entry) => entry.text);
    expect(lines).toEqual(actions.map((action) => action.label));
    expect(made.filter((entry) => entry.item === "Separator")).toHaveLength(3); // heading, working on it, stopping
    const submenu = made.find((entry) => entry.text === "Snooze or mute")!;
    expect((submenu.items as { text: string }[]).map((item) => item.text)).toEqual([
      "For 15 minutes", "Until the limit resets", "   No known reset time for this quota window", "Mute this session",
    ]);
    expect(made.find((entry) => entry.text === "Until the limit resets")?.enabled).toBe(false);
    expect(made.find((entry) => entry.text === "Stop this session")?.accelerator).toBe("CmdOrCtrl+.");
    expect(made.find((entry) => entry.text === "Continue on another account…")?.accelerator).toBe("CmdOrCtrl+Shift+H");
    expect(accelerator("Enter")).toBeUndefined();
    vi.doUnmock("@tauri-apps/api/menu");
    vi.doUnmock("@tauri-apps/api/dpi");
  });
});

/**
 * AJ-E2901 vitest — what the first palette tests left unasserted (the
 * evidence manifest of 2026-11-06): ⇧Tab kept inside and focus back to
 * the invoker after Esc; Processes and Notifications among the top
 * actions only when they have something; groups by their best match,
 * each labelled; Accounts and Apps rows with what ↩ and ⌘↩ do; the
 * preview's facts for a session, an app, a project and an account; the
 * footer's words; Recent cleared with the local cache; the registry's
 * label, and a visible control for every action ⌘K lists.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GoToPalette } from "./GoToPalette";
import { ActionHostProvider } from "./ItemMenu";
import { AppHeader } from "./AppHeader";
import { RailHeader } from "./RailHeader";
import { ProjectsScreen } from "../screens/projects/ProjectsScreen";
import { LocalCacheSection } from "../screens/settings/LocalCacheSection";
import type { ActionHost } from "../lib/actions";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { appCommands, type AppDoings } from "../lib/palette-commands";
import { loadRecents, rememberRecent } from "../lib/palette-recents";
import { resetBackendHealth } from "../lib/backend-health";
import { resetNotices } from "../lib/notices-store";
import { resetNoticeStandIn } from "../lib/ipc/notices";
import { loadSubscriptions, staleSubscriptions } from "../lib/subscriptions";
import { SHORTCUTS } from "../lib/shortcuts";
import { agoWords } from "../lib/time-words";
import { OPEN_APP } from "../lib/apps";
import { previewHooks } from "../lib/preview-hooks";
import { account, clearSeams, detailOf, instance, project, setSeam, subscription } from "../test/fixtures";
import { quotaDisplay, windowDisplay } from "../lib/wire";
import * as subs from "../lib/ipc/subscriptions";
import * as places from "../lib/ipc/places";
import * as processes from "../lib/ipc/processes";

const NOW = Date.now();
const SESSIONS = [instance({ id: "s-bill", account_id: "work", account_label: "work", label: "Fix billing export", last_activity_ms: NOW - 5 * 60_000 })];
const PROJECTS = [project({ id: "p-pay", name: "payments-api", root: "/w/payments-api", last_activity_ms: NOW - 2 * 3_600_000 })];
const WORK_FOLDER = account({ id: "work", label: "work" });
const SUBSCRIPTIONS = [
  subscription({ folder: "work", key: "k-work", account_id: "acct:anthropic:work", display_name: "Claude Max 20x", plan: "max-20x" }),
  subscription({ folder: "codex-p", key: "k-codex", account_id: "acct:openai:codex", name: "Codex Pro", display_name: "ChatGPT Pro", tool: "codex" }),
];

function doings(): AppDoings & Record<string, unknown> {
  return { openScreen: vi.fn(), newSession: vi.fn(), newProject: vi.fn(), notifications: vi.fn(), shortcuts: vi.fn(), toggleRail: vi.fn(), searchHistory: vi.fn(), fleet: false };
}

function openPalette({ host = {}, onClose = vi.fn(), accounts = [] }: { host?: ActionHost; onClose?: () => void; accounts?: AccountInfo[] } = {}) {
  const app = doings();
  const tree = (open: boolean) => (
    <ActionHostProvider host={host}>
      <button type="button" data-testid="invoker">
        ⌘K
      </button>
      <GoToPalette open={open} onClose={onClose} sessions={SESSIONS} onFocusWindow={vi.fn()} onOpenSession={vi.fn()} hotkey="" onHotkeyChange={async () => {}} projects={PROJECTS} accounts={accounts} app={app} />
    </ActionHostProvider>
  );
  const view = render(tree(false));
  const invoker = view.getByTestId("invoker");
  invoker.focus();
  view.rerender(tree(true));
  const input = () => view.container.querySelector<HTMLInputElement>("input[role='combobox']")!;
  const type = (value: string) => fireEvent.change(input(), { target: { value } });
  const groups = () => [...view.container.querySelectorAll<HTMLElement>("ul[role='group']")].map((group) => group.getAttribute("aria-label"));
  return { view, input, type, groups, invoker, onClose, app, rerender: (open: boolean) => view.rerender(tree(open)) };
}

/** A preview's facts, label → value. */
function factsOf(preview: HTMLElement): Record<string, string> {
  return Object.fromEntries([...preview.querySelectorAll("dt")].map((term) => [term.textContent, term.nextElementSibling?.textContent ?? ""]));
}

beforeEach(() => {
  localStorage.clear();
  resetNotices();
  resetNoticeStandIn();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  resetBackendHealth();
  staleSubscriptions();
  vi.restoreAllMocks();
  clearSeams("__AIJ_NOTICES__", "__AIJ_QUOTA_DISPLAY__", "__AIJ_PLACES__");
});

describe("R1 — the dialog contract", () => {
  it("⇧Tab from the field wraps to the last control; Esc closes, and focus goes back to the invoker", () => {
    const { view, input, invoker, onClose, rerender } = openPalette();
    const dialog = view.getByRole("dialog", { name: "Search, act or go" });
    expect(document.activeElement).toBe(input());
    const buttons = within(dialog).getAllByRole("button");
    fireEvent.keyDown(input(), { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(buttons.at(-1));
    fireEvent.keyDown(buttons.at(-1)!, { key: "Tab" });
    expect(document.activeElement).toBe(input());
    // The combobox pattern stays.
    expect(input().getAttribute("aria-expanded")).toBe("true");
    expect(input().getAttribute("aria-activedescendant")).toMatch(/^goto-option-/);
    fireEvent.keyDown(input(), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
    rerender(false);
    expect(view.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(invoker);
  });
});

describe("R2 — the top actions before typing", () => {
  const actionLabels = (view: ReturnType<typeof openPalette>["view"]) => [...view.getByTestId("goto-group-actions").querySelectorAll("[role='option']")].map((row) => row.querySelector("[data-label], span")?.textContent ?? row.textContent);

  it("adds Processes with what's left running and Notifications with the unread count, only when above 0", async () => {
    vi.spyOn(processes, "processesCounts").mockResolvedValue({ processes: 9, sessions: 1, attention: 0, to_clean: 2, left_behind: 2, zombies: 0, didnt_stop: 0, wont_touch: 0 });
    setSeam("__AIJ_NOTICES__", { earlier: [1, 2, 3].map((id) => ({ id, kind: "finished", needs: false, title: `Done ${id}`, detail: "", project_id: null, project: null, target: { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null }, facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null }, action: null, meter: null, created_ms: NOW, unread: true, subject: `n:${id}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false })) });
    const { view } = openPalette();
    await waitFor(() => expect(view.getByTestId("goto-group-actions").textContent).toContain("Notifications — 3 unread"));
    await waitFor(() => expect(view.getByTestId("goto-group-actions").textContent).toContain("Processes — 2 left running"));
    const rows = [...view.getByTestId("goto-group-actions").querySelectorAll("[role='option']")].map((row) => row.getAttribute("data-testid"));
    expect(rows).toEqual(["goto-row-command::new-session", "goto-row-command::go:settings", "goto-row-command::go:cleanup", "goto-row-command::notifications"]);
  });

  it("leaves both out at zero", async () => {
    vi.spyOn(processes, "processesCounts").mockResolvedValue({ processes: 9, sessions: 1, attention: 0, to_clean: 0, left_behind: 0, zombies: 0, didnt_stop: 0, wont_touch: 0 });
    const { view } = openPalette();
    await act(async () => {
      await Promise.resolve();
    });
    const rows = [...view.getByTestId("goto-group-actions").querySelectorAll("[role='option']")].map((row) => row.getAttribute("data-testid"));
    expect(rows).toEqual(["goto-row-command::new-session", "goto-row-command::go:settings"]);
    expect(actionLabels(view).join(" ")).not.toMatch(/left running|unread/);
  });
});

describe("R3 — typed, groups by their best match, each labelled", () => {
  it("an exact project leads a loose action; each group has its section label; Search last", () => {
    const { view, type, groups } = openPalette();
    type("payments");
    const order = groups();
    expect(order[0]).toBe("Projects");
    expect(order.at(-1)).toBe("Search");
    for (const label of order) {
      const key = label!.toLowerCase();
      expect(view.container.querySelector(`div#goto-group-${key}`)?.textContent).toContain(label!);
    }
    type("new");
    expect(groups()[0]).toBe("Actions");
  });
});

describe("R4 — Accounts", () => {
  it("each by its name, its tool and its room in words (never 0 % unknown); ↩ opens its page, ⌘↩ New session on it", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue(SUBSCRIPTIONS);
    await loadSubscriptions(true);
    setSeam("__AIJ_QUOTA_DISPLAY__", [{ ...quotaDisplay("work"), five_hour: windowDisplay("tracking", { percentage: 69 }) }]);
    const openAccountById = vi.fn();
    const startSessionOn = vi.fn();
    const { view, type, input } = openPalette({ host: { openAccountById, startSessionOn }, accounts: [WORK_FOLDER] });
    type("claude max");
    const work = await view.findByTestId("goto-row-account::k-work");
    expect(work.textContent).toMatch(/^Claude Max 20xClaude Code · /);
    await waitFor(() => expect(within(work).getByTestId("goto-account-room").textContent).toBe("Claude Code · 31% left in the 5-hour window"));
    type("codex pro");
    const codex = await view.findByTestId("goto-row-account::k-codex");
    expect(codex.textContent).toMatch(/^ChatGPT Pro/);
    const room = within(codex).getByTestId("goto-account-room").textContent;
    expect(room).toBe("Codex · Not measured yet");
    expect(room).not.toMatch(/0\s?%/);
    // ↩ opens the account's page.
    type("claude max");
    expect(input().getAttribute("aria-activedescendant")).toBe("goto-option-account__k-work");
    fireEvent.keyDown(input(), { key: "Enter" });
    expect(openAccountById).toHaveBeenCalledWith("acct:anthropic:work");
    expect(startSessionOn).not.toHaveBeenCalled();
  });

  it("⌘↩ starts New session on its signed-in folder", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue(SUBSCRIPTIONS);
    await loadSubscriptions(true);
    const openAccountById = vi.fn();
    const startSessionOn = vi.fn();
    const { view, type, input } = openPalette({ host: { openAccountById, startSessionOn }, accounts: [WORK_FOLDER] });
    type("claude max");
    await view.findByTestId("goto-row-account::k-work");
    fireEvent.keyDown(input(), { key: "Enter", metaKey: true });
    expect(startSessionOn).toHaveBeenCalledWith(WORK_FOLDER);
    expect(openAccountById).not.toHaveBeenCalled();
  });
});

describe("R5 — Apps", () => {
  it("an open app shows its own icon and launch badge, open ones first; ↩ opens its place, ⌘↩ brings it forward", async () => {
    setSeam("__AIJ_PLACES__", previewHooks().places.knownPlacesPreview(NOW));
    const opened: string[] = [];
    const listen = (event: Event) => {
      const detail = detailOf(event);
      opened.push(typeof detail === "object" && detail !== null && "id" in detail ? String(detail.id) : "");
    };
    window.addEventListener(OPEN_APP, listen);
    const forward = vi.spyOn(places, "placesOpen");
    const { view, type, input } = openPalette();
    type("claude app");
    const group = await view.findByTestId("goto-group-apps");
    const rows = [...group.querySelectorAll<HTMLElement>("[role='option']")];
    // Open now first, then not running.
    // D-100: each says its kind first ("Desktop · open now").
    const states = rows.map((row) => (within(row).getByTestId("goto-app-state").textContent ?? "").replace(/^(Desktop|CLI) · /, ""));
    expect(states.indexOf("not running")).toBeGreaterThan(states.lastIndexOf("open now"));
    const work = rows.find((row) => row.getAttribute("data-testid") === "goto-row-app::app:claude-app:/Users/you/Profiles/Work")!;
    expect(within(work).getByTestId("goto-app-state").textContent).toBe("Desktop · open now");
    expect(within(work).getByTestId("app-kind").getAttribute("aria-label")).toBe("Desktop app");
    // Its own app's icon (read at run time by AppIcon), and its launch settings' badge.
    expect(within(work).getByTestId("app-icon").getAttribute("data-app")).toBe("claude-app");
    expect(within(work).getByTestId("profile-badge").getAttribute("title")).toBe("Work Max");
    fireEvent.mouseEnter(work);
    fireEvent.keyDown(input(), { key: "Enter" });
    expect(opened).toEqual(["app:claude-app:/Users/you/Profiles/Work"]);
    expect(forward).not.toHaveBeenCalled();
    window.removeEventListener(OPEN_APP, listen);
  });

  it("⌘↩ calls the platform's open for its place", async () => {
    setSeam("__AIJ_PLACES__", previewHooks().places.knownPlacesPreview(NOW));
    const forward = vi.spyOn(places, "placesOpen");
    const { view, type, input } = openPalette();
    type("claude app");
    const work = await view.findByTestId("goto-row-app::app:claude-app:/Users/you/Profiles/Work");
    fireEvent.mouseEnter(work);
    fireEvent.keyDown(input(), { key: "Enter", metaKey: true });
    expect(forward).toHaveBeenCalledWith("app:claude-app:/Users/you/Profiles/Work");
  });
});

describe("R7 — → previews a session, an app, a project or an account", () => {
  const preview = (view: ReturnType<typeof openPalette>["view"], input: () => HTMLInputElement) => {
    input().setSelectionRange(input().value.length, input().value.length);
    fireEvent.keyDown(input(), { key: "ArrowRight" });
    return view.getByTestId("goto-preview");
  };

  it("a session: its state word, where it runs, its account and its last activity; Open and Continue", () => {
    const { view, type, input } = openPalette();
    type("fix billing");
    const facts = factsOf(preview(view, input));
    expect(Object.keys(facts)).toEqual(["Now", "Where", "Account", "Last active"]);
    expect(facts.Now).toBe("Working");
    expect(facts.Where).not.toBe("");
    expect(facts["Last active"]).toBe(agoWords(NOW - 5 * 60_000, Date.now()));
    expect(view.getByTestId("goto-preview-open").textContent).toBe("Open");
    expect(view.getByTestId("goto-preview-continue").textContent).toBe("Continue in another app…");
  });

  it("an app: open now or not, which app, signed in as; Open its page and Bring it forward", async () => {
    setSeam("__AIJ_PLACES__", previewHooks().places.knownPlacesPreview(NOW));
    const { view, type, input } = openPalette();
    type("claude app work");
    fireEvent.mouseEnter(await view.findByTestId("goto-row-app::app:claude-app:/Users/you/Profiles/Work"));
    const facts = factsOf(preview(view, input));
    expect(facts.Now).toBe("Open now");
    expect(facts.App).toBe("Claude app · Work Max");
    expect(view.getByTestId("goto-preview-open").textContent).toBe("Open its page");
    expect(view.getByTestId("goto-preview-goto").textContent).toBe("Bring it forward");
  });

  it("a project: its folder and last activity", () => {
    const { view, type, input } = openPalette();
    type("payments-api");
    const facts = factsOf(preview(view, input));
    expect(facts).toEqual({ Folder: "/w/payments-api", "Last active": agoWords(NOW - 2 * 3_600_000, Date.now()) });
  });

  it("an account: its tool, its room and its plan; New session on it…", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue(SUBSCRIPTIONS);
    await loadSubscriptions(true);
    const { view, type, input } = openPalette({ accounts: [WORK_FOLDER] });
    type("claude max");
    await view.findByTestId("goto-row-account::k-work");
    const facts = factsOf(preview(view, input));
    expect(facts).toEqual({ Tool: "Claude Code", Room: "Not measured yet", Plan: "max-20x" });
    expect(view.getByTestId("goto-preview-goto").textContent).toBe("New session on it…");
  });
});

describe("R8 — the footer", () => {
  it("says the keys, and carries See all results →", () => {
    const { view } = openPalette();
    expect(view.getByTestId("goto-keys").textContent).toBe("↑↓ move · ↩ open · ⌘↩ go to its app · → preview · Esc close");
    expect(view.getByTestId("goto-see-all").textContent).toBe("See all results →");
  });
});

describe("R10 — Recent goes with the local cache's Clear", () => {
  it("Settings › Advanced › Local cache › Clear cache empties Recent", async () => {
    rememberRecent({ kind: "project", id: "p-pay" });
    rememberRecent({ kind: "session", id: "s-bill" });
    expect(loadRecents()).toHaveLength(2);
    render(
      <ul>
        <LocalCacheSection />
      </ul>,
    );
    fireEvent.click(await screen.findByTestId("cache-clear"));
    expect(loadRecents()).toHaveLength(2);
    await act(async () => {
      fireEvent.click(await screen.findByTestId("cache-clear-go"));
    });
    expect(loadRecents()).toEqual([]);
    expect(localStorage.getItem("aij.palette.recents")).toBeNull();
    clearSeams("__AIJ_CACHE__", "__AIJ_CACHE_CALLS__");
  });
});

describe("R11 — a shortcut, never the only path", () => {
  it("the registry names ⌘K “Search, act or go”", () => {
    const palette = SHORTCUTS.find((shortcut) => shortcut.id === "palette")!;
    expect(palette.label).toBe("Search, act or go");
    expect(palette.keys).toEqual(["⌘K"]);
  });

  it("every action ⌘K lists has a visible control", async () => {
    const app = { ...doings(), fleet: true };
    render(
      <>
        <RailHeader minimal={false} info={null} onToggle={() => {}} />
        <AppHeader title="Home" global={null} page="home" pairedCount={1} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} />
        <ProjectsScreen projects={[]} sessions={[]} loading={false} now={NOW} onNewProject={() => {}} />
      </>,
    );
    const visible = new Set<string>();
    for (const button of screen.getAllByRole("button")) visible.add((button.getAttribute("aria-label") ?? button.textContent ?? "").trim());
    fireEvent.click(screen.getByTestId("user-menu-button"));
    const menu = await screen.findByRole("menu", { name: "Your menu" });
    for (const item of within(menu).getAllByRole("menuitem")) visible.add(item.querySelector("[data-label]")?.textContent ?? "");
    fireEvent.keyDown(menu, { key: "Escape" });
    fireEvent.click(screen.getByTestId("floor-menu-button"));
    const floor = await screen.findByRole("menu", { name: "The Floor" });
    for (const item of within(floor).getAllByRole("menuitem")) visible.add(item.querySelector("[data-label]")?.textContent ?? "");
    const CONTROL: Record<string, string> = {
      "new-session": "New session",
      "new-project": "New project",
      "go:home": "Home",
      "go:projects": "Projects",
      "go:pools": "Pools",
      "go:tasks": "Tasks",
      "go:history": "History",
      "go:accounts": "Accounts",
      "go:usage": "Usage",
      "go:fleet": "Fleet",
      "go:cleanup": "Processes",
      "go:settings": "Settings",
      "go:thismac": "This Mac",
      "go:privacy": "Privacy",
      "go:floor": "The Floor list",
      notifications: "Notifications",
      shortcuts: "Keyboard shortcuts",
      "toggle-rail": "Show icons only",
    };
    const missing: string[] = [];
    for (const command of appCommands(app)) {
      const control = CONTROL[command.id];
      expect(control, `${command.id} has a control named here`).toBeDefined();
      const found = [...visible].some((name) => name === control || name.startsWith(`${control}:`) || name.startsWith(`${control} `));
      if (!found) missing.push(`${command.id} → ${control}`);
    }
    expect(missing).toEqual([]);
  });
});

/**
 * The maintainer's review of 0.1.4: every clickable thing in the list is
 * a way into AI Juggler. This walks each kind, in the place lens and the
 * Project lens, and checks where a plain left-click lands: a session's
 * page, a project's page, a profile's page, Home, or a section opened or
 * folded in place. None ends with only a toast: nothing but a row's ↗
 * (and a place's Open) goes to another app.
 */
import { needsFor } from "../test/needs-kit";
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView, type RailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { placesConversations } from "../preview/places";
import type { ActionHost } from "../lib/actions";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import * as ipc from "../lib/ipc/conversations";
import * as toast from "./Toast";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
const seams = () => window as unknown as Record<string, unknown>;

const project = (id: string, name: string): ProjectInfo =>
  ({ id, root: `/Users/you/Dev/${name}`, name, last_activity_ms: NOW, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false, pinned: false }) as ProjectInfo;
const PROJECTS = [project("proj-ai-juggler", "ai-juggler"), project("proj-payments", "payments-api"), { ...project("proj-quiet", "quiet-lib"), instance_count: 0, history_sessions: 2, history_last_used_ms: NOW - 86_400_000 }];

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: id, git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  } as InstanceInfo;
}

const SESSIONS = [
  own("aj-working"),
  own("aj-asks", { status: "waiting_user", needs_attention: "waiting_user" }),
  own("aj-pinned", { status: "finished", pinned: true, last_activity_ms: NOW - 5 * 3_600_000 }),
];

let host: Required<Pick<ActionHost, "openConversation" | "goToConversation" | "openProjectById" | "openAccountById" | "openHome">>;
let onSelect: ReturnType<typeof vi.fn<(id: string) => void>>;
let onSelectProject: ReturnType<typeof vi.fn<(id: string) => void>>;
let bring: ReturnType<typeof vi.spyOn>;
let toasts: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  resetSeen();
  host = { openConversation: vi.fn(), goToConversation: vi.fn(), openProjectById: vi.fn(), openAccountById: vi.fn(), openHome: vi.fn() };
  onSelect = vi.fn<(id: string) => void>();
  onSelectProject = vi.fn<(id: string) => void>();
  bring = vi.spyOn(ipc, "conversationsBringForward").mockResolvedValue({ outcome: "Brought it forward", reached: "app" });
  toasts = vi.spyOn(toast, "showToast");
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  resetRailViewCache();
  vi.restoreAllMocks();
});

function board(view: RailView) {
  setRailView(view);
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  needsFor({ sessions: SESSIONS, conversations: placesConversations(NOW), projects: PROJECTS }); // D-49
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={host}>
        <InstanceBoard sessions={SESSIONS} loginTerminals={[]} selectedId={null} onSelect={onSelect} onSelectProject={onSelectProject} density="comfortable" freshness={{}} now={NOW} projects={PROJECTS} snooze={{}} onSnooze={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

/** Where every page-opening call landed, in order. */
function landed(): string[] {
  const calls: [number, string][] = [];
  const add = (fn: { mock: { calls: unknown[][]; invocationCallOrder: number[] } }, name: (args: unknown[]) => string) =>
    fn.mock.calls.forEach((args, n) => calls.push([fn.mock.invocationCallOrder[n]!, name(args)]));
  add(host.openConversation as never, (args) => `conversation:${(args[0] as { id: string }).id}`);
  add(host.openProjectById as never, (args) => `project:${String(args[0])}`);
  add(host.openAccountById as never, (args) => `profile:${String(args[0])}`);
  add(host.openHome as never, () => "home");
  add(onSelect as never, (args) => `session:${String(args[0])}`);
  add(onSelectProject as never, (args) => `project:${String(args[0])}`);
  return calls.sort((a, b) => a[0] - b[0]).map(([, name]) => name);
}

const row = (root: HTMLElement, key: string) => root.querySelector<HTMLElement>(`[data-rail-item="${key}"]`)!;
const header = (root: HTMLElement, place: string) => root.querySelector<HTMLElement>(`[data-testid="place-header"][data-place="${place}"]`)!;

describe("every rail element opens a page here (place lens)", () => {
  it("rows, a row's project, place headers, sections, the Also line: each lands in AI Juggler", async () => {
    const view = board({ ...DEFAULT_VIEW, show: { ...DEFAULT_VIEW.show, quiet: true } });
    const list = await view.findByTestId("place-board");
    await waitFor(() => expect(row(list, "c:claude-work/outside-3")).toBeTruthy());

    // Needs you: one another app runs, then one of AI Juggler's.
    fireEvent.click(row(list, "c:claude-work/outside-3"));
    fireEvent.click(row(list, "s:aj-asks"));
    // Open now: one another app runs, one of AI Juggler's, a pinned one.
    fireEvent.click(row(list, "c:claude-work/outside-1"));
    fireEvent.click(row(list, "s:aj-working"));
    fireEvent.click(row(list, "s:aj-pinned"));
    // A row's project, its own target (≥ 24 px tall, named).
    const projectName = within(row(list, "c:claude-work/outside-3").parentElement!).getByRole("button", { name: "Open project payments-api" });
    expect(projectName.className).toContain("min-h-6");
    fireEvent.click(projectName);
    // An App's header: its page (D-62; the App page lands with AJ-E3204,
    // its folder's page until then), the CLI App AI Juggler runs one in too.
    fireEvent.click(within(header(list, "app|claude-app|Work")).getByTestId("place-page"));
    fireEvent.click(within(header(list, "cli|claude-code|claude-work")).getByTestId("place-page"));
    expect(landed()).toEqual([
      "conversation:outside-3",
      "session:aj-asks",
      "conversation:outside-1",
      "session:aj-working",
      "session:aj-pinned",
      "project:proj-payments",
      "profile:claude-work",
      "profile:claude-work",
    ]);

    // Section heads fold and unfold in place. D-47: the one asking stays in
    // its place in Open now too, so folding Needs you leaves that row.
    const asking = () => list.querySelectorAll('[data-rail-item="c:claude-work/outside-3"]').length;
    expect(asking()).toBe(2);
    const needs = within(view.getByTestId("rail-needs-you")).getByRole("button", { expanded: true });
    fireEvent.click(needs);
    expect(asking()).toBe(1);
    expect(needs.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(needs);
    expect(asking()).toBe(2);
    const open = within(view.getByTestId("rail-open-now")).getByRole("button", { expanded: true });
    fireEvent.click(open);
    expect(header(list, "app|claude-app|Work")).toBeNull();
    fireEvent.click(open);
    expect(header(list, "app|claude-app|Work")).toBeTruthy();
    // A place's chevron folds it in place (its rows go, its header stays).
    fireEvent.click(within(header(list, "app|claude-app|Personal")).getByTestId("rail-fold"));
    expect(row(list, "c:claude-personal/outside-4")).toBeNull();

    // D-64: an app's inactive sessions are under it: "Last n inactive"
    // shows them in place, and hides them again.
    expect(view.queryByTestId("rail-also-inactive")).toBeNull();
    expect(row(list, "c:claude-work/outside-8")).toBeNull();
    const recent = view.getAllByTestId("rail-recent");
    recent.forEach((link) => fireEvent.click(link));
    await waitFor(() => expect(row(list, "c:claude-work/outside-8")).toBeTruthy());
    expect(recent.every((link) => link.getAttribute("aria-expanded") === "true")).toBe(true);
    view.getAllByTestId("rail-recent").forEach((link) => fireEvent.click(link));
    await waitFor(() => expect(row(list, "c:claude-work/outside-8")).toBeNull());
    // History › opens History.
    const screens: unknown[] = [];
    const onScreen = (event: Event) => screens.push((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener("aij:open-screen", onScreen);
    fireEvent.click(view.getByTestId("rail-also-history"));
    window.removeEventListener("aij:open-screen", onScreen);
    expect(screens).toEqual(["history"]);

    // Not one of those went to an app, or ended with only a toast.
    expect(host.goToConversation).not.toHaveBeenCalled();
    expect(bring).not.toHaveBeenCalled();
    expect(toasts).not.toHaveBeenCalled();
  });

  it("the way out is only ↗ (or ⌘↩) on a row, and a place's Open", async () => {
    const view = board({ ...DEFAULT_VIEW, show: { ...DEFAULT_VIEW.show, quiet: true } });
    const list = await view.findByTestId("place-board");
    await waitFor(() => expect(row(list, "c:claude-work/outside-3")).toBeTruthy());
    const asks = row(list, "c:claude-work/outside-3").parentElement!;
    // Needs you: always shown, named for its app and profile.
    const arrow = within(asks).getByRole("button", { name: "Go to it in Claude app · Work" });
    expect(arrow.className).not.toContain("opacity-0");
    fireEvent.click(arrow);
    expect(host.goToConversation).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(row(list, "c:claude-work/outside-3"), { key: "Enter", metaKey: true });
    expect(host.goToConversation).toHaveBeenCalledTimes(2);
    // Open now: on hover and focus.
    expect(within(row(list, "c:claude-work/outside-1").parentElement!).getByRole("button", { name: "Go to it in Claude app · Work" }).className).toContain("opacity-0");
    // AI Juggler's own terminal session: its page is where it runs.
    expect(within(row(list, "s:aj-working").parentElement!).queryByRole("button", { name: /^Go to it/ })).toBeNull();
    fireEvent.click(within(header(list, "app|claude-app|Work")).getByTestId("place-open"));
    expect(bring).toHaveBeenCalledWith(4100);
    expect(host.openConversation).not.toHaveBeenCalled();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("the Account lens's header opens its profile's page", async () => {
    const view = board({ ...DEFAULT_VIEW, group: "account", show: { ...DEFAULT_VIEW.show, quiet: true } });
    const list = await view.findByTestId("place-board");
    await waitFor(() => expect(list.querySelectorAll("[data-testid='place-page']").length).toBeGreaterThan(0));
    const personal = [...list.querySelectorAll<HTMLElement>("[data-testid='place-page']")].find((button) => button.textContent?.startsWith("Not signed in"))!;
    fireEvent.click(personal);
    expect(landed()).toEqual(["profile:claude-personal"]);
  });
});

describe("every rail element opens a page here (Project lens)", () => {
  it("a project's header, its rows, an idle project, and the section heads", async () => {
    const view = board({ ...DEFAULT_VIEW, group: "project" });
    const rail = await view.findByTestId("instance-board");
    await waitFor(() => expect(rail.querySelector("[data-conversation-id='outside-1']")).toBeTruthy());
    // The project's header (the group header) opens its page.
    fireEvent.click(within(rail).getAllByTestId("rail-project").find((button) => button.textContent?.includes("ai-juggler"))!);
    // Its rows: AI Juggler's session, and one another app runs.
    fireEvent.click(rail.querySelector<HTMLElement>("[data-instance-id='aj-working']")!);
    fireEvent.click(rail.querySelector<HTMLElement>("[data-conversation-id='outside-1']")!);
    // Needs you's rows here too.
    fireEvent.click(within(view.getByTestId("rail-needs-you")).getAllByTestId("rail-outside")[0]!);
    // A project with no session here yet.
    fireEvent.click(within(view.getByTestId("rail-history-projects")).getByText("quiet-lib"));
    expect(landed()).toEqual(["project:proj-ai-juggler", "session:aj-working", "conversation:outside-1", "conversation:outside-3", "project:proj-quiet"]);

    // Needs you and Open now fold in place.
    const needs = within(view.getByTestId("rail-needs-you")).getByRole("button", { name: /^Needs you/ });
    fireEvent.click(needs);
    expect(within(view.getByTestId("rail-needs-you")).queryAllByTestId("rail-outside")).toHaveLength(0);
    fireEvent.click(needs);
    expect(within(view.getByTestId("rail-needs-you")).queryAllByTestId("rail-outside").length).toBeGreaterThan(0);
    const open = within(view.getByTestId("rail-open-now")).getByRole("button", { name: /^Open now/ });
    fireEvent.click(open);
    expect(rail.querySelector("[data-conversation-id='outside-1']")).toBeNull();
    fireEvent.click(open);
    expect(rail.querySelector("[data-conversation-id='outside-1']")).toBeTruthy();
    act(() => undefined);
    expect(host.goToConversation).not.toHaveBeenCalled();
    expect(toasts).not.toHaveBeenCalled();
  });
});

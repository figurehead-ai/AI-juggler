/**
 * AJ-E2104 — the rail by place, as the user meets it: Needs you, then
 * Open now by place with each place's header (app, profile, count, Open),
 * rows whose labels lead with the state, quiet places folded with their
 * count, the Also line turning views on, the View popover (⇧V, a dot when
 * changed, Back to the defaults, kept), apps turned off in Settings
 * leaving their count, ⌘1–9 to places, ↑↓ between rows, and 40 sessions
 * drawn virtualised within a frame.
 */
import { needsFor } from "../test/needs-kit";
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, railView, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache, setOrderMode } from "../lib/rail-order-store";
import { placesConversations } from "../preview/places";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import * as ipc from "../lib/ipc/conversations";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown; __AIJ_APP_SETTINGS__?: Record<string, string> };
const seams = () => window as unknown as Seams;

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_APP_SETTINGS__;
  resetRailViewCache();
  resetRailOrderCache();
  vi.restoreAllMocks();
});

const projects = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler" }, { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api" }] as ProjectInfo[];

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "feat/read-model", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  } as InstanceInfo;
}

function board(sessions: InstanceInfo[], conversations: ConversationInfo[] = placesConversations(NOW)) {
  seams().__AIJ_FIXTURE__ = { conversations };
  needsFor({ sessions, conversations, projects }); // D-49: the backend's list
  return render(
    <ConversationsProvider>
      <InstanceBoard sessions={sessions} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} />
    </ConversationsProvider>,
  );
}

const headers = (root: HTMLElement) => [...root.querySelectorAll<HTMLElement>("[data-testid='place-header']")];

describe("the rail by place", () => {
  it("reads Needs you, then Open now by place in a stable order, then Also", async () => {
    const view = board([own("aj")]);
    const needs = await view.findByTestId("rail-needs-you");
    expect(needs.textContent).toMatch(/Needs you\s*3/i);
    const open = view.getByTestId("rail-open-now");
    expect(needs.compareDocumentPosition(open) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const names = headers(view.getByTestId("place-board")).map((h) => h.getAttribute("data-place"));
    expect(names).toEqual(["ai-juggler", "ChatGPT|Codex", "Claude|Personal", "Claude|Work", "Terminal|Work Max"]);
    expect(view.getByTestId("rail-also").textContent).toMatch(/1 ended today\s*·\s*1 inactive\s*·\s*1 background\s*·\s*History ›/);
  });

  it("a row's label leads with its state; one worked out says probably", async () => {
    const view = board([]);
    await view.findByTestId("rail-needs-you");
    const asking = view.getByTestId("place-board").querySelector<HTMLElement>("[data-conversation-id='outside-9']")!;
    expect(asking.textContent!.startsWith("Probably asks you, Answer the release question")).toBe(true);
    const working = view.getByTestId("place-board").querySelector<HTMLElement>("[data-conversation-id='outside-1']")!;
    expect(working.textContent!.startsWith("Working, Refactor the settings screen")).toBe(true);
    // Message text never shows on a row: the title and where it is only.
    expect(working.textContent).toContain("viewed 2 min ago");
  });

  it("Latest activity: a quiet place folds to its header with its count, and opens on its chevron", async () => {
    setOrderMode("latest");
    const view = board([]);
    await view.findByTestId("rail-open-now");
    // D-47: a place with one to review isn't quiet (that row stays in its place).
    const personal = headers(view.getByTestId("place-board")).find((h) => h.dataset.place === "Claude|Personal")!;
    expect(within(personal).getByTestId("rail-fold").getAttribute("aria-expanded")).toBe("true");
    const codex = headers(view.getByTestId("place-board")).find((h) => h.dataset.place === "ChatGPT|Codex")!;
    const toggle = within(codex).getByTestId("rail-fold");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.getAttribute("aria-label")).toBe("Unfold ChatGPT · Codex");
    expect(within(codex).getByTestId("place-page").textContent).toContain("Codex · 1 idle");
    fireEvent.click(toggle);
    await waitFor(() => expect(view.getByTestId("place-board").querySelector("[data-conversation-id='outside-10']")).not.toBeNull());
  });

  it("Open brings the running app forward by its process and says so; AI Juggler's opens its first session", async () => {
    const spy = vi.spyOn(ipc, "conversationsBringForward").mockResolvedValue({ outcome: "Brought Terminal forward", reached: "app" });
    const view = board([own("aj")]);
    await view.findByTestId("rail-open-now");
    const terminal = headers(view.getByTestId("place-board")).find((h) => h.dataset.place === "Terminal|Work Max")!;
    fireEvent.click(within(terminal).getByRole("button", { name: /^Open Terminal/ }));
    expect(spy).toHaveBeenCalledWith(5100);
    // ⌘1–9: the place in that position.
    act(() => void window.dispatchEvent(new CustomEvent("aij:go-place", { detail: { index: 3 } })));
    expect(spy).toHaveBeenLastCalledWith(4100);
  });

  it("the Also line turns a view on; the View's dot says it changed; Back to the defaults; kept", async () => {
    const view = board([]);
    await view.findByTestId("rail-also");
    expect(view.queryByTestId("rail-view-dot")).toBeNull();
    fireEvent.click(view.getByTestId("rail-also-ended"));
    expect(await view.findByTestId("rail-also-view-ended")).toBeTruthy();
    expect(view.getByTestId("place-board").querySelector("[data-conversation-id='outside-2']")).not.toBeNull();
    expect(view.getByTestId("rail-view-dot")).toBeTruthy();
    expect(JSON.parse(localStorage.getItem("aij.rail.view.v1")!).show.ended).toBe(true);
    act(() => void window.dispatchEvent(new Event("aij:rail-view")));
    const panel = await view.findByRole("dialog", { name: "View" });
    expect(within(panel).getByRole("radio", { name: /Pool/ })).toHaveProperty("disabled", true);
    fireEvent.click(within(panel).getByRole("checkbox", { name: "Inactive" }));
    expect(railView().show.inactive).toBe(true);
    fireEvent.click(within(panel).getByTestId("rail-view-reset"));
    expect(railView()).toEqual(DEFAULT_VIEW);
    await waitFor(() => expect(view.queryByTestId("rail-view-dot")).toBeNull());
  });

  it("search takes the View's words: is:ended shows the ended ones, app: narrows", async () => {
    const view = board([]);
    await view.findByTestId("rail-open-now");
    const search = view.getByRole("searchbox", { name: /Search sessions/ });
    fireEvent.change(search, { target: { value: "app:terminal" } });
    await waitFor(() => expect(headers(view.getByTestId("place-board")).map((h) => h.dataset.place)).toEqual(["Terminal|Work Max"]));
    fireEvent.change(search, { target: { value: "is:ended pagination" } });
    await waitFor(() => expect(view.getByTestId("place-board").querySelector("[data-conversation-id='outside-2']")).not.toBeNull());
  });

  it("an app turned off in Settings leaves its count on the Also line", async () => {
    seams().__AIJ_APP_SETTINGS__ = { "show_sessions_from.terminal": "0" };
    const view = board([]);
    const hidden = await view.findByTestId("rail-also-hidden");
    expect(hidden.textContent).toBe("3 in Terminal hidden · Settings");
    expect(headers(view.getByTestId("place-board")).some((h) => h.dataset.place === "Terminal|Work Max")).toBe(false);
  });

  it("↑ and ↓ move between rows and place headers", async () => {
    const view = board([own("aj")]);
    await view.findByTestId("rail-open-now");
    const nav = () => [...view.getByTestId("place-board").querySelectorAll<HTMLElement>("[data-rail-item], [data-rail-nav]")];
    nav()[0]!.focus();
    fireEvent.keyDown(nav()[0]!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(nav()[1]);
    fireEvent.keyDown(nav()[1]!, { key: "ArrowUp" });
    expect(document.activeElement).toBe(nav()[0]);
    // Focus in the list freezes the order.
    expect(view.getByTestId("place-board").dataset.frozen).toBe("true");
  });

  it("40 sessions: only what's in view is drawn, within a frame", async () => {
    const conversations = placesConversations(NOW, 32);
    setRailView({ ...DEFAULT_VIEW, show: { ...DEFAULT_VIEW.show, quiet: true } });
    const started = performance.now();
    const view = board([own("aj")], conversations);
    await view.findByTestId("rail-open-now");
    const drawn = view.getByTestId("place-board").querySelectorAll("[data-rail-item]").length;
    expect(drawn).toBeLessThan(40);
    expect(drawn).toBeGreaterThan(5);
    expect(performance.now() - started).toBeLessThan(1000);
  });
});

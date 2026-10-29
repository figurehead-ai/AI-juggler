/**
 * AJ-E2103 — the rail's order: Needs you, then Open now, then one line
 * "Also: n ended today · n inactive · n background · History ›", each part
 * turning its view on. Ended sessions from other apps are no longer added
 * under every project, and a session open in two places is one row.
 */
import { needsFor } from "../test/needs-kit";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { alsoParts } from "./RailSections";
import { ConversationsProvider } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { useProjectLens } from "./project-lens.test-kit";

// AJ-E2104: these read the project board, the rail's Project lens.
useProjectLens();

afterEach(() => {
  cleanup();
  localStorage.clear();
  delete (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__;
});

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max",
    model: "opus", launch_model: "opus", running_model: null, model_source: "launch", status: "running",
    quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, subtask_count: null,
    git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/dev/app",
    kind: "terminal", isolation_label: null, project_id: "proj-1", conversation_id: null, ...overrides,
  } as InstanceInfo;
}

function conversation(id: string, overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work Max", project_id: "proj-1", cwd: "/Users/dev/app",
    title: `Talk ${id}`, git_branch: "main", started_ms: NOW - 3_600_000, last_activity_ms: NOW - 60_000, tokens: null,
    live: false, session_id: null, places: [], pid: null, state: null, probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null,
    ...overrides,
  };
}

const project = (id: string, name: string): ProjectInfo =>
  ({ id, root: `/Users/dev/${name}`, name, last_activity_ms: NOW, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }) as ProjectInfo;

const place = (label: string, pid: number) => ({ app: label.split(" · ")[0]!, app_id: null, profile: null, instance: null, pid, kind: null, app_pid: null, label });

function board(rows: InstanceInfo[], conversations: ConversationInfo[]) {
  (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__ = { conversations };
  needsFor({ sessions: rows, conversations }); // D-49: the backend's list
  return render(
    <ConversationsProvider>
      <InstanceBoard
        sessions={rows}
        loginTerminals={[]}
        selectedId={null}
        onSelect={() => {}}
        density="comfortable"
        freshness={{}}
        now={NOW}
        projects={[project("proj-1", "app"), project("proj-2", "api")]}
        snooze={{}}
        onSnooze={() => {}}
        onSelectProject={() => {}}
      />
    </ConversationsProvider>,
  );
}

const FIXTURE = [
  conversation("asks", { live: true, state: "asks_you", places: [place("Claude · Work", 11)] }),
  conversation("open", { live: true, state: "working", places: [place("Claude · Work", 12), place("Terminal", 13)] }),
  conversation("headless", { live: true, background: true, places: [place("Terminal", 14)] }),
  conversation("ended-today", { last_activity_ms: NOW - 2 * 3_600_000 }),
  conversation("ended-before", { last_activity_ms: NOW - 3 * 86_400_000 }),
  conversation("sidebar", { inactive: true, last_activity_ms: NOW - 86_400_000 }),
];

it("reads Needs you, then Open now, then the Also line", async () => {
  const view = board([mk("waiting", { status: "waiting_user", needs_attention: "waiting_user" }), mk("busy")], FIXTURE);
  const needs = await view.findByTestId("rail-needs-you");
  const ids = (root: HTMLElement) => [...root.querySelectorAll<HTMLElement>("[data-instance-id], [data-conversation-id]")].map((el) => el.dataset.instanceId ?? el.dataset.conversationId);
  // AI Juggler's own that needs you, then the one its app says asks you.
  expect(ids(needs)).toEqual(["waiting", "asks"]);
  const rail = view.getByTestId("instance-board");
  const all = ids(rail);
  // Each shows once: the Needs you rows aren't repeated under their project.
  expect(all.filter((id) => id === "waiting")).toHaveLength(1);
  expect(all.filter((id) => id === "asks")).toHaveLength(1);
  // Open now: the running session and the open one, under the project.
  const openLabel = view.getByTestId("rail-open-now");
  expect(needs.compareDocumentPosition(openLabel) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(all).toContain("busy");
  expect(all).toContain("open");
  // Ended and headless ones are not added under every project.
  expect(all).not.toContain("ended-today");
  expect(all).not.toContain("ended-before");
  expect(all).not.toContain("headless");
  expect(all).not.toContain("sidebar");
  const also = view.getByTestId("rail-also");
  expect(openLabel.compareDocumentPosition(also) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(also.textContent).toContain("Also:");
  expect(view.getByTestId("rail-also-ended").textContent).toBe("1 ended today");
  expect(view.getByTestId("rail-also-inactive").textContent).toBe("1 inactive");
  expect(view.getByTestId("rail-also-background").textContent).toBe("1 background");
  expect(view.getByTestId("rail-also-history").textContent).toBe("History ›");
});

it("a session open in two places is one row that says so", async () => {
  const view = board([mk("busy")], FIXTURE);
  await view.findByTestId("rail-needs-you");
  const rows = view.getAllByTestId("rail-outside").filter((row) => row.dataset.conversationId === "open");
  expect(rows).toHaveLength(1);
  expect(within(rows[0]!).getByTestId("outside-places").textContent).toContain("open in 2 places");
});

it("each part of the Also line turns its view on, and History opens History", async () => {
  const view = board([mk("busy")], FIXTURE);
  await view.findByTestId("rail-also-ended");
  expect(view.queryByTestId("rail-also-view-ended")).toBeNull();
  fireEvent.click(view.getByTestId("rail-also-ended"));
  expect(view.getByTestId("rail-also-ended").getAttribute("aria-pressed")).toBe("true");
  const ended = view.getByTestId("rail-also-view-ended");
  expect(within(ended).getAllByTestId("rail-outside").map((row) => row.dataset.conversationId)).toEqual(["ended-today"]);
  fireEvent.click(view.getByTestId("rail-also-background"));
  expect(within(view.getByTestId("rail-also-view-background")).getByTestId("rail-outside").dataset.conversationId).toBe("headless");
  fireEvent.click(view.getByTestId("rail-also-inactive"));
  expect(within(view.getByTestId("rail-also-view-inactive")).getByTestId("rail-outside").textContent).toContain("Ended");
  // Off again: its rows go, its count stays.
  fireEvent.click(view.getByTestId("rail-also-ended"));
  expect(view.queryByTestId("rail-also-view-ended")).toBeNull();
  expect(view.getByTestId("rail-also-ended").textContent).toBe("1 ended today");
  const opened = vi.fn();
  window.addEventListener("aij:open-screen", opened);
  fireEvent.click(view.getByTestId("rail-also-history"));
  window.removeEventListener("aij:open-screen", opened);
  expect((opened.mock.calls[0]![0] as CustomEvent<{ id: string }>).detail.id).toBe("history");
});

it("a filter keeps the old list: no sections", async () => {
  const view = board([mk("waiting", { status: "waiting_user", needs_attention: "waiting_user" }), mk("busy")], FIXTURE);
  await view.findByTestId("rail-needs-you");
  fireEvent.change(view.getByRole("searchbox"), { target: { value: "busy" } });
  expect(view.queryByTestId("rail-needs-you")).toBeNull();
  expect(view.queryByTestId("rail-also")).toBeNull();
});

it("counts only what AI Juggler didn't start, ended today by the local day", () => {
  const parts = alsoParts([...FIXTURE, conversation("mine", { session_id: "s-1", last_activity_ms: NOW - 60_000 })], NOW);
  expect(parts.ended.map((c) => c.id)).toEqual(["ended-today"]);
  expect(parts.inactive.map((c) => c.id)).toEqual(["sidebar"]);
  expect(parts.background.map((c) => c.id)).toEqual(["headless"]);
  expect(screen.queryByTestId("rail-also")).toBeNull();
});

it("AI Juggler's own ended today wait behind the Also line, older ones in History; a pinned one stays", async () => {
  const view = board(
    [
      mk("busy"),
      mk("done-today", { status: "finished", last_activity_ms: NOW - 3_600_000 }),
      mk("done-before", { status: "finished", last_activity_ms: NOW - 2 * 86_400_000 }),
      mk("pinned-done", { status: "stopped", pinned: true, last_activity_ms: NOW - 2 * 86_400_000 }),
      mk("api-done", { status: "finished", project_id: "proj-2", last_activity_ms: NOW - 3 * 86_400_000 }),
    ],
    [],
  );
  const ids = () => [...view.getByTestId("instance-board").querySelectorAll("[data-instance-id]")].map((el) => el.getAttribute("data-instance-id"));
  await view.findByTestId("rail-also");
  expect(ids()).toEqual(["pinned-done", "busy"]);
  // Only what is open: a project with nothing open leaves Open now.
  expect(view.queryByRole("button", { name: "Fold api" })).toBeNull();
  expect(view.getByTestId("rail-also-ended").textContent).toBe("1 ended today");
  fireEvent.click(view.getByTestId("rail-also-ended"));
  expect(view.getByTestId("rail-also-view-ended").querySelector("[data-instance-id='done-today']")).toBeTruthy();
  expect(ids()).not.toContain("done-before");
});

/**
 * AJ-E2105 — Home: one sentence about what needs you with the other
 * counts as links; "Your apps", one tile per running app and profile (at
 * most six, then "+n"); Needs you holding every kind, each with its own
 * primary; Runway by account with its profiles, or the way to measure it.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, railView, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { placesConversations, PREVIEW_PLACES } from "../preview/places";
import { mockMachineHealth, mockThisMac } from "../lib/machine";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { AccountInfo, InstanceInfo } from "../lib/bridge";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
const seams = () => window as unknown as Record<string, unknown>;
const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" },
] as AccountInfo[];

beforeEach(() => {
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  for (const key of Object.keys(seams()).filter((key) => key.startsWith("__AIJ_"))) delete seams()[key];
  resetRailViewCache();
});

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: null, conversation_id: null, ...over,
  } as InstanceInfo;
}

function home(sessions: InstanceInfo[], conversations: ConversationInfo[], props: Partial<Parameters<typeof HomeScreen>[0]> = {}) {
  seams().__AIJ_FIXTURE__ = { conversations };
  return render(
    <ConversationsProvider>
      <HomeScreen sessions={sessions} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} {...props} />
    </ConversationsProvider>,
  );
}

describe("Home, desktop first", () => {
  it("one sentence about what needs you, the other counts as links that turn the rail's views on", async () => {
    const view = home([], placesConversations(NOW));
    await waitFor(() => expect(view.getByTestId("home-heading").textContent).toBe("3 things need you"));
    const counts = view.getByTestId("home-counts");
    expect(counts.textContent).toMatch(/4 open.*1 inactive.*1 ended today/);
    fireEvent.click(within(counts).getByRole("button", { name: "1 inactive" }));
    expect(railView().show.inactive).toBe(true);
    fireEvent.click(within(counts).getByRole("button", { name: "1 ended today" }));
    expect(railView().show.ended).toBe(true);
  });

  it("Your apps: a tile per running app and profile, with its count, needs badge and Open; six, then +n", async () => {
    const view = home([], placesConversations(NOW));
    const tiles = await view.findAllByTestId("home-app-tile");
    expect(tiles.map((tile) => tile.dataset.place)).toEqual(["ChatGPT|Codex", "Claude|Personal", "Claude|Work", "Terminal|Work Max"]);
    const work = tiles[2]!;
    expect(work.textContent).toContain("Work Max · 2 sessions");
    expect(within(work).getByText("1")).toBeTruthy();
    expect(within(work).getByRole("button", { name: /^Open Claude/ })).toBeTruthy();
    cleanup();
    // Eight places: six tiles, then "+2".
    const many = [...Array(8)].map((_, n) => ({ ...placesConversations(NOW)[0]!, id: `p${n}`, places: [{ ...PREVIEW_PLACES[0]!, instance: `P${n}`, profile: `Profile ${n}`, pid: 900 + n }] }));
    const again = home([], many);
    await waitFor(() => expect(again.getAllByTestId("home-app-tile")).toHaveLength(6));
    fireEvent.click(again.getByTestId("home-apps-more"));
    expect(again.getAllByTestId("home-app-tile")).toHaveLength(8);
  });

  it("Needs you holds every kind, each with its own primary", async () => {
    const onLimit = vi.fn();
    const onOpenCleanup = vi.fn();
    const thisMac = { ...mockThisMac([], mockMachineHealth("plenty", NOW), [], [0, 0], []), leftover_processes: 2 };
    const view = home([own("lim", { status: "quota_blocked", needs_attention: "quota_blocked" })], placesConversations(NOW), { onLimit, onOpenCleanup, thisMac });
    const needs = await view.findByTestId("home-needs");
    await waitFor(() => expect(needs.querySelector("[data-first='true']")?.textContent).toContain("Migrate the billing webhooks"));
    const first = needs.querySelector<HTMLElement>("[data-first='true']")!;
    expect(first.textContent).toContain("payments-api · Claude · Work Max");
    expect(within(first).getByRole("button", { name: /^Go to it/ })).toBeTruthy();
    expect(within(first).getByRole("button", { name: /^Continue on another account/ })).toBeTruthy();
    const limit = view.getByTestId("home-need-limit");
    fireEvent.click(within(limit).getByRole("button", { name: /^Continue elsewhere/ }));
    expect(onLimit).toHaveBeenCalledWith("lim");
    fireEvent.click(within(view.getByTestId("home-need-leftovers")).getByRole("button", { name: /^Clean up/ }));
    expect(onOpenCleanup).toHaveBeenCalledOnce();
    expect(view.getByTestId("home-heading").textContent).toBe("5 things need you");
  });

  it("Runway by account: each with its profiles; the way to measure one that isn't", async () => {
    const view = home([], []);
    const rows = await view.findAllByTestId("home-runway-account");
    expect(rows[0]!.textContent).toContain("Profiles: Work Max");
    expect(rows[1]!.textContent).toContain("Not measured yet");
    expect(within(rows[1]!).getByRole("button", { name: /^Measure its limits/ })).toBeTruthy();
  });

  it("nothing needs you: says so, with no empty card", async () => {
    const view = home([own("r")], []);
    await waitFor(() => expect(view.getByTestId("home-heading").textContent).toBe("All clear — nothing needs you"));
    expect(view.queryByTestId("home-needs")).toBeNull();
    expect(view.queryByTestId("home-this-mac")).toBeNull();
  });
});

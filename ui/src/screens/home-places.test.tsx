/**
 * AJ-E2105 — Home: one sentence about what needs you with the other
 * counts as links; "Your apps", one tile per running app and profile (at
 * most six, then "+n"); Needs you holding every kind, each with its own
 * primary; Runway by account with its profiles, or the way to measure it.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ActionHostProvider } from "../components/ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, railView, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { loadSubscriptions } from "../lib/subscriptions";
import { placesConversations, PREVIEW_PLACES } from "../preview/places";
import { mockMachineHealth } from "../preview/machine";
import { mockThisMac } from "../preview/this-mac";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { needsFor } from "../test/needs-kit";

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
  needsFor({ sessions, conversations, accounts: ACCOUNTS, thisMac: props.thisMac ?? null });
  return render(
    <ConversationsProvider>
      <HomeScreen sessions={sessions} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} {...props} />
    </ConversationsProvider>,
  );
}

describe("Home, desktop first", () => {
  it("one sentence about what needs you, the other counts as links that turn the rail's views on", async () => {
    const view = home([], placesConversations(NOW));
    // D-117: the two that ask are counted; the one done, to review, is
    // listed and named in the counts line, never counted.
    await waitFor(() => expect(view.getByTestId("home-heading").textContent).toBe("2 sessions need you"));
    const counts = view.getByTestId("home-counts");
    // D-47: open counts every open session, the ones that need you too
    // (they stay in their places in Open now); Needs you counts them once.
    expect(counts.textContent).toMatch(/7 open.*1 to review.*1 inactive.*1 ended today/);
    fireEvent.click(within(counts).getByRole("button", { name: "1 inactive session" }));
    expect(railView().show.inactive).toBe(true);
    fireEvent.click(within(counts).getByRole("button", { name: "1 ended today" }));
    expect(railView().show.ended).toBe(true);
  });

  it("Your apps (D-118): a compact row per running app and profile, most recently used first, with its count, needs mark and Open; eight, then Show all", async () => {
    const view = home([], placesConversations(NOW));
    const rows = await view.findAllByTestId("home-app-tile");
    expect(new Set(rows.map((row) => row.dataset.place))).toEqual(new Set(["app|codex-app|", "app|claude-app|Personal", "app|claude-app|Work", "cli|claude-code|claude-work"]));
    const work = rows.find((row) => row.dataset.place === "app|claude-app|Work")!;
    expect(work.textContent).toContain("Work Max·2 open");
    expect(within(work).getByTestId("home-app-needs").textContent).toBe("1 needs you");
    expect(within(work).getByRole("button", { name: /^Open Claude/ })).toBeTruthy();
    // The rows sit in the right column, under its own head.
    expect(view.getByTestId("home-side").contains(work)).toBe(true);
    cleanup();
    // Ten places: eight rows, then "Show all 10".
    const places = (count: number) => [...Array(count)].map((_, n) => ({ ...placesConversations(NOW)[0]!, id: `p${n}`, places: [{ ...PREVIEW_PLACES[0]!, instance: `P${n}`, profile: `Profile ${n}`, pid: 900 + n }] }));
    const again = home([], places(10));
    await waitFor(() => expect(again.getAllByTestId("home-app-tile")).toHaveLength(8));
    expect(again.getByTestId("home-apps-more").textContent).toBe("Show all 10");
    fireEvent.click(again.getByTestId("home-apps-more"));
    expect(again.getAllByTestId("home-app-tile")).toHaveLength(10);
    expect(again.getByTestId("home-apps-more").textContent).toBe("Show fewer");
    cleanup();
    // Nine: all nine, never one behind a "Show all" (the 0.2 walk, F18).
    const nine = home([], places(9));
    await waitFor(() => expect(nine.getAllByTestId("home-app-tile")).toHaveLength(9));
    expect(nine.queryByTestId("home-apps-more")).toBeNull();
  });

  it("Needs you holds every kind, each with its own primary", async () => {
    const onLimit = vi.fn();
    const onOpenCleanup = vi.fn();
    const thisMac = { ...mockThisMac([], mockMachineHealth("plenty", NOW), [], [0, 0], []), leftover_processes: 2 };
    const view = home([own("lim", { status: "quota_blocked", needs_attention: "quota_blocked" })], placesConversations(NOW), { onLimit, onOpenCleanup, thisMac });
    const needs = await view.findByTestId("home-needs");
    await waitFor(() => expect(needs.querySelector("[data-first='true']")?.textContent).toContain("Migrate the billing webhooks"));
    const first = needs.querySelector<HTMLElement>("[data-first='true']")!;
    expect(first.textContent).toContain("payments-api · Claude app · Work Max");
    expect(within(first).getByRole("button", { name: /^Open its page/ })).toBeTruthy();
    expect(within(first).getByRole("button", { name: /^Continue in another app/ })).toBeTruthy();
    const limit = view.getByTestId("home-need-limit");
    fireEvent.click(within(limit).getByRole("button", { name: /^Continue in another app/ }));
    expect(onLimit).toHaveBeenCalledWith("lim");
    // D-118: the pinned zone holds L2 and L3 only: the review is the
    // feed's Finished filter, the leftovers the top bar's Clean up.
    expect(view.queryByTestId("home-need-leftovers")).toBeNull();
    expect(onOpenCleanup).not.toHaveBeenCalled();
    expect(view.getByTestId("home-heading").textContent).toBe("3 sessions need you");
    expect(within(view.getByTestId("home-needs-now-section")).getByTestId("section-count").textContent).toBe("3");
    expect(view.getByTestId("home-counts").textContent).toContain("2 to review");
  });

  it("a session's need opens its page; its ↗ (and ⌘↩) goes to its app, and only while it's open", async () => {
    const goToConversation = vi.fn();
    const openConversation = vi.fn();
    seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
    needsFor({ sessions: [own("aj", { status: "waiting_user", needs_attention: "waiting_user" })], conversations: placesConversations(NOW), accounts: ACCOUNTS });
    const view = render(
      <ConversationsProvider>
        <ActionHostProvider host={{ goToConversation, openConversation }}>
          <HomeScreen sessions={[own("aj", { status: "waiting_user", needs_attention: "waiting_user" })]} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    const needs = await view.findByTestId("home-needs");
    await waitFor(() => expect(needs.querySelector("[data-first='true']")?.textContent).toContain("Migrate the billing webhooks"));
    const first = needs.querySelector<HTMLElement>("[data-first='true']")!;
    fireEvent.click(within(first).getByRole("button", { name: /^Open its page/ }));
    expect(openConversation).toHaveBeenCalledWith(expect.objectContaining({ id: "outside-3" }));
    expect(goToConversation).not.toHaveBeenCalled();
    // Always shown on a need, named for where it goes.
    const arrow = within(first).getByRole("button", { name: "Go to it in Claude app · Work Max" });
    fireEvent.click(arrow);
    expect(goToConversation).toHaveBeenCalledTimes(1);
    expect(openConversation).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(within(first).getByRole("button", { name: /^Open its page/ }), { key: "Enter", metaKey: true });
    expect(goToConversation).toHaveBeenCalledTimes(2);
    // AI Juggler's own session: its page is where it runs, so no ↗.
    // Named by the shared short name (never "main", the rc12 walk; D-96:
    // by its facts, folder · when it started; D-103.8: never its Account).
    const own_ = [...needs.querySelectorAll<HTMLElement>("[data-home-item]")].find((item) => /ai-juggler · started /.test(item.textContent ?? ""))!;
    expect(own_).toBeTruthy();
    expect(within(own_).queryByRole("button", { name: /^Go to it/ })).toBeNull();
  });

  it("Runway by account: each with its profiles; the way to measure one that isn't", async () => {
    // 0.4.4: Measure its limits asks the tool here (D-39), when it's on.
    seams().__AIJ_PROBE__ = { "claude-personal": { offered: true, state: "on", last_run_ms: null, last_outcome: null } };
    const view = home([], []);
    const rows = await view.findAllByTestId("home-runway-account");
    expect(rows[0]!.textContent).toContain("Apps: Claude Code CLI · Work Max");
    expect(rows[1]!.textContent).toContain("Not measured yet");
    // Read again: the rows are drawn anew once the readings answer.
    await waitFor(() => expect(within(view.getAllByTestId("home-runway-account")[1]!).getByRole("button", { name: /^Measure its limits/ })).toBeTruthy());
  });

  it("Runway by account groups the profiles signed in to one account", async () => {
    seams().__AIJ_FIXTURE__ = {
      conversations: [],
      subscriptions: [
        { key: "acct-1", tool: "claude-code", state: "signed_in", email: "o@work.com", organisation: null, reason: null, plan: "Max", renewal_date: null, fee_usd: null, differs: [],
          profiles: ["claude-work", "claude-personal"].map((id) => ({ id, label: id, folder: id, plan: "Max", renewal_date: null, fee_usd: null })) },
      ],
    };
    await loadSubscriptions(true); // the list is cached for half a minute
    const view = render(
      <ConversationsProvider>
        <HomeScreen sessions={[]} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
      </ConversationsProvider>,
    );
    await waitFor(() => expect(view.getAllByTestId("home-runway-account")).toHaveLength(1));
    const row = view.getByTestId("home-runway-account");
    expect(row.textContent).toContain("Apps: Claude Code CLI · Work Max, Claude Code CLI · Personal");
    expect(row.textContent).toContain("o@work.com");
  });

  it("an App's room is Runway by account's alone (D-118): its row never repeats it", async () => {
    seams().__AIJ_QUOTA_DISPLAY__ = [{ account: "claude-work", five_hour: { percentage: 40, state: "measured", resets_at_ms: null }, weekly: null }];
    const measured = home([], placesConversations(NOW));
    await waitFor(() => expect(within(measured.getByTestId("home-runway")).getByText("60% left")).toBeTruthy());
    for (const row of measured.getAllByTestId("home-app-tile")) expect(row.textContent).not.toMatch(/% left|not measured/i);
  });

  it("the pinned zone and the feed share the left column; Apps and Runway the right (D-118 R1)", async () => {
    const ended = own("done", { status: "finished", last_activity_ms: NOW - 60_000 });
    const view = home([own("f1", { status: "failed" }), ended], []);
    const main = await view.findByTestId("home-main");
    expect(main.contains(await view.findByTestId("home-needs-now-section"))).toBe(true);
    expect(main.contains(view.getByTestId("home-feed"))).toBe(true);
    const side = view.getByTestId("home-side");
    expect(side.tagName).toBe("ASIDE");
    expect(side.getAttribute("aria-label")).toBe("Apps and runway");
    expect(side.contains(view.getByTestId("home-runway"))).toBe(true);
    expect(view.queryByTestId("home-recent")).toBeNull();
  });

  it("stacks under about 1,000 px, the feed first, with Apps & runway ↓ in the head; a strip of App icons under 640 (AJ-E3701 R4)", async () => {
    const width = (px: number) => vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: px, height: 800, top: 0, left: 0, right: px, bottom: 800, x: 0, y: 0, toJSON: () => ({}) });
    const spy = width(900);
    const view = home([], placesConversations(NOW));
    const root = await view.findByTestId("home");
    expect(root.dataset.layout).toBe("stacked");
    expect(view.getByTestId("home-side-link").textContent).toBe("Apps & runway ↓");
    // The feed first, then Apps and Runway below it.
    expect(view.getByTestId("home-main").compareDocumentPosition(view.getByTestId("home-side")) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(await view.findAllByTestId("home-app-tile")).not.toHaveLength(0);
    spy.mockRestore();
    cleanup();
    const narrow = width(600);
    const strip = home([], placesConversations(NOW));
    expect(await strip.findByTestId("home-apps-strip")).toBeTruthy();
    narrow.mockRestore();
  });

  it("nothing needs you: says so, with no empty card", async () => {
    const view = home([own("r")], []);
    await waitFor(() => expect(view.getByTestId("home-heading").textContent).toBe("All clear — nothing needs you"));
    expect(view.queryByTestId("home-needs")).toBeNull();
    // D-118: the pinned zone says it in one muted line.
    expect(view.getByTestId("home-pinned-none").textContent).toBe("Nothing needs you");
    expect(view.queryByTestId("home-this-mac")).toBeNull();
  });
});

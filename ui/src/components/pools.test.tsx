/**
 * AJ-E1702 / AJ-E1703 vitest — pools in the rail, Home and the sheets,
 * agent access (D-28/D-29/D-32), and the pool page's cards.
 *
 * - The rail: the grouping switch offers Pool only once you've made a
 *   pool (0.3); then Open now groups by pool and a pool's header opens its
 *   page. There is no separate pools section.
 * - A session's menu: "Move to pool…" in Working on it, with its icon.
 * - The sheets: make a pool (the session goes in, its page opens), move a
 *   session (a taken name refused in words). Renaming, roles and deleting
 *   are the canvas's (pages/pool/canvas.test.tsx).
 * - Agent access: the level picker, Custom's actions (never delivery),
 *   the hop limit and stop-all, rules taken back; Settings lists every
 *   pool.
 * - Cards: rows like Home's (the rows' place line, the rail's state
 *   words), the rest behind a disclosure; unknown fields
 *   say Unavailable; the agent's words are marked as its own.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS, MOCK_SESSIONS, type InstanceInfo } from "../lib/bridge";
import { actionSpecs } from "../lib/actions";
import { poolMembers, poolsList, resetPreviewPools, type ContextCard, type PoolMemberRow, type PoolsInfo } from "../lib/ipc/pools";
import { forgetPaged, type PagedList } from "../lib/paged";
import { openPoolSheet, refreshPools, resetPools, usePoolPlace } from "../lib/pools";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, railView, resetRailViewCache, setRailView } from "../lib/rail-view";
import { ActionHostProvider } from "./ItemMenu";
import { sessionState } from "../lib/rail-items";
import { PoolAccessPanel } from "./PoolAccess";
import { AgentAccessCard } from "../screens/settings/AgentAccessPane";
import { PoolCardRows } from "./PoolCards";
import { sessionTitle } from "./ManageWork";
import { PoolSheet } from "./PoolSheet";
import { InstanceBoard } from "./WorkflowRail";
import { pick } from "../test/pick";

type Seams = { __AIJ_POOLS__?: unknown; __AIJ_POOL_CALLS__?: string[]; __AIJ_POOL_ACTIVITY__?: unknown };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_POOL_CALLS__ ?? [];
const [first, second, third] = MOCK_SESSIONS as [InstanceInfo, InstanceInfo, InstanceInfo];

beforeEach(() => {
  resetPreviewPools();
  resetPools();
  forgetPaged();
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  delete seams().__AIJ_POOLS__;
  delete seams().__AIJ_POOL_CALLS__;
  delete seams().__AIJ_POOL_ACTIVITY__;
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  resetRailViewCache();
});

function seedCheckout(): void {
  seams().__AIJ_POOLS__ = {
    pools: [
      { id: "pool-checkout", name: "Checkout redesign" },
      { id: "pool-docs", name: "Docs" },
    ],
    members: [
      { session_id: first.id, pool_id: "pool-checkout", role: "lead" },
      { session_id: second.id, pool_id: "pool-checkout", role: "member" },
    ],
  };
}

async function loaded(): Promise<PoolsInfo> {
  await act(async () => {
    await refreshPools();
  });
  return poolsList();
}

describe("pools in the rail: the Pool lens", () => {
  const board = (host = {}) =>
    render(
      <ConversationsProvider>
        <ActionHostProvider host={host}>
          <InstanceBoard
            sessions={MOCK_SESSIONS}
            loginTerminals={[]}
            selectedId={null}
            onSelect={() => {}}
            density="compact"
            freshness={{}}
            now={Date.now()}
            projects={MOCK_PROJECTS}
            snooze={{}}
            onSnooze={() => {}}
          />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
  // 0.3: the rail's grouping switch (no longer in the View).
  const groupSwitch = (view: ReturnType<typeof board>) => view.getByRole("radiogroup", { name: "Group sessions by" });

  it("waits until a pool of yours exists; the rail shows places meanwhile", async () => {
    act(() => setRailView({ ...DEFAULT_VIEW, group: "pool" }));
    const view = board();
    await loaded();
    expect(within(groupSwitch(view)).queryByRole("radio", { name: "Pool" })).toBeNull();
    expect(within(groupSwitch(view)).getByRole("radio", { name: "App" }).getAttribute("aria-checked")).toBe("true");
    expect(view.getAllByTestId("place-header").some((header) => header.textContent?.includes("AI Juggler"))).toBe(true);
  });

  it("groups Open now by pool, and a pool's header opens its page", async () => {
    seedCheckout();
    const opened: string[] = [];
    const view = board({ openPool: (id: string) => opened.push(id) });
    await loaded();
    const radio = await within(groupSwitch(view)).findByRole("radio", { name: "Pool" });
    fireEvent.click(radio);
    expect(railView().group).toBe("pool");
    const open = await view.findByRole("button", { name: /^Open Checkout redesign/ });
    const names = view.getAllByTestId("place-header").map((header) => header.textContent ?? "");
    expect(names.some((text) => text.startsWith("Checkout redesign"))).toBe(true);
    // No separate pools section: an empty pool isn't in Open now.
    expect(names.some((text) => text.startsWith("Docs"))).toBe(false);
    fireEvent.click(open);
    expect(opened).toEqual(["pool-checkout"]);
  });

  it("offers Move to pool… on a session's menu, in Working on it, with its own icon", () => {
    const spec = actionSpecs("session").find((entry) => entry.id === "pool")!;
    expect(spec.label).toBe("Move to pool…");
    expect(spec.group).toBe("work");
    expect(spec.icon).toBe("pool");
    expect(spec.applies(first, {})).toBe(false);
    expect(spec.applies(first, { moveSessionToPool: () => {} })).toBe(true);
  });
});

describe("instant and paged (I6)", () => {
  it("names only the places apart from a project's default, and counts each pool", async () => {
    seedCheckout();
    const pools = await poolsList();
    expect(pools.placements).toEqual([
      { session_id: first.id, pool_id: "pool-checkout", role: "lead" },
      { session_id: second.id, pool_id: "pool-checkout", role: "member" },
    ].sort((a, b) => a.session_id.localeCompare(b.session_id)));
    expect(pools.pools.find((pool) => pool.id === "pool-checkout")?.members).toBe(2);
    expect(pools.pools.find((pool) => pool.id === "pool-docs")?.members).toBe(0);
  });

  it("reads a pool's members a page at a time, its lead first", async () => {
    seedCheckout();
    const page = await poolMembers("pool-checkout", { cursor: null, limit: 1 });
    expect(page.items.map((row) => [row.session.id, row.card.role])).toEqual([[first.id, "lead"]]);
    expect(page.total).toBe(2);
    const rest = await poolMembers("pool-checkout", { cursor: page.next_cursor, limit: 1 });
    expect(rest.items.map((row) => row.session.id)).toEqual([second.id]);
    expect(rest.next_cursor).toBeNull();
  });

  it("reads one session's place alone, and the rail's lookups see it", async () => {
    seedCheckout();
    await loaded();
    const seen: (string | null)[] = [];
    function Probe(): null {
      seen.push(usePoolPlace(first.id)?.pool_id ?? null);
      return null;
    }
    render(<Probe />);
    await waitFor(() => expect(seen.at(-1)).toBe("pool-checkout"));
  });
});

describe("the pool sheets", () => {
  it("makes a pool with the session in it, then opens its page (the rail's lens stays as you set it)", async () => {
    const pools = await poolsList();
    const opened: string[] = [];
    const { getByTestId } = render(
      <ActionHostProvider host={{ openPool: (id: string) => opened.push(id) }}>
        <PoolSheet target={{ mode: "create", sessionId: first.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />
      </ActionHostProvider>,
    );
    expect(getByTestId("pool-create-session").textContent).toContain("goes in it");
    fireEvent.change(getByTestId("pool-create-name"), { target: { value: "  Checkout redesign " } });
    fireEvent.click(getByTestId("pool-create-save"));
    await waitFor(() => expect(calls()).toEqual(["create:Checkout redesign", `move:${first.id}:pool-preview-1`]));
    await waitFor(() => expect(opened).toEqual(["pool-preview-1"]));
    expect(railView().group).toBe("place");
  });

  it("refuses a taken name in words", async () => {
    seedCheckout();
    const pools = await poolsList();
    const { getByTestId, findByTestId } = render(<PoolSheet target={{ mode: "create" }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    fireEvent.change(getByTestId("pool-create-name"), { target: { value: "docs" } });
    fireEvent.click(getByTestId("pool-create-save"));
    expect((await findByTestId("pool-error")).textContent).toBe("There's already a pool called «Docs» — pick another name");
  });

  it("moves a session: its pool marked, one click elsewhere", async () => {
    seedCheckout();
    const pools = await loaded();
    const { getAllByTestId } = render(<PoolSheet target={{ mode: "move", sessionId: first.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    const choices = getAllByTestId("pool-move-choice");
    const here = choices.find((choice) => choice.getAttribute("aria-current") === "true")!;
    expect(here.textContent).toContain("Checkout redesign");
    fireEvent.click(choices.find((choice) => choice.textContent?.startsWith("Docs"))!);
    await waitFor(() => expect(calls()).toEqual([`move:${first.id}:pool-docs`]));
  });

  it("offers a session its own project's default pool, never another's", async () => {
    seedCheckout();
    const pools = await loaded();
    const { getAllByTestId } = render(<PoolSheet target={{ mode: "move", sessionId: third.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    const names = getAllByTestId("pool-move-choice").map((choice) => choice.textContent);
    expect(names).toEqual(["Checkout redesign", "Docs", "payments-api · all sessionsIt's here"]);
  });

  it("moving a session with no pool of yours yet makes one", async () => {
    const pools = await loaded();
    const { getByTestId, queryByTestId } = render(<PoolSheet target={{ mode: "move", sessionId: first.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    expect(queryByTestId("pool-move-list")).toBeNull();
    expect(getByTestId("pool-create-session").textContent).toContain("goes in it");
  });

  it("opens from anywhere and closes", () => {
    act(() => openPoolSheet({ mode: "create" }));
    act(() => openPoolSheet(null));
  });
});

describe("agent access", () => {
  it("picks a level, and Custom never offers delivery", async () => {
    const pools = await loaded();
    const pool = pools.pools[0]!;
    const { getByTestId, rerender, queryByText } = render(<PoolAccessPanel pool={pool} />);
    expect((within(getByTestId("pool-level-work_together")).getByRole("radio") as HTMLInputElement).checked).toBe(true);
    fireEvent.click(within(getByTestId("pool-level-custom")).getByRole("radio"));
    await waitFor(() => expect(calls()[0]).toMatch(/^access:pool-default:proj-ai-juggler:custom:/));
    rerender(<PoolAccessPanel pool={(await poolsList()).pools[0]!} />);
    expect(getByTestId("pool-custom")).toBeTruthy();
    expect(queryByText("Put a message straight into another session")).toBeNull();
  });

  it("sets the hop limit and the stop-all, and takes a rule back", async () => {
    seams().__AIJ_POOLS__ = {
      pools: [{ id: "pool-team", name: "Team", rules: [{ id: 7, pool_id: "pool-team", action: "assign_task", created_at: Date.now(), made_by: "user" }] }],
    };
    const pool = (await loaded()).pools[0]!;
    const { getByTestId } = render(<PoolAccessPanel pool={pool} />);
    pick(getByTestId("pool-hop-limit"), "3");
    await waitFor(() => expect(calls()).toContain("delivery:pool-team:3:false"));
    expect(getByTestId("pool-rule").textContent).toContain("Give a task to someone else");
    fireEvent.click(getByTestId("pool-rule-revoke"));
    await waitFor(() => expect(calls()).toContain("revoke:7"));
  });

  it("lists every pool on Settings › Agent access", async () => {
    seedCheckout();
    const { findAllByTestId } = render(<AgentAccessCard />);
    await loaded();
    const rows = await findAllByTestId("agent-access-pool");
    const said = (row: HTMLElement) => [row.firstElementChild!.firstElementChild!.textContent, within(row).getByTestId("agent-access-level").getAttribute("aria-expanded")];
    expect(rows.map(said)).toEqual([
      ["Checkout redesign", "false"],
      ["Docs", "false"],
      ["ai-juggler · all sessions", "false"],
      ["payments-api · all sessions", "false"],
      ["No project · all sessions", "false"],
    ]);
    expect(rows.every((row) => within(row).getByTestId("agent-access-level").textContent!.startsWith("Work together"))).toBe(true);
  });

  /** The 0.2 walk (F10): a row's name opens the pool's page; its level
   * unfolds in place; a long list comes a page at a time. */
  it("opens a pool's page from its row, and pages a long list", async () => {
    seams().__AIJ_POOLS__ = { pools: Array.from({ length: 30 }, (_, i) => ({ id: `pool-${i}`, name: `Pool ${String(i).padStart(2, "0")}` })), members: [] };
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => opened.push(id) }}>
        <AgentAccessCard />
      </ActionHostProvider>,
    );
    await loaded();
    const rows = await view.findAllByTestId("agent-access-pool");
    expect(rows).toHaveLength(12);
    fireEvent.click(within(rows[1]!).getByTestId("agent-access-open"));
    expect(opened).toEqual(["pool-1"]);
    expect(within(rows[1]!).getByTestId("agent-access-open").textContent).toBe("Pool 01: open its page");
    // The level unfolds in place, and folds again.
    const level = within(rows[0]!).getByTestId("agent-access-level");
    fireEvent.click(level);
    expect(level.getAttribute("aria-expanded")).toBe("true");
    expect(rows[0]!.querySelector("[id^='agent-access-panel-']")).toBeTruthy();
    // The rest, a page at a time (the projects' own pools grow with them).
    const more = view.getByTestId("agent-access-more");
    expect(more.textContent).toContain("of 21");
    fireEvent.click(more);
    expect(view.getAllByTestId("agent-access-pool").length).toBeGreaterThan(12);
  });
});

describe("context cards", () => {
  const card = (over: Partial<ContextCard>): ContextCard => ({
    session_id: first.id,
    name: "Split checkout",
    place: "shop · AI Juggler · Work Max",
    tool: "Claude Code",
    account: "Work Max",
    state: "running",
    role: "lead",
    task: { value: "Split the checkout (in progress)", unavailable: null },
    said: { value: "Split checkout into steps", unavailable: null },
    last_activity_ms: Date.now() - 120_000,
    files: ["src/cart.ts", "src/pay.ts"],
    files_unavailable: null,
    ...over,
  });
  /** A read list of these cards' members, as `usePoolMembers` gives it. */
  const listOf = (cards: ContextCard[]): PagedList<PoolMemberRow> => ({
    items: cards.map((entry) => ({ session: MOCK_SESSIONS.find((session) => session.id === entry.session_id)!, card: entry })),
    total: cards.length,
    loading: false,
    stale: false,
    loadingMore: false,
    hasMore: false,
    error: null,
    loadMore: () => {},
    refresh: () => {},
  });

  it("shows a row like Home's, and the rest behind its disclosure", () => {
    const { getByTestId, getByLabelText } = render(<PoolCardRows list={listOf([card({})])} sessions={MOCK_SESSIONS} now={Date.now()} />);
    // The session's one title, as the rail names it (not the card's own
    // fallback, which said "its folder").
    expect(getByTestId("pool-card-name").textContent).toBe(`${sessionTitle(first)} · ${sessionState(first).word}`);
    // D-36: the rows' place line; the tool beside it (the line names the profile).
    expect(getByTestId("pool-card").textContent).toContain("shop · AI Juggler · Work Max");
    expect(getByTestId("pool-card").textContent).toContain("Claude Code");
    // AJ-E2104: the rail's one state vocabulary.
    expect(getByTestId("pool-card-state").textContent).toBe(sessionState(first).word);
    const more = getByLabelText(`Show what ${sessionTitle(first)} is doing`);
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(getByTestId("pool-card-said").textContent).toBe("“Split checkout into steps”said by the agent");
    expect(getByTestId("pool-card-files").textContent).toBe("src/cart.tssrc/pay.ts");
    expect(getByTestId("pool-card-detail").textContent).toContain("Split the checkout (in progress)");
  });

  it("says Unavailable and why, never an empty guess", () => {
    const unknown = card({
      said: { value: null, unavailable: "Unavailable — read only while it runs in AI Juggler" },
      files: [],
      files_unavailable: "Unavailable — read only while it runs in AI Juggler",
    });
    const { getByLabelText, getByTestId } = render(<PoolCardRows list={listOf([unknown])} sessions={MOCK_SESSIONS} now={Date.now()} />);
    fireEvent.click(getByLabelText(/^Show what/));
    expect(getByTestId("pool-card-said").textContent).toBe("Unavailable — read only while it runs in AI Juggler");
    expect(getByTestId("pool-card-files").textContent).toBe("Unavailable — read only while it runs in AI Juggler");
  });

  it("says so when a pool is empty", () => {
    const { getByTestId } = render(<PoolCardRows list={listOf([])} sessions={[]} now={Date.now()} />);
    expect(getByTestId("pool-cards-empty")).toBeTruthy();
  });
});

describe("the pool page's count (the 0.2 walk: \"0 sessions\" beside three open in the Claude app)", () => {
  it("says what it counts: AI Juggler's sessions, and the project's open in other apps beside them", async () => {
    const { poolCountWords } = await import("../pages/PoolPage");
    expect(poolCountWords(0, 3)).toBe("0 sessions in AI Juggler · 3 open in other apps");
    expect(poolCountWords(1, 0)).toBe("1 session in AI Juggler");
  });
});

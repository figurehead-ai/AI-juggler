/**
 * AJ-E1702 / AJ-E1703 vitest — pools in the rail, Home and the sheets,
 * agent access (D-28/D-29/D-32), and the pool page's cards.
 *
 * - The rail: Group: Pool and the pool filter appear once you've made a
 *   pool; pool groups keep a place for an empty pool; a pool's header
 *   opens its page.
 * - A session's menu: "Move to pool…" opens the move sheet.
 * - The sheets: make a pool (the session goes in), move a session, rename
 *   (a taken name refused in words), "All sessions" can't be renamed or
 *   deleted, deleting says what happens first.
 * - Agent access: the level picker, Custom's actions (never delivery),
 *   the hop limit and stop-all, rules taken back; Settings lists every
 *   pool.
 * - Cards: rows like Home's, the rest behind a disclosure; unknown fields
 *   say Unavailable; the agent's words are marked as its own.
 * - Home: one pool's sessions only, remembered.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS, MOCK_SESSIONS, type InstanceInfo } from "../lib/bridge";
import { actionSpecs } from "../lib/actions";
import { DEFAULT_POOL_ID, poolsList, resetPreviewPools, type ContextCard, type PoolsInfo } from "../lib/ipc/pools";
import { openPoolSheet, refreshPools, resetPools, setPools } from "../lib/pools";
import { groupSessions } from "../lib/railview";
import { useHomePool } from "./HomePoolFilter";
import { ActionHostProvider } from "./ItemMenu";
import { AgentAccessCard, PoolAccessPanel } from "./PoolAccess";
import { PoolCardRows } from "./PoolCards";
import { PoolSheet } from "./PoolSheet";
import { InstanceBoard } from "./WorkflowRail";

type Seams = { __AIJ_POOLS__?: unknown; __AIJ_POOL_CALLS__?: string[]; __AIJ_POOL_ACTIVITY__?: unknown };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_POOL_CALLS__ ?? [];
const [first, second, third] = MOCK_SESSIONS as [InstanceInfo, InstanceInfo, InstanceInfo];

beforeEach(() => {
  resetPreviewPools();
  resetPools();
  delete seams().__AIJ_POOLS__;
  delete seams().__AIJ_POOL_CALLS__;
  delete seams().__AIJ_POOL_ACTIVITY__;
});
afterEach(() => {
  cleanup();
  localStorage.clear();
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

describe("pools in the rail", () => {
  const board = (host = {}) =>
    render(
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
      </ActionHostProvider>,
    );

  it("stays out of sight until a pool exists", async () => {
    const { container, queryByLabelText } = board();
    await loaded();
    expect(queryByLabelText("Filter by pool")).toBeNull();
    expect([...container.querySelectorAll("select[aria-label='Group by'] option")].map((o) => o.textContent)).not.toContain("Group: Pool");
  });

  it("groups by pool, keeps a place for an empty pool and opens a pool's page", async () => {
    seedCheckout();
    const opened: string[] = [];
    const { getByLabelText, getAllByTestId, getByText } = board({ openPool: (id: string) => opened.push(id) });
    await loaded();
    fireEvent.click(getByLabelText(/^Filters/));
    fireEvent.change(getByLabelText("Group by"), { target: { value: "pool" } });
    const headers = getAllByTestId("rail-pool").map((header) => header.textContent ?? "");
    expect(headers.some((text) => text.startsWith("Checkout redesign"))).toBe(true);
    expect(headers.some((text) => text.startsWith("Docs") && text.endsWith("0"))).toBe(true);
    expect(headers.some((text) => text.startsWith("All sessions"))).toBe(true);
    fireEvent.click(getAllByTestId("rail-pool").find((header) => header.textContent?.startsWith("Checkout"))!);
    expect(opened).toEqual(["pool-checkout"]);
    expect(getByText("New pool")).toBeTruthy();
  });

  it("filters to one pool", async () => {
    seedCheckout();
    const { getByLabelText, container } = board();
    await loaded();
    fireEvent.click(getByLabelText(/^Filters/));
    fireEvent.change(getByLabelText("Filter by pool"), { target: { value: "pool-checkout" } });
    const ids = [...container.querySelectorAll("[data-instance-id]")].map((row) => row.getAttribute("data-instance-id"));
    expect(new Set(ids)).toEqual(new Set([first.id, second.id]));
    expect(getByLabelText("Filters (some on)")).toBeTruthy();
  });

  it("groups pure: the order rows bring, empty pools after", async () => {
    seedCheckout();
    const info = await poolsList();
    const groups = groupSessions([third, first, second], "pool", MOCK_PROJECTS, info);
    expect(groups.map((group) => group.label)).toEqual(["All sessions", "Checkout redesign", "Docs"]);
    expect(groups[1]!.rows.map((row) => row.id)).toEqual([first.id, second.id]);
    expect(groups[2]!.rows).toEqual([]);
  });

  it("offers Move to pool… on a session's menu when the app can", () => {
    const spec = actionSpecs("session").find((entry) => entry.id === "pool")!;
    expect(spec.label).toBe("Move to pool…");
    expect(spec.applies(first, {})).toBe(false);
    expect(spec.applies(first, { moveSessionToPool: () => {} })).toBe(true);
  });
});

describe("the pool sheets", () => {
  it("makes a pool with the session in it, then opens it", async () => {
    const pools = await poolsList();
    const { getByTestId } = render(<PoolSheet target={{ mode: "create", sessionId: first.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    expect(getByTestId("pool-create-session").textContent).toContain("goes in it");
    fireEvent.change(getByTestId("pool-create-name"), { target: { value: "  Checkout redesign " } });
    fireEvent.click(getByTestId("pool-create-save"));
    await waitFor(() => expect(calls()).toEqual(["create:Checkout redesign", `move:${first.id}:pool-preview-1`]));
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

  it("moving a session with no pool of yours yet makes one", async () => {
    const pools = await loaded();
    const { getByTestId, queryByTestId } = render(<PoolSheet target={{ mode: "move", sessionId: first.id }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    expect(queryByTestId("pool-move-list")).toBeNull();
    expect(getByTestId("pool-create-session").textContent).toContain("goes in it");
  });

  it("All sessions keeps its name and can't be deleted", async () => {
    const pools = await poolsList();
    const { queryByTestId, getByText } = render(<PoolSheet target={{ mode: "pool", poolId: DEFAULT_POOL_ID }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    expect(queryByTestId("pool-name")).toBeNull();
    expect(queryByTestId("pool-delete")).toBeNull();
    expect(getByText("All sessions is always here.")).toBeTruthy();
  });

  it("says what deleting does before it does it", async () => {
    seedCheckout();
    const pools = await poolsList();
    const { getByTestId } = render(<PoolSheet target={{ mode: "pool", poolId: "pool-checkout" }} pools={pools} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />);
    fireEvent.click(getByTestId("pool-delete"));
    expect(getByTestId("pool-delete-confirm").textContent).toContain("Its 2 sessions go back to All sessions; nothing else changes.");
    expect(calls()).toEqual([]);
    fireEvent.click(getByTestId("pool-delete-yes"));
    await waitFor(() => expect(calls()).toEqual(["delete:pool-checkout"]));
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
    await waitFor(() => expect(calls()[0]).toMatch(/^access:pool-default:custom:/));
    rerender(<PoolAccessPanel pool={(await poolsList()).pools[0]!} />);
    expect(getByTestId("pool-custom")).toBeTruthy();
    expect(queryByText("Put a message straight into another session")).toBeNull();
  });

  it("sets the hop limit and the stop-all, and takes a rule back", async () => {
    seams().__AIJ_POOLS__ = {
      pools: [{ id: "pool-team", name: "Team", rules: [{ id: 7, pool_id: "pool-team", action: "assign_task", created_at: Date.now(), made_by: "user" }] }],
    };
    const pool = (await loaded()).pools[1]!;
    const { getByTestId } = render(<PoolAccessPanel pool={pool} />);
    fireEvent.change(getByTestId("pool-hop-limit"), { target: { value: "3" } });
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
    expect(rows.map((row) => row.querySelector("summary")?.textContent)).toEqual([
      "›All sessionsWork together",
      "›Checkout redesignWork together",
      "›DocsWork together",
    ]);
  });
});

describe("context cards", () => {
  const card = (over: Partial<ContextCard>): ContextCard => ({
    session_id: first.id,
    name: "Split checkout",
    place: "shop · feature/steps",
    tool: "Claude Code",
    account: "Work Max",
    state: "running",
    role: "lead",
    task: { value: null, unavailable: "Unavailable — AI Juggler doesn't keep tasks yet" },
    said: { value: "Split checkout into steps", unavailable: null },
    last_activity_ms: Date.now() - 120_000,
    files: ["src/cart.ts", "src/pay.ts"],
    files_unavailable: null,
    ...over,
  });

  it("shows a row like Home's, and the rest behind its disclosure", () => {
    const { getByTestId, getByLabelText } = render(<PoolCardRows cards={[card({})]} sessions={MOCK_SESSIONS} now={Date.now()} />);
    expect(getByTestId("pool-card").textContent).toContain("Split checkout");
    expect(getByTestId("pool-card").textContent).toContain("Claude Code · Work Max");
    const more = getByLabelText("Show what Split checkout is doing");
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(getByTestId("pool-card-said").textContent).toBe("“Split checkout into steps”said by the agent");
    expect(getByTestId("pool-card-files").textContent).toBe("src/cart.tssrc/pay.ts");
    expect(getByTestId("pool-card-detail").textContent).toContain("Unavailable — AI Juggler doesn't keep tasks yet");
  });

  it("says Unavailable and why, never an empty guess", () => {
    const unknown = card({
      said: { value: null, unavailable: "Unavailable — read only while it runs in AI Juggler" },
      files: [],
      files_unavailable: "Unavailable — read only while it runs in AI Juggler",
    });
    const { getByLabelText, getByTestId } = render(<PoolCardRows cards={[unknown]} sessions={MOCK_SESSIONS} now={Date.now()} />);
    fireEvent.click(getByLabelText(/^Show what/));
    expect(getByTestId("pool-card-said").textContent).toBe("Unavailable — read only while it runs in AI Juggler");
    expect(getByTestId("pool-card-files").textContent).toBe("Unavailable — read only while it runs in AI Juggler");
  });

  it("says so when a pool is empty", () => {
    const { getByTestId } = render(<PoolCardRows cards={[]} sessions={[]} now={Date.now()} />);
    expect(getByTestId("pool-cards-empty")).toBeTruthy();
  });
});

describe("Home's pool filter", () => {
  function Probe({ all }: { all: InstanceInfo[] }) {
    const { sessions, picker } = useHomePool(all);
    return (
      <div>
        {picker}
        <output data-testid="shown">{sessions.map((row) => row.id).join(",")}</output>
      </div>
    );
  }

  it("shows one pool's sessions, remembered", async () => {
    seedCheckout();
    const { findByTestId, getByTestId } = render(<Probe all={MOCK_SESSIONS} />);
    await loaded();
    fireEvent.change(await findByTestId("home-pool-filter"), { target: { value: "pool-checkout" } });
    expect(getByTestId("shown").textContent).toBe(`${first.id},${second.id}`);
    expect(localStorage.getItem("aij.home.pool")).toBe("pool-checkout");
  });

  it("isn't there without a pool of yours", async () => {
    const { queryByTestId } = render(<Probe all={MOCK_SESSIONS} />);
    await loaded();
    act(() => setPools({ pools: [], members: [] }));
    expect(queryByTestId("home-pool-filter")).toBeNull();
  });
});

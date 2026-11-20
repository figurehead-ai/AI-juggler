/**
 * AJ-E2807: the Pools screen's order, a card's menu (Open its canvas,
 * Rename…, Delete… saying what happens to its members) and its empty text.
 */
import { asInput } from "../../test/fixtures";
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { PoolInfo } from "../../types/generated/PoolInfo";
import { resetPreviewPools, scopeOf } from "../../lib/ipc/pools";
import { forgetPaged } from "../../lib/paged";
import { refreshPools, resetPools, setPools } from "../../lib/pools";
import { cardOrder, PoolsScreen } from "./PoolsScreen";
import { visitOrder } from "./visit-order";

const first: InstanceInfo = MOCK_SESSIONS[0]!;
const second: InstanceInfo = MOCK_SESSIONS[1]!;
const seed = (pools: unknown) => Object.assign(window, { __AIJ_POOLS__: pools });

const pool = (id: string, kind: string, members: number): PoolInfo => ({
  id,
  name: id,
  kind,
  project_id: kind === "default" ? id : null,
  access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false },
  rules: [],
  spans_macs: false,
  scopes: 1,
  members,
  ...scopeOf(kind, kind === "default" ? id : null),
});

beforeEach(() => {
  resetPreviewPools();
  resetPools();
  forgetPaged();
  Reflect.deleteProperty(window, "__AIJ_POOLS__");
});
afterEach(cleanup);

// X2 (the rc26 walk): a 50-member pool took 120–140 ms from the click to
// its canvas, its two reads made only once it opened. Pointing at its card
// reads them ahead.
describe("a pool's card reads its pool ahead", () => {
  it("on hover or focus: the pool's page then paints from what was read", async () => {
    seed({ pools: [{ id: "pool-team", name: "Team", rules: [] }], members: [{ session_id: first.id, pool_id: "pool-team", role: "lead" }] });
    await act(async () => {
      await refreshPools();
    });
    const { forgetCanvases, rememberedCanvas } = await import("../../pages/pool/prefetch");
    forgetCanvases();
    // The screen's own idle read-ahead (250 ms after it paints) is held
    // back, so only the hover or focus can read the pool: no wall-clock
    // window to beat (a 200 ms one failed on a loaded run).
    const realSetTimeout = window.setTimeout;
    const timers = vi.spyOn(window, "setTimeout").mockImplementation(((handler: TimerHandler, ms?: number, ...rest: unknown[]) =>
      ms === 250 ? 0 : realSetTimeout(handler, ms, ...rest)) as typeof window.setTimeout);
    onTestFinished(() => timers.mockRestore());
    const view = render(
      <ActionHostProvider host={{}}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const card = (await view.findAllByTestId("pool-card-tile")).find((tile) => tile.dataset.pool === "pool-team")!;
    expect(rememberedCanvas("pool-team")).toBeNull();
    fireEvent.pointerEnter(card);
    await waitFor(() => expect(rememberedCanvas("pool-team")?.pool_id).toBe("pool-team"));
    forgetCanvases();
    fireEvent.focus(card);
    await waitFor(() => expect(rememberedCanvas("pool-team")?.pool_id).toBe("pool-team"));
    expect(timers.mock.calls.some(([, ms]) => ms === 250)).toBe(true);
  });
});

describe("the Pools screen (AJ-E2807)", () => {
  it("puts the pools that ask you first, then the latest active (R2)", () => {
    const pools = [pool("mine", "user", 2), pool("quiet", "default", 3), pool("asking", "default", 1), pool("busy", "user", 1)];
    const order = cardOrder(pools, new Map(), new Map([["asking", 1]]), new Map([["busy", 900], ["mine", 100], ["quiet", 500]]));
    expect(order.map((row) => row.id)).toEqual(["asking", "busy", "quiet", "mine"]);
  });

  // F-rc3g-14: the cards swapped places as their sessions worked, so a
  // click aimed at A opened B. The order holds from your first move; a
  // pool made meanwhile goes at the end.
  it("keeps its cards in place while you're on it, a new pool at the end", async () => {
    seed({
      pools: [
        { id: "pool-a", name: "A", rules: [] },
        { id: "pool-b", name: "B", rules: [] },
      ],
      members: [
        { session_id: first.id, pool_id: "pool-a", role: "member" },
        { session_id: second.id, pool_id: "pool-b", role: "member" },
      ],
    });
    await act(async () => {
      await refreshPools();
    });
    const at = (a: number, b: number) => [
      { ...first, status: "running" as const, last_activity_ms: a },
      { ...second, status: "running" as const, last_activity_ms: b },
    ];
    const screen = (sessions: InstanceInfo[]) => <PoolsScreen sessions={sessions} projects={MOCK_PROJECTS} accounts={[]} />;
    const view = render(screen(at(1_000, 2_000)));
    const mine = () =>
      view
        .queryAllByTestId("pool-card-tile")
        .map((tile) => tile.dataset.pool ?? "")
        .filter((id) => ["pool-a", "pool-b", "pool-c"].includes(id));
    await waitFor(() => expect(mine()).toEqual(["pool-b", "pool-a"]));
    // The person moves toward a card; A's session then works.
    fireEvent.pointerMove(window);
    view.rerender(screen(at(3_000, 2_000)));
    expect(mine()).toEqual(["pool-b", "pool-a"]);
    resetPreviewPools();
    seed({
      pools: [
        { id: "pool-c", name: "C", rules: [] },
        { id: "pool-a", name: "A", rules: [] },
        { id: "pool-b", name: "B", rules: [] },
      ],
      members: [
        { session_id: first.id, pool_id: "pool-a", role: "member" },
        { session_id: second.id, pool_id: "pool-b", role: "member" },
      ],
    });
    await act(async () => {
      await refreshPools();
    });
    await waitFor(() => expect(mine()).toEqual(["pool-b", "pool-a", "pool-c"]));
    expect(visitOrder(["b", "a", "gone"], ["c", "a", "b"])).toEqual(["b", "a", "c"]);
    expect(visitOrder(null, ["c", "a"]), "before the order holds").toEqual(["c", "a"]);
  });

  // AJ-E3014 item 1: made from here, a pool opens its page; back on the
  // screen it took slot 3, then moved again as its sessions joined.
  it("a pool made since the last visit comes after the others on the next, however its sessions move", async () => {
    const third: InstanceInfo = MOCK_SESSIONS[2]!;
    const members = [
      { session_id: first.id, pool_id: "pool-a", role: "member" },
      { session_id: second.id, pool_id: "pool-b", role: "member" },
    ];
    seed({ pools: [{ id: "pool-a", name: "A", rules: [] }, { id: "pool-b", name: "B", rules: [] }], members });
    await act(async () => {
      await refreshPools();
    });
    const busy = (a: number, b: number, c: number) => [
      { ...first, status: "running" as const, last_activity_ms: a },
      { ...second, status: "running" as const, last_activity_ms: b },
      { ...third, status: "running" as const, last_activity_ms: c },
    ];
    const screen = (sessions: InstanceInfo[]) => <PoolsScreen sessions={sessions} projects={MOCK_PROJECTS} accounts={[]} />;
    const mine = (view: ReturnType<typeof render>) =>
      view
        .queryAllByTestId("pool-card-tile")
        .map((tile) => tile.dataset.pool ?? "")
        .filter((id) => ["pool-a", "pool-b", "pool-c"].includes(id));
    const one = render(screen(busy(1_000, 2_000, 0)));
    await waitFor(() => expect(mine(one)).toEqual(["pool-b", "pool-a"]));
    one.unmount();
    // Made meanwhile (on its own page), its session the latest active.
    resetPreviewPools();
    seed({ pools: [{ id: "pool-c", name: "C", rules: [] }, { id: "pool-a", name: "A", rules: [] }, { id: "pool-b", name: "B", rules: [] }], members: [...members, { session_id: third.id, pool_id: "pool-c", role: "member" }] });
    await act(async () => {
      await refreshPools();
    });
    const two = render(screen(busy(1_000, 2_000, 5_000)));
    await waitFor(() => expect(mine(two)).toEqual(["pool-b", "pool-a", "pool-c"]));
    two.rerender(screen(busy(1_000, 2_000, 9_000)));
    expect(mine(two)).toEqual(["pool-b", "pool-a", "pool-c"]);
  });

  it("gives a card its menu: open, rename, and delete saying what happens to its members (R4)", async () => {
    seed({
      pools: [{ id: "pool-team", name: "Team", rules: [] }],
      members: [
        { session_id: first.id, pool_id: "pool-team", role: "lead" },
        { session_id: second.id, pool_id: "pool-team", role: "member" },
      ],
    });
    await act(async () => {
      await refreshPools();
    });
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => void opened.push(id) }}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const names = () => view.queryAllByTestId("pool-card-tile").map((tile) => tile.textContent ?? "");
    const card = (await view.findAllByTestId("pool-card-tile")).find((tile) => tile.dataset.pool === "pool-team")!;
    const box = card.parentElement!;
    // A few cards keep a card's width (the E28 walk: one pool was a
    // full-width banner).
    expect(view.getByTestId("pools-cards").style.gridTemplateColumns).toContain("auto-fill");
    fireEvent.contextMenu(box, { clientX: 30, clientY: 30, button: 2 });
    const menu = view.getByRole("menu");
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Open its diagram", "Rename…", "Delete…"]);
    for (const item of within(menu).getAllByRole("menuitem")) expect(item.querySelector("svg"), item.textContent ?? "").toBeTruthy();
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Open its diagram" }));
    expect(opened).toEqual(["pool-team"]);
    // Rename… from its ⋯ button, always in sight (SK-1: it was there only
    // on hover, so no picture showed a way to delete a pool).
    expect(within(box).getByTestId("pool-card-menu").className).not.toContain("opacity-0");
    fireEvent.click(within(box).getByTestId("pool-card-menu"));
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: "Rename…" }));
    // The one rename sheet: its name selected on open, so typing replaces
    // it (the 0.4.6 RC walk A8: "Work" + "Acme" saved "WorkAcme").
    const field = asInput(view.getByTestId("pool-rename-name"));
    expect(document.activeElement).toBe(field);
    expect([field.selectionStart, field.selectionEnd]).toEqual([0, "Team".length]);
    fireEvent.change(field, { target: { value: "Checkout team" } });
    fireEvent.click(view.getByTestId("pool-rename-save"));
    await waitFor(() => expect(names().some((words) => words.includes("Checkout team"))).toBe(true));
    // Delete… asks first, saying where its sessions go
    fireEvent.keyDown(box, { key: "F10", shiftKey: true });
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: "Delete…" }));
    expect(view.getByTestId("pool-delete-words").textContent).toContain("Its 2 sessions go back to their projects' own pools");
    fireEvent.click(view.getByTestId("pool-delete-confirm"));
    await waitFor(() => expect(names().some((words) => words.includes("Checkout team"))).toBe(false));
  });

  it("a project's own pool can't be renamed or deleted, and its menu says why", async () => {
    await act(async () => {
      await refreshPools();
    });
    const view = render(
      <ActionHostProvider host={{ openPool: () => undefined }}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const tiles = await view.findAllByTestId("pool-card-tile");
    const project = tiles.find((tile) => within(tile).getByTestId("pool-card-scope").textContent === "Project pool")!;
    fireEvent.contextMenu(project.parentElement!, { clientX: 30, clientY: 30, button: 2 });
    const rename = within(view.getByRole("menu")).getByRole("menuitem", { name: /Rename…/ });
    expect(rename.hasAttribute("disabled") || rename.getAttribute("aria-disabled") === "true").toBe(true);
    expect(view.getByRole("menu").textContent).toContain("A project's own pool goes with its project");
  });

  it("F-rc3g-7: a project's own pool's card says its one name, the one its page, a session's Pools tab and the agents' tools use", async () => {
    await act(async () => {
      await refreshPools();
    });
    const view = render(
      <ActionHostProvider host={{ openPool: () => undefined }}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const tiles = await view.findAllByTestId("pool-card-tile");
    const own = tiles.filter((tile) => within(tile).getByTestId("pool-card-scope").textContent === "Project pool");
    expect(own.length).toBeGreaterThan(0);
    for (const tile of own) expect(within(tile).getByTestId("pool-card-name").textContent).toMatch(/^.+ · all sessions$/);
    // Never the project's name alone, which no agent's tool took (the P4 walk: "web").
    expect(own.map((tile) => within(tile).getByTestId("pool-card-name").textContent)).not.toContain(MOCK_PROJECTS[0]!.name);
  });

  it("with no project yet, says pools come with it (R5)", async () => {
    act(() => setPools({ pools: [], placements: [] }));
    const view = render(<PoolsScreen sessions={[]} projects={[]} accounts={[]} />);
    expect((await view.findByTestId("pools-empty")).textContent).toBe("Pools appear with your first project.");
  });

  it("shows 24 cards at a time, the rest a click away, never all at once (R2)", async () => {
    seed({ pools: Array.from({ length: 30 }, (_, n) => ({ id: `pool-${String(n).padStart(2, "0")}`, name: `Pool ${n}`, rules: [] })), members: [] });
    await act(async () => {
      await refreshPools();
    });
    const view = render(<PoolsScreen sessions={[]} projects={MOCK_PROJECTS} accounts={[]} />);
    await waitFor(() => expect(view.getAllByTestId("pool-card-tile")).toHaveLength(24));
    const more = view.getByTestId("pools-more");
    // 30 of yours and the preview's two project pools: 32.
    expect(more.textContent).toBe("Show 8 more of 8");
    fireEvent.click(more);
    await waitFor(() => expect(view.getAllByTestId("pool-card-tile")).toHaveLength(32));
    expect(view.queryByTestId("pools-more")).toBeNull();
  });
});

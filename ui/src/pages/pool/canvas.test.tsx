/**
 * 0.3 vitest (redesign §5) — the pool canvas and the Pools screen.
 *
 * - Geometry: the automatic layout is deterministic, the lead on top, the
 *   user's places kept and never landed on; fitting, zooming, the arrow
 *   keys' neighbour, an edge's curve.
 * - The Pools screen's words: a new pool's name, the cards' order (empty
 *   project pools wait), a card's line.
 * - The page: members as nodes, an agent's ask answered on its node, a
 *   member taken out with Undo, rename in the header (a project's pool
 *   keeps its name), the pool's settings in the inspector (level, hop
 *   limit, stop-all, a rule taken back, delete says what it does first),
 *   deep links into Messages, Tasks and Agent access.
 */
import { act, cleanup, fireEvent, render, renderHook, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { scopeOf } from "../../lib/ipc/pools";
import { onPoolsChanged, refreshPools, defaultPoolIdOf, openPoolSheet, usePoolSheet } from "../../lib/pools";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { pick } from "../../test/pick";
import { rememberTab, rememberedTabId } from "../registry";
import { PoolPage } from "../PoolPage";
import { first, openPage, resetCanvasTests, second, seams, seedTeam, third } from "./canvas.test-kit";
import { cardOrder, cardWords, newPoolName, PoolsScreen } from "../../screens/pools/PoolsScreen";
import { deliveryShort, deliveryWords } from "./InspectPool";
import { CanvasEdges, pendingWords } from "./CanvasEdges";
import { CompactNode } from "./CompactNode";
import type { ContextCard } from "../../types/generated/ContextCard";
import { curvePoint, edgeCurve, fitView, nodeMiddle, layout, neighbour, NODE_H, NODE_W, nodeForm, zoomAt } from "./layout";


beforeEach(resetCanvasTests);
afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("the canvas's geometry", () => {
  const nodes = [
    { id: "b", role: "member" },
    { id: "lead", role: "lead" },
    { id: "a", role: "member" },
    { id: "o", role: "observer" },
  ];
  it("lays out the same members the same way, the lead on top", () => {
    const one = layout(nodes, new Map());
    const two = layout([...nodes].reverse(), new Map());
    expect([...one.entries()].sort()).toEqual([...two.entries()].sort());
    const lead = one.get("lead")!;
    for (const id of ["a", "b", "o"]) expect(one.get(id)!.y).toBeGreaterThan(lead.y + NODE_H);
    // Members before observers, then by id: a, b, o left to right.
    expect(one.get("a")!.x).toBeLessThan(one.get("b")!.x);
    expect(one.get("b")!.x).toBeLessThan(one.get("o")!.x);
  });

  it("keeps the user's places, and never lands a node on one", () => {
    const auto = layout(nodes, new Map());
    const taken = new Map([["b", auto.get("a")!]]);
    const out = layout(nodes, taken);
    expect(out.get("b")).toEqual(auto.get("a"));
    const a = out.get("a")!;
    const b = out.get("b")!;
    expect(Math.abs(a.x - b.x) >= NODE_W || Math.abs(a.y - b.y) >= NODE_H).toBe(true);
  });

  it("fits every node in the window, never past 100 %, and zooms around a point", () => {
    const points = layout(nodes, new Map());
    const view = fitView(points.values(), 1000, 600);
    expect(view.scale).toBeLessThanOrEqual(1);
    const zoomed = zoomAt(view, 2, { x: 300, y: 200 });
    // The point under the pointer stays where it was.
    expect((300 - zoomed.x) / zoomed.scale).toBeCloseTo((300 - view.x) / view.scale);
  });

  it("keeps every node out from under the group column at the right, and lets a folding palette give way first (the L3 canvas walk)", () => {
    const points = layout(nodes, new Map());
    const rightEdge = (view: ReturnType<typeof fitView>) => Math.max(...[...points.values()].map((at) => view.x + (at.x + NODE_W) * view.scale));
    const leftEdge = (view: ReturnType<typeof fitView>) => Math.min(...[...points.values()].map((at) => view.x + at.x * view.scale));
    // Room for both: nothing under the palette (260) or the column (240).
    const wide = fitView(points.values(), 1400, 700, 48, 260, 0, 0, 240);
    expect(rightEdge(wide)).toBeLessThanOrEqual(1400 - 240);
    expect(leftEdge(wide)).toBeGreaterThanOrEqual(260);
    // Too narrow for both: the column stays clear; the open palette folds.
    const narrow = fitView(points.values(), 640, 600, 48, 260, 0, 0, 240);
    expect(rightEdge(narrow)).toBeLessThanOrEqual(640 - 240 + 1);
  });

  it("starts a grid too wide to fit at its first column, beside the palette (rc4 walk, 50 members)", () => {
    const wideGrid = new Map(Array.from({ length: 50 }, (_, i) => [`m${i}`, { x: (i % 8) * (NODE_W + 40), y: Math.floor(i / 8) * (NODE_H + 60) }] as const));
    // No column over the right: the readable floor, the first node just past the palette.
    const opened = fitView(wideGrid.values(), 1280, 800, 48, 260, 0, 0, 0);
    expect(opened.scale).toBeCloseTo(0.45);
    expect(opened.x).toBeGreaterThanOrEqual(260);
    expect(opened.x).toBeLessThanOrEqual(260 + 48 + 1);
  });

  it("keeps every member clear of the right-hand column, below the readable floor if it must (rc5 walk)", () => {
    const grid = (columns: number) => new Map(Array.from({ length: 49 }, (_, i) => [`m${i}`, { x: (i % columns) * (NODE_W + 40), y: Math.floor(i / columns) * (NODE_H + 60) }] as const));
    const right = (view: ReturnType<typeof fitView>, points: Map<string, { x: number; y: number }>) => Math.max(...[...points.values()].map((at) => view.x + (at.x + NODE_W) * view.scale));
    const seven = grid(7);
    const fitted = fitView(seven.values(), 1280, 800, 48, 260, 0, 0, 240);
    expect(fitted.scale).toBeLessThan(0.45);
    expect(right(fitted, seven)).toBeLessThanOrEqual(1280 - 240 + 1);
    expect(fitted.x).toBeGreaterThanOrEqual(260);
    // Too wide even at the least scale: the first column beside the palette, the rest a pan away.
    const many = grid(20);
    const panned = fitView(many.values(), 1280, 800, 48, 260, 0, 0, 240);
    expect(panned.scale).toBeCloseTo(0.3);
    expect(panned.x).toBeGreaterThanOrEqual(260);
  });

  it("moves to the nearest node the arrow points at", () => {
    const points = new Map([
      ["mid", { x: 0, y: 0 }],
      ["right", { x: 300, y: 10 }],
      ["below", { x: 20, y: 250 }],
      ["far", { x: 900, y: 0 }],
    ]);
    expect(neighbour(points, "mid", "ArrowRight")).toBe("right");
    expect(neighbour(points, "mid", "ArrowDown")).toBe("below");
    expect(neighbour(points, "mid", "ArrowUp")).toBeNull();
  });

  it("bends the two directions of a pair apart", () => {
    const a = { x: 0, y: 0 };
    const b = { x: 600, y: 0 };
    const there = edgeCurve(a, b, true);
    const back = edgeCurve(b, a, true);
    const line = NODE_H / 2;
    expect(Math.sign(there.mid.y - line)).toBe(-Math.sign(back.mid.y - line));
    expect(there.d.startsWith("M ")).toBe(true);
  });
});

describe("an edge never runs behind a node (the 0.3 review: Fix → Review behind Checkout copy pass)", () => {
  const a = { x: 0, y: 0 };
  const between = { x: 300, y: 0 };
  const b = { x: 600, y: 0 };
  const inside = (p: { x: number; y: number }, box: { x: number; y: number }) => p.x > box.x && p.x < box.x + NODE_W && p.y > box.y && p.y < box.y + NODE_H;
  const runsThrough = (curve: ReturnType<typeof edgeCurve>, box: { x: number; y: number }) =>
    Array.from({ length: 49 }, (_, n) => curvePoint(curve.start, curve.control, curve.end, n / 48)).some((p) => inside(p, box));

  it("bends round a node in its way, and its middle (its label) is off that node", () => {
    expect(runsThrough(edgeCurve(a, b, false), between)).toBe(true);
    const routed = edgeCurve(a, b, false, [between]);
    expect(runsThrough(routed, between)).toBe(false);
    expect(inside(routed.mid, between)).toBe(false);
  });

  it("keeps its gentle arc when nothing is in the way", () => {
    expect(edgeCurve(a, b, false, [{ x: 300, y: 600 }]).d).toBe(edgeCurve(a, b, false).d);
  });

  it("at fit, a badge sits on its own line, not where another edge's line runs", () => {
    // The review's row: A, M and B side by side, C under M. A → B goes
    // round M on the side M → C leaves free, so its badge, on its own
    // line, is off M → C's.
    const row = { a: { x: 0, y: 0 }, m: { x: 268, y: 0 }, b: { x: 536, y: 0 }, c: { x: 268, y: 212 } };
    const points = new Map(Object.entries(row));
    const at = Date.now();
    const waiting = [{ message_id: "m-1", thread_id: "thr", subject: "Totals", recipient: "session:b", kind: "waiting" as const, ask_id: null, at_ms: at }];
    const edges = [
      { from_session_id: "a", to_session_id: "b", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr", pending: waiting },
      { from_session_id: "m", to_session_id: "c", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr2", pending: [] },
    ];
    const view = render(<CanvasEdges edges={edges} points={points} names={new Map()} selected={null} pulses={new Map()} reducedMotion scale={0.6} onSelect={() => {}} />);
    const label = view.getAllByTestId("edge-label").find((one) => one.dataset.edge === "a>b")!;
    const [x, y] = label.style.transform.match(/translate\(([-\d.]+)px, ([-\d.]+)px\)/)!.slice(1).map(Number) as [number, number];
    const own = edgeCurve(row.a, row.b, false, [row.m, row.c], [[nodeMiddle(row.m), nodeMiddle(row.c)]]);
    // It goes round M on the side M → C doesn't use (above).
    expect(own.mid.y).toBeLessThan(row.m.y);
    expect(runsThrough(own, row.m)).toBe(false);
    const onOwn = Array.from({ length: 101 }, (_, n) => curvePoint(own.start, own.control, own.end, n / 100)).some((p) => Math.hypot(p.x - x, p.y - y) < 4);
    expect(onOwn).toBe(true);
    // Clear of M → C's line: no point of it under the badge (110 × 26 at
    // screen size, so 183 × 43 canvas units at 60 %).
    const other = edgeCurve(row.m, row.c, false, [row.a, row.b]);
    const under = Array.from({ length: 49 }, (_, n) => curvePoint(other.start, other.control, other.end, n / 48)).filter((p) => Math.abs(p.x - x) < 110 / 0.6 / 2 && Math.abs(p.y - y) < 26 / 0.6 / 2);
    expect(under).toEqual([]);
  });
});

describe("the Pools screen's words", () => {
  it("names a new pool so it's never taken", () => {
    expect(newPoolName(["Docs"])).toBe("New pool");
    expect(newPoolName(["new pool", "New pool 2"])).toBe("New pool 3");
  });
  it("puts yours first, then projects' pools with open sessions; an empty project pool waits", () => {
    const pool = (id: string, kind: string, members: number) => ({ id, name: id, kind, project_id: kind === "default" ? id : null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members, ...scopeOf(kind, kind === "default" ? id : null) });
    const order = cardOrder([pool("quiet", "default", 3), pool("busy", "default", 1), pool("empty", "default", 0), pool("mine", "user", 0)], new Map([["busy", 1]]));
    expect(order.map((row) => row.id)).toEqual(["mine", "busy", "quiet"]);
  });
  it("says a card's line in words", () => {
    expect(cardWords(3, 1, 2)).toBe("3 members · 1 needs you · 2 tasks moving");
    expect(cardWords(1, 0, 0)).toBe("1 member · quiet");
    expect(cardWords(0, 0, 0)).toBe("0 members");
  });
});

/** The pool sheet's target now (read through its hook). */
function sheetTarget(): unknown {
  const probe = renderHook(() => usePoolSheet());
  const seen = probe.result.current;
  probe.unmount();
  return seen;
}

describe("one count of what needs you", () => {
  it("the pool's header says what its Pools card says (the 0.3 review: \"1 asks you\" beside \"4 need you\")", async () => {
    seedTeam();
    const page = await openPage();
    await waitFor(() => expect(page.getByTestId("pool-page-count").textContent).toMatch(/\d+ needs? you/));
    const header = page.getByTestId("pool-page-count").textContent!.match(/(\d+ needs? you)/)![1];
    page.unmount();
    const screen = render(
      <ActionHostProvider host={{}}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    await waitFor(() => expect(screen.getAllByTestId("pool-card-words").some((line) => line.textContent!.includes(header!))).toBe(true));
  });
});

describe("a pool's page: its canvas", () => {
  it("a member's App opens its page from the node (design review #14)", async () => {
    seedTeam();
    const opened: string[] = [];
    await act(async () => {
      await refreshPools();
    });
    const view = render(
      <ConversationsProvider want={null}>
        <ActionHostProvider host={{ openAccountById: (id) => opened.push(id) }}>
          <PoolPage poolId="pool-team" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
    const node = view.container.querySelector(`[data-node="${first.id}"]`) as HTMLElement;
    fireEvent.click(within(node).getByTestId("node-app"));
    expect(opened).toEqual([first.account_id]);
  });

  it("says where it sits, Pools › its name, and Pools goes back (AJ-E2803 R3)", async () => {
    seedTeam();
    const view = await openPage();
    const crumbs = view.getByTestId("page-crumbs");
    expect(crumbs.textContent).toMatch(/^Pools›/);
    const screens: string[] = [];
    const on = (event: Event) => {
      if (event instanceof CustomEvent) screens.push(JSON.stringify(event.detail));
    };
    window.addEventListener("aij:open-screen", on);
    fireEvent.click(within(crumbs).getByTestId("crumb-pools"));
    window.removeEventListener("aij:open-screen", on);
    expect(screens).toEqual([JSON.stringify({ id: "pools" })]);
  });

  it("moves a node 8 px with Option and an arrow, 32 with Option Shift (AJ-E2808 R7)", async () => {
    seedTeam();
    const view = await openPage();
    const name = view.container.querySelector<HTMLButtonElement>(`[data-node-name="${first.id}"]`)!;
    const places = (): string[] => {
      const calls: unknown = seams().__AIJ_POOL_CANVAS_CALLS__;
      return Array.isArray(calls) ? calls.map(String).filter((call) => call.startsWith("place:")) : [];
    };
    fireEvent.keyDown(name, { key: "ArrowRight", altKey: true });
    await waitFor(() => expect(places().length).toBeGreaterThan(0));
    const at = (call: string) => Number(new RegExp(`${first.id}@(-?\\d+),`).exec(call)?.[1]);
    const before = places().length;
    const x0 = at(places().at(-1)!);
    fireEvent.keyDown(name, { key: "ArrowRight", altKey: true });
    await waitFor(() => expect(places().length).toBe(before + 1));
    expect(at(places().at(-1)!) - x0).toBe(8);
    fireEvent.keyDown(name, { key: "ArrowRight", altKey: true, shiftKey: true });
    await waitFor(() => expect(places().length).toBe(before + 2));
    expect(at(places().at(-1)!) - x0).toBe(40);
  });

  it("a node's menu moves it to another pool through the pool sheet, never starting it (AJ-E2808 R3)", async () => {
    seedTeam();
    const view = await openPage();
    const name = view.container.querySelector<HTMLButtonElement>(`[data-node-name="${first.id}"]`)!;
    fireEvent.keyDown(name, { key: "F10", shiftKey: true });
    const move = await view.findByTestId("node-menu-move");
    expect(move.textContent).toContain("Move to pool…");
    fireEvent.click(move);
    expect(sheetTarget()).toEqual({ mode: "move", sessionId: first.id });
    openPoolSheet(null);
  });

  it("draws its members as nodes, the lead marked, and answers an ask on its node", async () => {
    seedTeam();
    const view = await openPage();
    expect(view.getAllByTestId("canvas-node")).toHaveLength(2);
    expect(within(view.container.querySelector(`[data-node="${first.id}"]`)!).getByTestId("node-lead")).toBeTruthy();
    const asker = view.container.querySelector(`[data-node="${second.id}"]`) as HTMLElement;
    await waitFor(() => expect(within(asker).getByTestId("node-ask").textContent).toContain("B wants to message A"));
    fireEvent.click(within(asker).getByTestId("node-ask-once"));
    await waitFor(() => expect(seams().__AIJ_MESSAGES_CALLS__).toContain("answer:ask-1:once"));
  });

  it("takes a member out with Delete, and Undo puts it back", async () => {
    seedTeam();
    const view = await openPage();
    const name = view.container.querySelector(`[data-node-name="${second.id}"]`) as HTMLElement;
    fireEvent.keyDown(name, { key: "Delete" });
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`move:${second.id}:${defaultPoolIdOf(second.project_id)}`));
    fireEvent.click(await view.findByTestId("canvas-removed"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`move:${second.id}:pool-team`));
  });

  it("renames your pool in its header; a project's own pool keeps its name", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.getByTestId("pool-name"));
    const input = view.getByTestId("pool-name-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "Checkout team" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("rename:pool-team:Checkout team"));
    cleanup();
    const project = await openPage(defaultPoolIdOf(third.project_id));
    expect(project.queryByTestId("pool-name")).toBeNull();
    fireEvent.click(project.getByTestId("inspect-open-settings"));
    expect(project.queryByTestId("pool-delete")).toBeNull();
  });

  it("changes what agents may do in the inspector, and says what deleting does first", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.getByTestId("inspect-open-settings"));
    pick(view.getByTestId("pool-level"), "Full");
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("access:pool-team:full:"));
    pick(view.getByTestId("pool-hop-limit"), "3");
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("delivery:pool-team:3:false"));
    fireEvent.click(view.getByTestId("pool-stop-all"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("delivery:pool-team:3:true"));
    fireEvent.click(view.getByTestId("pool-rule-revoke"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("revoke:7"));
    fireEvent.click(view.getByTestId("pool-delete"));
    expect(view.getByRole("group", { name: "Delete this pool?" }).textContent).toContain("Its sessions go back to their projects' own pools");
    expect(seams().__AIJ_POOL_CALLS__).not.toContain("delete:pool-team");
  });

  it("deleting a pool updates the pools everywhere, so it leaves the list (the P11 walk)", async () => {
    seedTeam();
    const view = await openPage();
    const changed = vi.fn();
    const stop = onPoolsChanged(changed);
    fireEvent.click(view.getByTestId("inspect-open-settings"));
    fireEvent.click(view.getByTestId("pool-delete"));
    fireEvent.click(view.getByTestId("pool-delete-confirm"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain("delete:pool-team"));
    await waitFor(() => expect(changed).toHaveBeenCalled());
    stop();
  });

  it("lands where a link meant: Messages, Tasks, Agent access", async () => {
    seedTeam();
    rememberTab("pool", "messages");
    let view = await openPage();
    expect(view.getByTestId("pool-panel-messages-open")).toBeTruthy();
    // Open, Messages takes the page's width: the inspector steps aside.
    expect(view.getByTestId("pool-inspector-slot").className).toBe("hidden");
    fireEvent.click(view.getByTestId("pool-panel-close"));
    expect(view.getByTestId("pool-inspector-slot").className).toBe("contents");
    cleanup();
    rememberTab("pool", "access");
    view = await openPage();
    expect(view.getByTestId("inspect-pool")).toBeTruthy();
    expect(view.queryByTestId("pool-panel-messages-open")).toBeNull();
  });

  it("a landing is used once: a panel opened here doesn't open the next pool on it (the P11 walk)", async () => {
    seedTeam();
    rememberTab("pool", "tasks");
    let view = await openPage();
    expect(view.getByTestId("pool-panel-tasks-open")).toBeTruthy();
    expect(rememberedTabId("pool")).toBe("members");
    cleanup();
    view = await openPage();
    expect(view.queryByTestId("pool-panel-tasks-open")).toBeNull();
  });

  it("says how a message gets into a session", () => {
    const pool = { id: "p", name: "p", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
    expect(deliveryWords(pool)).toBe("When you click Deliver");
    expect(deliveryWords({ ...pool, access: { ...pool.access, level: "full" } })).toBe("By themselves, between turns");
    expect(deliveryWords({ ...pool, access: { ...pool.access, delivery_stopped: true } })).toBe("Stopped: nothing goes into a session");
    // The Activity panel's short form (design review #76).
    expect(deliveryShort(pool)).toBe("Wait for you");
    expect(deliveryShort({ ...pool, access: { ...pool.access, level: "full" } })).toBe("Go in by themselves");
    expect(deliveryShort({ ...pool, access: { ...pool.access, delivery_stopped: true } })).toBe("Stopped");
  });

  it("fits members above the zoom bar, not under it (design review #10)", () => {
    const points = [{ x: 0, y: 0 }, { x: 0, y: 600 }];
    const view = fitView(points, 800, 700, 48, 0, 0, 52);
    // The lowest card's foot (its point plus a card and what waits on it)
    // ends above the bar.
    const foot = view.y + (600 + NODE_H * 2) * view.scale;
    expect(foot).toBeLessThanOrEqual(700 - 52);
  });

  it("a paused chain says the pool's own number", () => {
    const edge = { from_session_id: "a", to_session_id: "b", messages: 6, tasks: 0, last_at_ms: 0, thread_id: "t", pending: [{ message_id: "m", thread_id: "t", subject: "s", recipient: "r", kind: "hop_limit", ask_id: "k", at_ms: 0 }] } as never;
    expect(pendingWords(edge, 5)?.label).toBe("Paused after 5 in a row");
    expect(pendingWords(edge)?.short).toBe("Paused");
  });
});

describe("semantic zoom (the 0.3 review: 9–10 px words at fit)", () => {
  it("draws full cards from 100 %, and below it as many screen-size lines as the box holds", () => {
    expect(nodeForm(1)).toEqual({ full: true });
    expect(nodeForm(1.4)).toEqual({ full: true });
    // A zoom's rounding a hair under 100 % is still 100 %.
    expect(nodeForm(0.999).full).toBe(true);
    expect(nodeForm(0.9)).toMatchObject({ full: false, lines: 3 });
    expect(nodeForm(0.6)).toMatchObject({ full: false, lines: 2 });
    expect(nodeForm(0.3)).toMatchObject({ full: false, lines: 1 });
    // Fit's floor leaves a node room for its name and its state.
    expect(nodeForm(0.45)).toMatchObject({ full: false, lines: 2 });
  });

  const zoomOut = (view: ReturnType<typeof render>, times: number) => {
    for (let i = 0; i < times; i += 1) fireEvent.click(view.getByRole("button", { name: "Zoom out" }));
  };

  it("below 100 % keeps the node's name, keys and description, and its badge opens the ask to answer in place", async () => {
    seedTeam();
    const view = await openPage();
    const asker = () => view.container.querySelector(`[data-node="${second.id}"]`) as HTMLElement;
    await waitFor(() => expect(within(asker()).getByTestId("node-ask")).toBeTruthy());
    expect(asker().dataset.form).toBe("full");
    zoomOut(view, 3);
    expect(asker().dataset.form).toBe("compact");
    // No inline answers on the small node; the same tab stop and words.
    expect(within(asker()).queryByTestId("node-ask")).toBeNull();
    const name = asker().querySelector(`[data-node-name="${second.id}"]`) as HTMLElement;
    expect(name.getAttribute("aria-keyshortcuts")).toBe("Enter Delete ArrowUp ArrowDown ArrowLeft ArrowRight");
    expect(name.getAttribute("aria-description")).toContain("waiting on you");
    const badge = within(asker()).getByTestId("node-badge");
    expect(badge.tagName).toBe("BUTTON");
    fireEvent.click(badge);
    const panel = within(asker()).getByTestId("node-popover");
    expect(panel.getAttribute("role")).toBe("dialog");
    expect(within(panel).getByTestId("node-ask").textContent).toContain("B wants to message A");
    fireEvent.click(within(panel).getByTestId("node-ask-once"));
    await waitFor(() => expect(seams().__AIJ_MESSAGES_CALLS__).toContain("answer:ask-1:once"));
  });

  it("Esc closes the badge's panel and gives the keyboard back to the badge", async () => {
    seedTeam();
    const view = await openPage();
    const asker = () => view.container.querySelector(`[data-node="${second.id}"]`) as HTMLElement;
    await waitFor(() => expect(within(asker()).getByTestId("node-ask")).toBeTruthy());
    zoomOut(view, 2);
    fireEvent.click(within(asker()).getByTestId("node-badge"));
    const panel = within(asker()).getByTestId("node-popover");
    fireEvent.keyDown(panel, { key: "Escape" });
    expect(within(asker()).queryByTestId("node-popover")).toBeNull();
    expect(document.activeElement).toBe(within(asker()).getByTestId("node-badge"));
  });

  it("below 100 % what waits on an edge is a badge that opens its answer", async () => {
    seedTeam();
    seams().__AIJ_POOL_CANVAS__ = {
      "pool-team": {
        edges: [
          {
            from_session_id: first.id,
            to_session_id: second.id,
            messages: 2,
            tasks: 0,
            last_at_ms: Date.now() - 1000,
            thread_id: "thr-1",
            pending: [{ message_id: "m-9", thread_id: "thr-1", subject: "Totals", recipient: `session:${second.id}`, kind: "waiting", ask_id: null, at_ms: Date.now() - 1000 }],
          },
        ],
      },
    };
    const view = await openPage();
    await waitFor(() => expect(view.getByTestId("edge-pending")).toBeTruthy());
    zoomOut(view, 2);
    expect(view.queryByTestId("edge-pending")).toBeNull();
    const badge = view.getByTestId("edge-badge");
    expect(badge.textContent).toBe("1 waiting");
    expect(badge.getAttribute("aria-label")).toMatch(/^1 waiting: 1 message waiting, /);
    fireEvent.click(badge);
    expect(badge.getAttribute("aria-expanded")).toBe("true");
    expect(within(view.getByTestId("edge-badge-panel")).getByTestId("edge-deliver").textContent).toBe("Deliver");
  });

  it("AJ-E3008 R4: a line paused at the hop limit offers Let it continue and Don't deliver", async () => {
    seedTeam();
    seams().__AIJ_POOL_CANVAS__ = {
      "pool-team": {
        edges: [
          {
            from_session_id: first.id,
            to_session_id: second.id,
            messages: 6,
            tasks: 0,
            last_at_ms: Date.now() - 1000,
            thread_id: "thr-1",
            pending: [{ message_id: "m-hop", thread_id: "thr-1", subject: "Totals", recipient: `session:${second.id}`, kind: "hop_limit", ask_id: null, at_ms: Date.now() - 1000 }],
          },
        ],
      },
    };
    const view = await openPage();
    const held = await waitFor(() => view.getByTestId("edge-pending"));
    expect(within(held).getByTestId("edge-deliver").textContent).toBe("Let it continue");
    expect(within(held).getByTestId("edge-dont-deliver").textContent).toBe("Don't deliver");
  });

  it("a narrow box at fit draws a two-line name at 12 px, so a long one reads whole", () => {
    const member = {
      id: first.id,
      session: { ...first, label: "Spike: payment sandbox" },
      card: { account: "Work Max", tool: "Claude Code", task: { value: "None" } } as unknown as ContextCard,
      role: "member",
      name: "A",
    };
    const node = (scale: number) => {
      const view = render(
        <CompactNode member={member} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={scale} onSelect={() => {}} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />,
      );
      const name = view.container.querySelector(`[data-node-name="${first.id}"]`) as HTMLElement;
      // A wrapped name's clamp is on its inner span (WebKit doesn't clamp a
      // button's own content); its size stays on the button.
      const text = name.querySelector<HTMLElement>("[data-node-name-text]");
      const classes = [...name.className.split(" "), ...(text ? text.className.split(" ") : [])];
      if (text) expect(name.className.split(" ")).not.toContain("line-clamp-2");
      view.unmount();
      return classes;
    };
    // Fit in a 1024 px window (56 %, a box about 130 px wide): 12 px, two lines.
    expect(node(0.56)).toEqual(expect.arrayContaining(["text-label", "line-clamp-2"]));
    // A wider box (69 %, about 160 px): 13 px, two lines.
    expect(node(0.69)).toEqual(expect.arrayContaining(["text-meta", "line-clamp-2"]));
    expect(node(0.69)).not.toContain("text-label");
    // rc7 walk: the 30 % floor (a box about 70 × 34 px) read "we…": the
    // name now wraps on two lines at 12 px there too.
    expect(node(0.3)).toEqual(expect.arrayContaining(["text-label", "line-clamp-2"]));
  });

  it("an ask's edge takes no label's room: the other way's badge stays on its own line", () => {
    // A pair: A → B carries a waiting message (a badge), B → A an agent's
    // ask (answered on B's node, so its edge has no label). The ask's edge
    // once reserved a label's room and pushed A → B's badge off its line.
    const points = new Map([
      ["a", { x: 0, y: 0 }],
      ["b", { x: 0, y: 400 }],
    ]);
    const at = Date.now();
    const pending = (kind: "waiting" | "asked") => [{ message_id: `m-${kind}`, thread_id: "thr", subject: "Totals", recipient: "session:x", kind, ask_id: kind === "asked" ? "ask-1" : null, at_ms: at }];
    const edges = [
      { from_session_id: "a", to_session_id: "b", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr", pending: pending("waiting") },
      { from_session_id: "b", to_session_id: "a", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr", pending: pending("asked") },
    ];
    const view = render(<CanvasEdges edges={edges} points={points} names={new Map([["a", "A"], ["b", "B"]])} selected={null} pulses={new Map()} reducedMotion scale={0.6} onSelect={() => {}} />);
    const labels = view.getAllByTestId("edge-label");
    expect(labels.map((label) => label.dataset.edge)).toEqual(["a>b"]);
    const mid = edgeCurve(points.get("a")!, points.get("b")!, true).mid;
    expect(labels[0]!.style.transform.startsWith(`translate(${mid.x}px, ${mid.y}px)`)).toBe(true);
    expect(within(labels[0]!).getByTestId("edge-badge").textContent).toBe("1 waiting");
  });

  it("back at 100 % the cards are whole again", async () => {
    seedTeam();
    const view = await openPage();
    zoomOut(view, 2);
    expect(view.container.querySelector("[data-form='compact']")).toBeTruthy();
    fireEvent.click(view.getByTestId("canvas-zoom"));
    expect(view.container.querySelector("[data-form='compact']")).toBeNull();
    expect(view.getByTestId("canvas-zoom").textContent).toBe("100%");
  });
});

describe("the List view under 1100 px", () => {
  it("a narrow window opens a pool as rows; the view you choose is kept for that pool", async () => {
    window.innerWidth = 1024;
    seedTeam();
    await act(async () => {
      await refreshPools();
    });
    const page = () => (
      <ConversationsProvider want={null}>
        <ActionHostProvider host={{}}>
          <PoolPage poolId="pool-team" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
        </ActionHostProvider>
      </ConversationsProvider>
    );
    let view = render(page());
    await waitFor(() => expect(view.getAllByTestId("pool-card").length).toBeGreaterThan(0));
    expect(view.queryByTestId("pool-canvas")).toBeNull();
    fireEvent.click(view.getByTestId("pool-mode-canvas"));
    await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
    cleanup();
    view = render(page());
    await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
    // A wide window with nothing chosen: the canvas.
    cleanup();
    localStorage.clear();
    window.innerWidth = 1280;
    view = render(page());
    await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
  });
});

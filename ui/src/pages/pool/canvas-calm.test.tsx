/**
 * The rc3 critique of the running canvas (D-126; pools are calm in 0.4.6,
 * calm r2's language): an App's logo muted at rest; one mark a need, on
 * its node, nothing on a line at rest; typical titles read whole at the
 * default view; a pool never laid out opens tidy, once.
 */
import { act, cleanup, fireEvent, render, renderHook, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_SESSIONS } from "../../preview/fixtures";
import { poolCanvasPlace, type CanvasEdgeInfo, type NodeAskInfo } from "../../lib/ipc/pool-canvas";
import { refreshPools } from "../../lib/pools";
import type { ContextCard } from "../../types/generated/ContextCard";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { first, resetCanvasTests, seams, second, seedTeam } from "./canvas.test-kit";
import { CanvasEdges } from "./CanvasEdges";
import { CanvasNode } from "./CanvasNode";
import { fitView, layout, nodeMiddle, NODE_H, NODE_W, type LayoutNode, type Point } from "./layout";
import { compactName, nameLines, type NodeProps, type Paused } from "./NodeParts";
import { tidyLayout, type Link } from "./tidy-layout";
import { useCanvas, type MemberNode } from "./useCanvas";

beforeEach(resetCanvasTests);
afterEach(cleanup);

const card: ContextCard = {
  session_id: first.id,
  name: "A",
  place: "shop · Work Max",
  tool: "Claude Code",
  account: "Work Max",
  state: "running",
  role: "member",
  task: { value: "None", unavailable: null },
  said: { value: null, unavailable: "Unavailable" },
  last_activity_ms: null,
  files: [],
  files_unavailable: null,
};
const member = (over: Partial<InstanceInfo> = {}): MemberNode => ({ id: first.id, session: { ...first, status: "running", needs_attention: null, ...over }, card, role: "member", name: "A" });
const ask: NodeAskInfo = { session_id: first.id, source: "message", id: "ask-1", title: "A wants to message B", offer_always: false, at_ms: 0 };
const hop = (to = first.id): CanvasEdgeInfo => ({ from_session_id: "s-cart", to_session_id: to, messages: 6, tasks: 0, last_at_ms: 0, thread_id: "t", pending: [{ message_id: "m", thread_id: "t", subject: "s", recipient: `session:${to}`, kind: "hop_limit", ask_id: "k", at_ms: 0 }] });
const paused: Paused = { edge: hop(), from: "Fix the cart totals" };
const draw = (props: Partial<NodeProps> = {}) =>
  render(<CanvasNode member={member()} at={{ x: 0, y: 0 }} asks={[]} paused={[]} hops={5} selected={false} tabStop dragging={false} scale={0.6} onSelect={() => {}} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} {...props} />);

describe("an App's logo on a node is muted at rest (calm r2)", () => {
  for (const scale of [1, 0.6]) {
    it(`at ${scale * 100} %: muted at rest, in full colour when it needs you or is chosen; hover and focus light it`, () => {
      const rest = draw({ scale });
      const logo = rest.getByTestId("node-logo");
      expect(logo.className).toContain("app-tone");
      expect(logo.className).not.toContain("is-lit");
      // Its card is the tone's host: hovered or focused, it lights.
      expect(logo.closest(".tone-host")).toBeTruthy();
      rest.unmount();
      for (const lit of [{ asks: [ask] }, { selected: true }, { paused: [paused] }]) {
        const view = draw({ scale, ...lit });
        expect(view.getByTestId("node-logo").className).toContain("is-lit");
        view.unmount();
      }
    });
  }
});

describe("one mark a need, on the node (the rc3 critique)", () => {
  it("below 100 %: its word in place of its state, counted, no badge; it opens every answer, a paused line's too", () => {
    const view = draw({ member: member({ status: "waiting_user", needs_attention: "waiting_user" }), paused: [paused] });
    expect(view.queryByTestId("node-badge")).toBeNull();
    const marks = view.getAllByTestId("node-need");
    expect(marks).toHaveLength(1);
    expect(marks[0]!.textContent).toBe("Asks you +1");
    expect(marks[0]!.getAttribute("aria-label")).toBe("Asks you +1: 2 waiting on you about A");
    expect(view.getByTestId("node-description").textContent).toContain("2 waiting on you");
    fireEvent.click(marks[0]!);
    const panel = view.getByTestId("node-popover");
    expect(within(panel).getByTestId("node-alert")).toBeTruthy();
    expect(within(panel).getByTestId("node-paused").textContent).toContain("Fix the cart totals: Paused after 5 in a row");
    expect(within(panel).getByTestId("edge-deliver").textContent).toBe("Let it continue");
  });

  it("at 100 %: no badge on its corner; a paused line is answered on the card", () => {
    const view = draw({ scale: 1, paused: [paused] });
    expect(view.queryByTestId("node-badge")).toBeNull();
    expect(view.getByTestId("node-paused").textContent).toContain("Paused after 5 in a row");
    expect(within(view.getByTestId("node-paused")).getByTestId("edge-dont-deliver")).toBeTruthy();
  });

  it("a line says nothing at rest (no chip, no badge, no amber); hovered, its words, quiet", () => {
    const at = 0;
    const edges: CanvasEdgeInfo[] = [hop("b"), { from_session_id: "a", to_session_id: "b", messages: 2, tasks: 0, last_at_ms: at, thread_id: "t2", pending: [{ message_id: "m2", thread_id: "t2", subject: "s", recipient: "session:b", kind: "waiting", ask_id: null, at_ms: at }] }];
    const points = new Map<string, Point>([["s-cart", { x: 0, y: 0 }], ["a", { x: 0, y: 300 }], ["b", { x: 400, y: 150 }]]);
    const view = render(<CanvasEdges edges={edges} points={points} names={new Map([["s-cart", "Fix the cart totals"], ["a", "A"], ["b", "B"]])} selected={null} pulses={new Map()} reducedMotion scale={0.6} hops={5} onSelect={() => {}} />);
    expect(view.queryAllByTestId("edge-label")).toEqual([]);
    expect(view.queryByTestId("edge-badge")).toBeNull();
    expect(view.queryByTestId("edge-pending")).toBeNull();
    expect(view.container.innerHTML).not.toMatch(/warning/);
    fireEvent.mouseEnter(view.container.querySelector('g[data-edge="s-cart>b"] path')!);
    const label = view.getByTestId("edge-label");
    expect(label.textContent).toBe("6 messages · Paused");
    expect(label.querySelector("button")!.className).toContain("text-muted");
  });
});

describe("a pair is one line (round 2: the mock draws one line a pair)", () => {
  const points = new Map<string, Point>([["a", { x: 0, y: 0 }], ["b", { x: 400, y: 0 }]]);
  const names = new Map([["a", "Planner"], ["b", "Fix the cart totals"]]);
  const way = (from: string, to: string, messages: number, waits = false): CanvasEdgeInfo => ({
    from_session_id: from,
    to_session_id: to,
    messages,
    tasks: 0,
    last_at_ms: 0,
    thread_id: "t",
    pending: waits ? [{ message_id: `m-${from}`, thread_id: "t", subject: "s", recipient: `session:${to}`, kind: "waiting", ask_id: null, at_ms: 0 }] : [],
  });

  it("two ways draw one path with an arrowhead at each end; a click chooses the way that needs you, else the busier", () => {
    const picked: string[] = [];
    const view = render(<CanvasEdges edges={[way("a", "b", 5), way("b", "a", 2, true)]} points={points} names={names} selected={null} pulses={new Map()} reducedMotion scale={1} onSelect={(key) => picked.push(key)} />);
    const groups = view.container.querySelectorAll("g[data-edge]");
    expect(groups).toHaveLength(1);
    expect(groups[0]!.getAttribute("data-edge")).toBe("b>a");
    expect(groups[0]!.getAttribute("data-edge-back")).toBe("a>b");
    const drawnLine = [...groups[0]!.querySelectorAll("path")].find((path) => path.getAttribute("marker-end"))!;
    expect(drawnLine.getAttribute("marker-start")).toBe(drawnLine.getAttribute("marker-end"));
    fireEvent.click(groups[0]!.querySelector("path")!);
    expect(picked).toEqual(["b>a"]);
    view.unmount();
    // Nothing waits: the busier way leads.
    const calm = render(<CanvasEdges edges={[way("a", "b", 5), way("b", "a", 2)]} points={points} names={names} selected={null} pulses={new Map()} reducedMotion scale={1} onSelect={() => {}} />);
    expect(calm.container.querySelector("g[data-edge]")!.getAttribute("data-edge")).toBe("a>b");
    // One way only: one arrowhead.
    calm.rerender(<CanvasEdges edges={[way("a", "b", 5)]} points={points} names={names} selected={null} pulses={new Map()} reducedMotion scale={1} onSelect={() => {}} />);
    const single = [...calm.container.querySelectorAll("g[data-edge] path")].find((path) => path.getAttribute("marker-end"))!;
    expect(single.getAttribute("marker-start")).toBeNull();
  });

  it("hovered with neither way chosen it speaks for both; chosen either way, the line is lit and speaks for that way", () => {
    const edges = [way("a", "b", 5), way("b", "a", 2)];
    const view = render(<CanvasEdges edges={edges} points={points} names={names} selected={null} pulses={new Map()} reducedMotion scale={1} onSelect={() => {}} />);
    fireEvent.mouseEnter(view.container.querySelector('g[data-edge="a>b"] path')!);
    expect(view.getByTestId("edge-label").textContent).toBe("7 messages · both ways");
    view.rerender(<CanvasEdges edges={edges} points={points} names={names} selected="b>a" pulses={new Map()} reducedMotion scale={1} onSelect={() => {}} />);
    const label = view.getByTestId("edge-label");
    expect(label.dataset.edge).toBe("b>a");
    expect(label.textContent).toBe("2 messages");
    const drawnLine = [...view.container.querySelectorAll('g[data-edge="a>b"] path')].find((path) => path.getAttribute("marker-end"))!;
    expect(drawnLine.getAttribute("class")).toContain("stroke-accent-hi");
  });
});

/** The gallery's Checkout team: the lead and five members, the lines
 * between them (the fixture's messages and its one task). */
const TEAM: LayoutNode[] = [
  { id: "s-plan", role: "lead" },
  ...["s-cart", "s-copy", "s-review", "s-tax", "s-spike"].map((id) => ({ id, role: "member" })),
];
const TITLES = ["Fix the cart totals", "Checkout copy pass", "Review the checkout steps", "Spike: payment sandbox", "Tax rules research"];
const link = (from: string, to: string): Link => ({ from_session_id: from, to_session_id: to });
const LINES = [link("s-plan", "s-cart"), link("s-cart", "s-plan"), link("s-copy", "s-plan"), link("s-review", "s-cart"), link("s-cart", "s-review"), link("s-plan", "s-copy")];

describe("typical titles read whole at the default view (the rc3 critique: cut at a 53 % fit)", () => {
  it("the six-member pool's fit in the running view leaves each title its two lines", () => {
    // The running view's canvas at 1280 × 800 (592 × 567): no palette
    // line drawn (it's quiet), the zoom bar at its foot.
    const fit = fitView(layout(TEAM, new Map()).values(), 592, 567, 48, 0, 0, 52);
    expect(fit.scale).toBeGreaterThanOrEqual(0.62);
    for (const title of TITLES) {
      const name = compactName(title, fit.scale, false, false);
      expect(name.lines, title).toBe(2);
      expect(name.wrapWords, title).toBe(true);
      expect(nameLines(title, name.room, name.fontPx), title).toBeLessThanOrEqual(2);
    }
  });
});

/** How untidy a layout reads: lines across lines, and lines behind a
 * third node (straight, middle to middle). */
function untidy(at: ReadonlyMap<string, Point>, lines: readonly Link[]): number {
  const seen = new Set<string>();
  const segs = lines.flatMap((one) => {
    const key = [one.from_session_id, one.to_session_id].sort().join("|");
    if (seen.has(key)) return [];
    seen.add(key);
    return [{ a: one.from_session_id, b: one.to_session_id, p: nodeMiddle(at.get(one.from_session_id)!), q: nodeMiddle(at.get(one.to_session_id)!) }];
  });
  const side = (p: Point, q: Point, r: Point) => Math.sign((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x));
  let count = 0;
  segs.forEach((one, i) => {
    for (const two of segs.slice(i + 1)) {
      if ([two.a, two.b].some((id) => id === one.a || id === one.b)) continue;
      if (side(one.p, one.q, two.p) * side(one.p, one.q, two.q) < 0 && side(two.p, two.q, one.p) * side(two.p, two.q, one.q) < 0) count += 1;
    }
    for (const [id, box] of at) {
      if (id === one.a || id === one.b) continue;
      const hit = Array.from({ length: 31 }, (_, n) => ({ x: one.p.x + ((one.q.x - one.p.x) * n) / 30, y: one.p.y + ((one.q.y - one.p.y) * n) / 30 })).some((p) => p.x > box.x && p.x < box.x + NODE_W && p.y > box.y && p.y < box.y + NODE_H);
      if (hit) count += 1;
    }
  });
  return count;
}

describe("a pool never laid out opens tidy, once (the rc3 critique: its lines crossed)", () => {
  it("the tidy layout puts the members so no line crosses another or runs behind a node, the same way each time", () => {
    expect(untidy(layout(TEAM, new Map()), LINES)).toBeGreaterThan(0);
    const tidy = tidyLayout(TEAM, LINES)!;
    expect(untidy(tidy, LINES)).toBe(0);
    // The automatic layout's places, the lead still on top.
    const places = (at: ReadonlyMap<string, Point>) => [...at.values()].map((p) => `${p.x},${p.y}`).sort();
    expect(places(tidy)).toEqual(places(layout(TEAM, new Map())));
    expect(tidy.get("s-plan")).toEqual(layout(TEAM, new Map()).get("s-plan"));
    expect([...tidyLayout(TEAM, LINES)!]).toEqual([...tidy]);
    // No line between two of them: nothing to tidy.
    expect(tidyLayout(TEAM, [link("s-plan", "s-nobody")])).toBeNull();
  });

  it("a big pool is tidied in bounded time, everyone in a place of their own", () => {
    const many: LayoutNode[] = [{ id: "lead", role: "lead" }, ...Array.from({ length: 14 }, (_, n) => ({ id: `m${String(n).padStart(2, "0")}`, role: "member" }))];
    const lines = many.slice(1).map((one, n) => link(one.id, n % 3 === 0 ? "lead" : many[1 + ((n * 5) % 14)]!.id));
    const started = performance.now();
    const tidy = tidyLayout(many, lines)!;
    expect(performance.now() - started).toBeLessThan(2000);
    expect(new Set([...tidy.values()].map((p) => `${p.x},${p.y}`)).size).toBe(many.length);
    expect(untidy(tidy, lines)).toBeLessThanOrEqual(untidy(layout(many, new Map()), lines));
  });

  const canvasCalls = () => (seams().__AIJ_POOL_CANVAS_CALLS__ ?? []).filter((call) => call.startsWith("place:"));

  it("its first open with lines keeps the tidy places; the next open, and a pool you arranged, are left as they are", async () => {
    seedTeam();
    await act(async () => {
      await refreshPools();
    });
    const first_ = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    await waitFor(() => expect(canvasCalls()).toHaveLength(1));
    expect(canvasCalls()[0]).toMatch(new RegExp(`^place:pool-team:.*${first.id}@.*${second.id}@|^place:pool-team:.*${second.id}@.*${first.id}@`));
    expect(first_.result.current.arranged).toBe(true);
    first_.unmount();
    // Opened again: its places are kept, nothing is laid out again.
    const again = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    await waitFor(() => expect(again.result.current.canvas?.positions.length).toBe(2));
    expect(canvasCalls()).toHaveLength(1);
    again.unmount();
  });

  it("a pool with its own places, or with no lines yet, isn't touched", async () => {
    seedTeam();
    await poolCanvasPlace("pool-team", [{ session_id: first.id, x: 400, y: 300 }]);
    seams().__AIJ_POOL_CANVAS_CALLS__ = [];
    await act(async () => {
      await refreshPools();
    });
    const placed = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    await waitFor(() => expect(placed.result.current.canvas?.positions.length).toBe(1));
    await act(async () => {
      await new Promise((done) => setTimeout(done, 50));
    });
    expect(canvasCalls()).toEqual([]);
    expect(placed.result.current.points.get(first.id)).toEqual({ x: 400, y: 300 });
    placed.unmount();
    // No lines yet: the automatic layout is already tidy; nothing kept.
    resetCanvasTests();
    seedTeam();
    seams().__AIJ_MESSAGES__ = { threads: [], messages: {}, needs: [] };
    await act(async () => {
      await refreshPools();
    });
    const quiet = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    await waitFor(() => expect(quiet.result.current.canvas?.anchors[first.id]).toBeTruthy());
    await act(async () => {
      await new Promise((done) => setTimeout(done, 50));
    });
    expect(canvasCalls()).toEqual([]);
    expect(quiet.result.current.arranged).toBe(false);
    quiet.unmount();
  });
});

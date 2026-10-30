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
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS, type InstanceInfo } from "../../lib/bridge";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewMessages } from "../../lib/ipc/messages";
import { resetPreviewCanvas } from "../../lib/ipc/pool-canvas";
import { forgetPaged } from "../../lib/paged";
import { refreshPools, resetPools, defaultPoolIdOf } from "../../lib/pools";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ToastHost, resetToasts } from "../../components/Toast";
import { pick } from "../../test/pick";
import { rememberTab } from "../registry";
import { PoolPage } from "../PoolPage";
import { forgetCanvasViews } from "./Canvas";
import { cardOrder, cardWords, newPoolName, PoolsScreen } from "../../screens/pools/PoolsScreen";
import { deliveryWords } from "./InspectPool";
import { CanvasEdges } from "./CanvasEdges";
import { CompactNode } from "./CompactNode";
import type { ContextCard } from "../../types/generated/ContextCard";
import { curvePoint, edgeCurve, fitView, nodeMiddle, layout, neighbour, NODE_H, NODE_W, nodeForm, zoomAt } from "./layout";

type Seams = Record<string, unknown> & { __AIJ_POOL_CALLS__?: string[]; __AIJ_MESSAGES_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const [first, second, third] = MOCK_SESSIONS as [InstanceInfo, InstanceInfo, InstanceInfo];

beforeEach(() => {
  // A desktop window: under 1100 px a pool starts in the List view.
  Object.defineProperty(window, "innerWidth", { value: 1280, configurable: true, writable: true });
  resetPreviewPools();
  resetPreviewMessages();
  resetPreviewCanvas();
  forgetCanvasViews();
  resetPools();
  forgetPaged();
  resetToasts();
  for (const key of ["__AIJ_POOLS__", "__AIJ_POOL_CALLS__", "__AIJ_MESSAGES__", "__AIJ_MESSAGES_CALLS__", "__AIJ_POOL_CANVAS__", "__AIJ_POOL_CANVAS_CALLS__"]) delete seams()[key];
});
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
    const pool = (id: string, kind: string, members: number) => ({ id, name: id, kind, project_id: kind === "default" ? id : null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members });
    const order = cardOrder([pool("quiet", "default", 3), pool("busy", "default", 1), pool("empty", "default", 0), pool("mine", "user", 0)], new Map([["busy", 1]]));
    expect(order.map((row) => row.id)).toEqual(["mine", "busy", "quiet"]);
  });
  it("says a card's line in words", () => {
    expect(cardWords(3, 1, 2)).toBe("3 members · 1 needs you · 2 tasks moving");
    expect(cardWords(1, 0, 0)).toBe("1 member · quiet");
    expect(cardWords(0, 0, 0)).toBe("0 members");
  });
});

function seedTeam(): void {
  seams().__AIJ_POOLS__ = {
    pools: [{ id: "pool-team", name: "Team", rules: [{ id: 7, pool_id: "pool-team", action: "claim_task", created_at: Date.now() - 60_000, made_by: "user" }] }],
    members: [
      { session_id: first.id, pool_id: "pool-team", role: "lead" },
      { session_id: second.id, pool_id: "pool-team", role: "member" },
    ],
  };
  const at = Date.now() - 60_000;
  seams().__AIJ_MESSAGES__ = {
    threads: [{ id: "thr-1", pool_id: "pool-team", pool_name: "Team", subject: "Totals", people: [], count: 1, hop: 1, hop_cap: 6, state: "open", state_reason: null, updated_at_ms: at, last_from: "", last_line: "", unread: 0 }],
    messages: {
      "thr-1": [
        { id: "m-1", thread_id: "thr-1", from_name: "B", from_kind: "agent", from_session_id: second.id, kind: "note", subject: "Totals", body: "May I?", hop: 1, redacted: 0, to_all: false, at_ms: at, recipients: [{ id: `session:${first.id}`, name: "A", session_id: first.id, state: "held", via: null, deliverable: false, how: "" }] },
      ],
    },
    needs: [{ key: "ask:ask-1", kind: "ask", pool_id: "pool-team", pool_name: "Team", thread_id: "thr-1", message_id: "m-1", session_id: null, ask_id: "ask-1", offer_always: false, title: "B wants to message A", detail: "", preview: "May I?", at_ms: at }],
  };
}

async function openPage(poolId = "pool-team") {
  await act(async () => {
    await refreshPools();
  });
  const view = render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId={poolId} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} now={Date.now()} />
        <ToastHost />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
  await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
  return view;
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
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} />
      </ActionHostProvider>,
    );
    await waitFor(() => expect(screen.getAllByTestId("pool-card-words").some((line) => line.textContent!.includes(header!))).toBe(true));
  });
});

describe("a pool's page: its canvas", () => {
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

  it("says how a message gets into a session", () => {
    const pool = { id: "p", name: "p", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0 };
    expect(deliveryWords(pool)).toBe("When you click Deliver");
    expect(deliveryWords({ ...pool, access: { ...pool.access, level: "full" } })).toBe("By themselves, between turns");
    expect(deliveryWords({ ...pool, access: { ...pool.access, delivery_stopped: true } })).toBe("Stopped: nothing goes into a session");
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

  it("a narrow box at fit draws a two-line name at 12 px, so a long one reads whole", () => {
    const member = {
      id: first.id,
      session: { ...first, label: "Spike: payment sandbox" },
      card: { account: "Work Max", tool: "Claude Code", task: { value: "None" } } as unknown as ContextCard,
      role: "member",
    };
    const node = (scale: number) => {
      const view = render(
        <CompactNode member={member} at={{ x: 0, y: 0 }} asks={[]} held={0} selected={false} tabStop dragging={false} scale={scale} onSelect={() => {}} onMenu={() => {}} onPointerDown={() => {}} onKey={() => {}} />,
      );
      const name = view.container.querySelector(`[data-node-name="${first.id}"]`) as HTMLElement;
      const classes = name.className.split(" ");
      view.unmount();
      return classes;
    };
    // Fit in a 1024 px window (56 %, a box about 130 px wide): 12 px, two lines.
    expect(node(0.56)).toEqual(expect.arrayContaining(["text-label", "line-clamp-2"]));
    // A wider box (69 %, about 160 px): 13 px, two lines.
    expect(node(0.69)).toEqual(expect.arrayContaining(["text-meta", "line-clamp-2"]));
    expect(node(0.69)).not.toContain("text-label");
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
          <PoolPage poolId="pool-team" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} now={Date.now()} />
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

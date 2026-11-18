/**
 * The pool canvas's edges: messages and task hand-offs from one member to
 * another over the last week, one arrow a direction, thicker the more
 * there were. New activity sends a dot along its edge (a brief glow
 * instead when you'd rather have less motion).
 *
 * Calm (the rc3 critique, D-117): at rest a line is quiet and says
 * nothing; hovered or chosen it takes its colour and its words ("3
 * messages · Paused"), dashed while something on it waits. What waits on
 * you is its member's one mark: a chain paused at the hop limit on the
 * node it was going to (Let it continue, Don't deliver: `PausedLine`),
 * an agent's ask on its sender's; a message waiting for a session run
 * here goes in at its next turn, its Deliver in the line's inspector.
 */
import { memo, useMemo, useState, type JSX } from "react";

import { errorText } from "../../lib/bridge";
import { messagesAnswer, messagesDeliver } from "../../lib/ipc/messages";
import type { CanvasEdgeInfo, PendingInfo } from "../../lib/ipc/pool-canvas";
import type { TalkRule } from "../../lib/ipc/pool-model";
import { curvePoint, edgeCurve, labelOnNode, nodeForm, nodeMiddle, type LabelRoom, type Point } from "./layout";
import { edgeKey } from "./useCanvas";

export type EdgeWords = { count: string; aria: string };

/** A line's own talk rule on its label (P2 step 3: set on the canvas, it
 * shows there, not only in the inspector). */
const OWN_RULE_WORDS: Record<TalkRule, string> = { allowed: "Allowed here", ask: "Ask first here", blocked: "Blocked here" };

/** An edge in words: "3 messages · 1 task", and its own rule when the
 * line has one ("1 message · Allowed here"). */
export function edgeWords(edge: CanvasEdgeInfo, from: string, to: string, own?: TalkRule): EdgeWords {
  const parts = [
    edge.messages > 0 ? `${edge.messages} ${edge.messages === 1 ? "message" : "messages"}` : null,
    edge.tasks > 0 ? `${edge.tasks} ${edge.tasks === 1 ? "task" : "tasks"}` : null,
  ].filter(Boolean) as string[];
  const counted = parts.join(" · ") || "Waiting";
  const count = own ? `${counted} · ${OWN_RULE_WORDS[own]}` : counted;
  return { count, aria: `${count}, ${from} to ${to}` };
}

/** The lines with their own rule, by edge key: the pool's talk rows
 * whose `from` and `to` are a line's two ends' anchors (the canvas's
 * `anchors`, by node id: `work:<id>` for a session in a Work item, an
 * App's too; else `session:<id>`; a desktop node's `conversation:` ref).
 * One-way: A→B's own rule leaves B→A to its pair. */
export function ownLineRules(talk: readonly { from: string; to: string; rule: string }[], anchors: Readonly<Record<string, string>> = {}): LineRules {
  const out = new Map<string, TalkRule>();
  const nodes = new Map<string, string[]>();
  for (const [id, anchor] of Object.entries(anchors)) nodes.set(anchor, [...(nodes.get(anchor) ?? []), id]);
  for (const { from, to, rule } of talk) {
    if (rule !== "allowed" && rule !== "ask" && rule !== "blocked") continue;
    for (const a of nodes.get(from) ?? []) for (const b of nodes.get(to) ?? []) out.set(edgeKey(a, b), rule);
  }
  return out;
}

/** The lines with their own talk rule, by edge key. */
export type LineRules = ReadonlyMap<string, TalkRule>;

/** What Deliver does for each kind of waiting message. */
export async function settle(pending: PendingInfo, deliver: boolean): Promise<void> {
  if (pending.kind === "waiting") {
    if (deliver) await messagesDeliver(pending.message_id, pending.recipient);
    return;
  }
  if (pending.ask_id) await messagesAnswer(pending.ask_id, deliver ? "once" : "deny");
}

/** What waits on an edge, in words: the panel's line and the compact
 * badge's ("Paused after 5 in a row" / "Paused"). None when nothing on
 * it is answered here (one above the pool's level is answered on its
 * sender's node). */
export function pendingWords(edge: CanvasEdgeInfo, hops?: number): { label: string; short: string; hop: boolean } | null {
  const actionable = edge.pending.filter((p) => p.kind !== "asked");
  const first = actionable[0];
  if (!first) return null;
  const hop = first.kind === "hop_limit";
  // The pool's own number, where it's known (design review #76).
  if (hop) return { label: hops ? `Paused after ${hops} in a row` : "Paused: too many replies in a row", short: "Paused", hop };
  return { label: actionable.length === 1 ? "1 message waiting" : `${actionable.length} messages waiting`, short: `${actionable.length} waiting`, hop };
}

/** The answer an edge carries (the first that waits): in the inspector
 * and the List view; `bare`, its buttons only (on its node's line). */
export function PendingChip({ edge, compact = false, bare = false, hops }: { edge: CanvasEdgeInfo; compact?: boolean; bare?: boolean; hops?: number }): JSX.Element | null {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const actionable = edge.pending.filter((p) => p.kind !== "asked");
  const first = actionable[0];
  const words = pendingWords(edge, hops);
  if (!first || !words) return null;
  const hop = words.hop;
  const run = (deliver: boolean) => {
    setBusy(true);
    setError(null);
    void (hop ? settle(first, deliver) : Promise.all(actionable.filter((p) => p.kind === "waiting").map((p) => settle(p, true)))).then(
      () => setBusy(false),
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const label = words.label;
  const button = "whitespace-nowrap rounded-md border px-1.5 py-0.5 text-label outline-none focus-visible:ring-1 focus-visible:ring-accent/60 disabled:opacity-60";
  return (
    <span className={bare ? "flex flex-col gap-1" : "flex flex-col items-center gap-1 rounded-lg border border-warning/50 bg-surface px-2 py-1 shadow-card"} data-testid="edge-pending" role="group" aria-label={label}>
      {!compact && !bare && <span className="whitespace-nowrap text-label text-warning">{label}</span>}
      <span className="flex gap-1">
        <button type="button" disabled={busy} onClick={() => run(true)} className={button + " border-warning/60 bg-warning/15 font-medium text-ink hover:bg-warning/25"} data-testid="edge-deliver">
          {hop ? "Let it continue" : "Deliver"}
        </button>
        {hop && (
          <button type="button" disabled={busy} onClick={() => run(false)} className={button + " border-hairline text-muted hover:bg-surface-2 hover:text-ink"} data-testid="edge-dont-deliver">
            Don't deliver
          </button>
        )}
      </span>
      {error && (
        <span className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </span>
  );
}

/** A label's room in canvas units: its words at 100 % and up (about 120
 * × 48; a node grows below its box with an ask), or at screen size below
 * (about 110 × 26, so larger in canvas units the further out). */
const FULL_ROOM: LabelRoom = { w: 120, h: 48, below: 60 };

/** Where an edge's label goes: on its own line, at its middle or further
 * along, where it covers no node, no other label and no other edge's line
 * (so it never reads as another edge's); else the nearest free place
 * above or below its middle. */
function apart<T extends { key: string; mid: Point; along: Point[] }>(labels: T[], nodes: Iterable<Point>, lines: readonly { key: string; points: Point[] }[], room: LabelRoom = FULL_ROOM): T[] {
  const boxes = [...nodes];
  const placed: Point[] = [];
  const { w, h } = room;
  const onNode = (at: Point) => labelOnNode(at, boxes, room);
  const onLabel = (at: Point) => placed.some((other) => Math.abs(other.x - at.x) < w + 40 && Math.abs(other.y - at.y) < h + 10);
  const sx = w * 0.75;
  const sy = h * 1.25;
  return labels.map((label) => {
    const onLine = (at: Point) => lines.some((line) => line.key !== label.key && line.points.some((p) => Math.abs(p.x - at.x) < w / 2 && Math.abs(p.y - at.y) < h / 2));
    const free = (spot: Point) => !onNode(spot) && !onLabel(spot);
    const tries = [[0, 0], [0, -1], [0, 1], [-1, 0], [1, 0], [-1, -1], [1, -1], [-1, 1], [1, 1], [0, -2], [0, 2]].map(([dx, dy]) => ({ x: label.mid.x + dx! * sx, y: label.mid.y + dy! * sy }));
    const at = label.along.find((spot) => free(spot) && !onLine(spot)) ?? tries.find(free) ?? tries.find((spot) => !onLabel(spot)) ?? label.mid;
    placed.push(at);
    return { ...label, mid: at };
  });
}

/** Where along its own line an edge's label may sit, best first. */
const ALONG = [0.5, 0.4, 0.6, 0.3, 0.7];

/** `waits`: something waits on the edge (hovered or chosen, it draws
 * dashed). */
type Drawn = { edge: CanvasEdgeInfo; key: string; d: string; mid: Point; along: Point[]; line: Point[]; width: number; waits: boolean; tasksOnly: boolean };

export const CanvasEdges = memo(function CanvasEdges({
  edges,
  points,
  names,
  selected,
  pulses,
  reducedMotion,
  scale,
  hops,
  lineRules,
  onSelect,
}: {
  edges: CanvasEdgeInfo[];
  points: ReadonlyMap<string, Point>;
  names: ReadonlyMap<string, string>;
  selected: string | null;
  pulses: ReadonlyMap<string, number>;
  reducedMotion: boolean;
  /** The canvas's zoom: under 100 % labels draw at screen size (semantic
   * zoom). */
  scale: number;
  /** The pool's replies in a row before a chain pauses. */
  hops?: number;
  lineRules?: LineRules;
  onSelect: (key: string) => void;
}): JSX.Element {
  const [hover, setHover] = useState<string | null>(null);
  const compact = !nodeForm(scale).full;
  const unit = compact ? 1 / scale : 1;
  const room: LabelRoom = useMemo(() => (compact ? { w: 110 * unit, h: 26 * unit, below: 13 * unit } : FULL_ROOM), [compact, unit]);
  // The routes and their label spots: again only when the nodes, the edges
  // or the zoom change, not on hover or selection.
  const drawn = useMemo(() => {
    const keys = new Set(edges.map((edge) => edgeKey(edge.from_session_id, edge.to_session_id)));
    // Each edge's line from middle to middle: a detour's label keeps off them.
    const lines = edges.flatMap((edge) => {
      const from = points.get(edge.from_session_id);
      const to = points.get(edge.to_session_id);
      return from && to ? [{ key: edgeKey(edge.from_session_id, edge.to_session_id), line: [nodeMiddle(from), nodeMiddle(to)] as const }] : [];
    });
    const out: Drawn[] = [];
    for (const edge of edges) {
      const from = points.get(edge.from_session_id);
      const to = points.get(edge.to_session_id);
      if (!from || !to) continue;
      const key = edgeKey(edge.from_session_id, edge.to_session_id);
      const back = edgeKey(edge.to_session_id, edge.from_session_id);
      const around = [...points].filter(([id]) => id !== edge.from_session_id && id !== edge.to_session_id).map(([, at]) => at);
      const busy = lines.filter((other) => other.key !== key && other.key !== back).map((other) => other.line);
      const curve = edgeCurve(from, to, keys.has(back), around, busy, room);
      const total = edge.messages + edge.tasks;
      const at = (t: number) => curvePoint(curve.start, curve.control, curve.end, t);
      out.push({ edge, key, d: curve.d, mid: curve.mid, along: ALONG.map(at), line: Array.from({ length: 17 }, (_, n) => at(n / 16)), width: Math.min(4, 1.5 + Math.log2(1 + total) * 0.6), waits: edge.pending.length > 0, tasksOnly: edge.messages === 0 && edge.tasks > 0 });
    }
    return out;
  }, [edges, points, room]);
  return (
    <>
      <svg className="pointer-events-none absolute left-0 top-0 overflow-visible" width="1" height="1" aria-hidden="true" data-testid="canvas-edges">
        <defs>
          {(["rest", "msg", "task", "sel"] as const).map((kind) => (
            <marker key={kind} id={`canvas-arrow-${kind}`} viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M0 1 L9 5 L0 9 Z" className={kind === "rest" ? "fill-muted" : kind === "msg" ? "fill-accent" : kind === "task" ? "fill-live" : "fill-accent-hi"} />
            </marker>
          ))}
        </defs>
        {drawn.map(({ key, d, width, waits, tasksOnly }) => {
          const on = selected === key;
          // Hovered or chosen, a line says what it is (its colour, dashed
          // while something on it waits); at rest it is quiet.
          const lit = on || hover === key;
          const pulse = pulses.has(key);
          const own = lineRules?.get(key);
          const tone = on ? "stroke-accent-hi" : own === "blocked" ? "stroke-danger" : !lit ? "stroke-muted" : tasksOnly ? "stroke-live" : "stroke-accent";
          return (
            <g key={key} data-edge={key}>
              <path
                d={d}
                fill="none"
                strokeWidth={14}
                className="pointer-events-auto cursor-pointer stroke-transparent"
                onMouseEnter={() => setHover(key)}
                onMouseLeave={() => setHover((was) => (was === key ? null : was))}
                onClick={(event) => {
                  event.stopPropagation();
                  onSelect(key);
                }}
              />
              <path
                d={d}
                fill="none"
                strokeWidth={on || (pulse && reducedMotion) ? width + 1.5 : width}
                strokeDasharray={waits && lit ? "6 5" : own === "blocked" ? "2 4" : undefined}
                markerEnd={`url(#canvas-arrow-${on ? "sel" : !lit ? "rest" : tasksOnly ? "task" : "msg"})`}
                className={tone + " transition-[stroke-width] duration-200" + (lit ? "" : " opacity-80")}
              />
              {pulse && !reducedMotion && (
                <circle r="4.5" className="fill-accent-hi" data-testid="edge-pulse">
                  <animateMotion dur="1.2s" repeatCount="1" fill="freeze" path={d} />
                  <animate attributeName="opacity" values="1;1;0" keyTimes="0;0.8;1" dur="1.2s" fill="freeze" />
                </circle>
              )}
            </g>
          );
        })}
      </svg>
      {apart(
        drawn.filter(({ key }) => selected === key || hover === key || lineRules?.has(key)),
        points.values(),
        drawn.map(({ key, line }) => ({ key, points: line })),
        room,
      ).map(({ edge, key, mid }) => {
          const from = names.get(edge.from_session_id) ?? "A member";
          const own = lineRules?.get(key);
          const said = edgeWords(edge, from, names.get(edge.to_session_id) ?? "a member", own);
          // What waits on it, in its words, quiet ("2 messages · Paused").
          const waiting = selected === key || hover === key ? pendingWords(edge, hops)?.short : undefined;
          const words = waiting ? { count: `${said.count} · ${waiting}`, aria: `${said.count} · ${waiting}, ${from} to ${names.get(edge.to_session_id) ?? "a member"}` } : said;
          return (
            <div
              key={key}
              className="absolute left-0 top-0 z-10 flex flex-col items-center gap-1"
              // Below 100 %: undo the canvas's scale, so it reads at screen size.
              style={{ transform: `translate(${mid.x}px, ${mid.y}px)${compact ? ` scale(${unit})` : ""} translate(-50%, -50%)` }}
              data-testid="edge-label"
              data-edge={key}
            >
              {(selected === key || hover === key || own) && (
              <button
                type="button"
                tabIndex={-1}
                aria-label={words.aria}
                onClick={(event) => {
                  event.stopPropagation();
                  onSelect(key);
                }}
                className={
                  "whitespace-nowrap rounded-full border bg-surface px-2 py-0.5 text-label outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 " +
                  (selected === key ? "border-accent text-ink" : "border-hairline text-muted")
                }
              >
                {words.count}
              </button>
              )}
            </div>
          );
        })}
    </>
  );
});

/**
 * The pool canvas's edges: messages and task hand-offs from one member to
 * another over the last week, one arrow a direction, thicker the more
 * there were. New activity sends a dot along its edge (a brief glow
 * instead when you'd rather have less motion). What waits on an edge
 * sits on it with its answer: Deliver for a message waiting for a session
 * run here; Let it continue or Don't deliver for a chain at the hop limit.
 */
import { memo, useEffect, useRef, useState, type JSX } from "react";

import { errorText } from "../../lib/bridge";
import { messagesAnswer, messagesDeliver } from "../../lib/ipc/messages";
import type { CanvasEdgeInfo, PendingInfo } from "../../lib/ipc/pool-canvas";
import { edgeCurve, NODE_H, NODE_W, nodeForm, type Point } from "./layout";
import { edgeKey } from "./useCanvas";

export type EdgeWords = { count: string; aria: string };

/** An edge in words: "3 messages · 1 task". */
export function edgeWords(edge: CanvasEdgeInfo, from: string, to: string): EdgeWords {
  const parts = [
    edge.messages > 0 ? `${edge.messages} ${edge.messages === 1 ? "message" : "messages"}` : null,
    edge.tasks > 0 ? `${edge.tasks} ${edge.tasks === 1 ? "task" : "tasks"}` : null,
  ].filter(Boolean) as string[];
  const count = parts.join(" · ") || "Waiting";
  return { count, aria: `${count}, ${from} to ${to}` };
}

/** What Deliver does for each kind of waiting message. */
export async function settle(pending: PendingInfo, deliver: boolean): Promise<void> {
  if (pending.kind === "waiting") {
    if (deliver) await messagesDeliver(pending.message_id, pending.recipient);
    return;
  }
  if (pending.ask_id) await messagesAnswer(pending.ask_id, deliver ? "once" : "deny");
}

/** What waits on an edge, in words: the panel's line and the compact
 * badge's ("Paused at the hop limit" / "Paused"). None when nothing on
 * it is answered here (one above the pool's level is answered on its
 * sender's node). */
export function pendingWords(edge: CanvasEdgeInfo): { label: string; short: string; hop: boolean } | null {
  const actionable = edge.pending.filter((p) => p.kind !== "asked");
  const first = actionable[0];
  if (!first) return null;
  const hop = first.kind === "hop_limit";
  if (hop) return { label: "Paused at the hop limit", short: "Paused", hop };
  return { label: actionable.length === 1 ? "1 message waiting" : `${actionable.length} messages waiting`, short: `${actionable.length} waiting`, hop };
}

/** The answer an edge carries in place (the first that waits). */
export function PendingChip({ edge, compact = false }: { edge: CanvasEdgeInfo; compact?: boolean }): JSX.Element | null {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const actionable = edge.pending.filter((p) => p.kind !== "asked");
  const first = actionable[0];
  const words = pendingWords(edge);
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
    <span className="flex flex-col items-center gap-1 rounded-lg border border-warning/50 bg-surface px-2 py-1 shadow-card" data-testid="edge-pending" role="group" aria-label={label}>
      {!compact && <span className="whitespace-nowrap text-label text-warning">{label}</span>}
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

/** A label's room in canvas units: the full card's chip (about 120 × 48;
 * a node grows below its box with an ask), or a compact badge at screen
 * size (about 110 × 26, so larger in canvas units the further out). */
type LabelRoom = { w: number; h: number; below: number };
const FULL_ROOM: LabelRoom = { w: 120, h: 48, below: 60 };

/** Where an edge's label goes: at its middle unless that covers a node
 * or another label, else the nearest free place above or below it. */
function apart<T extends { mid: Point }>(labels: T[], nodes: Iterable<Point>, room: LabelRoom = FULL_ROOM): T[] {
  const boxes = [...nodes];
  const placed: Point[] = [];
  const { w, h, below } = room;
  const onNode = (at: Point) => boxes.some((node) => at.x > node.x - w / 2 && at.x < node.x + NODE_W + w / 2 && at.y > node.y - h / 2 && at.y < node.y + NODE_H + below);
  const onLabel = (at: Point) => placed.some((other) => Math.abs(other.x - at.x) < w + 40 && Math.abs(other.y - at.y) < h + 10);
  const sx = w * 0.75;
  const sy = h * 1.25;
  return labels.map((label) => {
    const tries = [[0, 0], [0, -1], [0, 1], [-1, 0], [1, 0], [-1, -1], [1, -1], [-1, 1], [1, 1], [0, -2], [0, 2]].map(([dx, dy]) => ({ x: label.mid.x + dx! * sx, y: label.mid.y + dy! * sy }));
    const at = tries.find((spot) => !onNode(spot) && !onLabel(spot)) ?? tries.find((spot) => !onLabel(spot)) ?? label.mid;
    placed.push(at);
    return { ...label, mid: at };
  });
}

/** `waits`: something waits on the edge (it draws dashed); `answers`: it
 * carries an answer here (a badge or chip). An agent's ask waits on its
 * sender's node, so its edge is dashed but has no label: it must not
 * take a label's room and push another edge's badge off its line. */
type Drawn = { edge: CanvasEdgeInfo; key: string; d: string; mid: Point; width: number; waits: boolean; answers: boolean; tasksOnly: boolean };

export const CanvasEdges = memo(function CanvasEdges({
  edges,
  points,
  names,
  selected,
  pulses,
  reducedMotion,
  scale,
  onSelect,
}: {
  edges: CanvasEdgeInfo[];
  points: ReadonlyMap<string, Point>;
  names: ReadonlyMap<string, string>;
  selected: string | null;
  pulses: ReadonlyMap<string, number>;
  reducedMotion: boolean;
  /** The canvas's zoom: under 100 % labels draw at screen size, and what
   * waits on an edge is a badge that opens its answers (semantic zoom). */
  scale: number;
  onSelect: (key: string) => void;
}): JSX.Element {
  const [hover, setHover] = useState<string | null>(null);
  const compact = !nodeForm(scale).full;
  const unit = compact ? 1 / scale : 1;
  const room: LabelRoom = compact ? { w: 110 * unit, h: 26 * unit, below: 13 * unit } : FULL_ROOM;
  const keys = new Set(edges.map((edge) => edgeKey(edge.from_session_id, edge.to_session_id)));
  const drawn: Drawn[] = [];
  for (const edge of edges) {
    const from = points.get(edge.from_session_id);
    const to = points.get(edge.to_session_id);
    if (!from || !to) continue;
    const key = edgeKey(edge.from_session_id, edge.to_session_id);
    const curve = edgeCurve(from, to, keys.has(edgeKey(edge.to_session_id, edge.from_session_id)));
    const total = edge.messages + edge.tasks;
    drawn.push({ edge, key, d: curve.d, mid: curve.mid, width: Math.min(4, 1.5 + Math.log2(1 + total) * 0.6), waits: edge.pending.length > 0, answers: pendingWords(edge) !== null, tasksOnly: edge.messages === 0 && edge.tasks > 0 });
  }
  return (
    <>
      <svg className="pointer-events-none absolute left-0 top-0 overflow-visible" width="1" height="1" aria-hidden="true" data-testid="canvas-edges">
        <defs>
          {(["msg", "task", "wait", "sel"] as const).map((kind) => (
            <marker key={kind} id={`canvas-arrow-${kind}`} viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M0 1 L9 5 L0 9 Z" className={kind === "msg" ? "fill-accent" : kind === "task" ? "fill-live" : kind === "wait" ? "fill-warning" : "fill-accent-hi"} />
            </marker>
          ))}
        </defs>
        {drawn.map(({ key, d, width, waits, tasksOnly }) => {
          const on = selected === key;
          const pulse = pulses.has(key);
          const tone = on ? "stroke-accent-hi" : waits ? "stroke-warning" : tasksOnly ? "stroke-live" : "stroke-accent";
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
                strokeDasharray={waits ? "6 5" : undefined}
                markerEnd={`url(#canvas-arrow-${on ? "sel" : waits ? "wait" : tasksOnly ? "task" : "msg"})`}
                className={tone + " transition-[stroke-width] duration-200" + (on ? "" : " opacity-80")}
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
      {apart(drawn.filter(({ key, answers }) => answers || selected === key || hover === key), points.values(), room).map(({ edge, key, mid, answers }) => {
          const from = names.get(edge.from_session_id) ?? "A member";
          const words = edgeWords(edge, from, names.get(edge.to_session_id) ?? "a member");
          return (
            <div
              key={key}
              className="absolute left-0 top-0 z-10 flex flex-col items-center gap-1"
              // Below 100 %: undo the canvas's scale, so it reads at screen size.
              style={{ transform: `translate(${mid.x}px, ${mid.y}px)${compact ? ` scale(${unit})` : ""} translate(-50%, -50%)` }}
              data-testid="edge-label"
              data-edge={key}
            >
              {(selected === key || hover === key) && (
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
              {answers && (compact ? <EdgeBadge edge={edge} about={words.aria} /> : <PendingChip edge={edge} compact={false} />)}
            </div>
          );
        })}
    </>
  );
});

/** Below 100 %: what waits on an edge as a small badge ("1 waiting",
 * "Paused"), its answers a click away in a small panel under it (the
 * full card's chip, at screen size). Esc or a click elsewhere closes it. */
function EdgeBadge({ edge, about }: { edge: CanvasEdgeInfo; about: string }): JSX.Element | null {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement | null>(null);
  const button = useRef<HTMLButtonElement | null>(null);
  const words = pendingWords(edge);
  useEffect(() => {
    if (!open) return;
    box.current?.querySelector<HTMLButtonElement>("[data-testid='edge-pending'] button")?.focus();
    const away = (event: PointerEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", away);
    return () => window.removeEventListener("pointerdown", away);
  }, [open]);
  if (!words) return null;
  return (
    <span
      ref={box}
      className="relative flex flex-col items-center gap-1"
      onKeyDown={(event) => {
        if (event.key !== "Escape" || !open) return;
        event.preventDefault();
        event.stopPropagation();
        setOpen(false);
        button.current?.focus();
      }}
    >
      <button
        ref={button}
        type="button"
        aria-expanded={open}
        // Its name starts with what it shows (axe: label-content-name-mismatch).
        aria-label={`${words.short}: ${words.label}, ${about}`}
        title={words.label}
        onClick={(event) => {
          event.stopPropagation();
          setOpen((now) => !now);
        }}
        className="flex items-center gap-1.5 whitespace-nowrap rounded-full border border-warning/60 bg-surface px-2 py-0.5 text-label font-medium text-warning shadow-card outline-none hover:bg-warning-soft focus-visible:ring-2 focus-visible:ring-accent/60"
        data-testid="edge-badge"
      >
        <span aria-hidden="true" className="size-1.5 rounded-full bg-warning" />
        {words.short}
      </button>
      {open && (
        <span className="absolute top-full z-20 mt-1.5" data-testid="edge-badge-panel">
          <PendingChip edge={edge} />
        </span>
      )}
    </span>
  );
}

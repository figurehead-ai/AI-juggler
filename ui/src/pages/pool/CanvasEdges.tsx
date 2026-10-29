/**
 * The pool canvas's edges: messages and task hand-offs from one member to
 * another over the last week, one arrow a direction, thicker the more
 * there were. New activity sends a dot along its edge (a brief glow
 * instead when you'd rather have less motion). What waits on an edge
 * sits on it with its answer: Deliver for a message waiting for a session
 * run here; Let it continue or Don't deliver for a chain at the hop limit.
 */
import { memo, useState, type JSX } from "react";

import { errorText } from "../../lib/bridge";
import { messagesAnswer, messagesDeliver } from "../../lib/ipc/messages";
import type { CanvasEdgeInfo, PendingInfo } from "../../lib/ipc/pool-canvas";
import { edgeCurve, type Point } from "./layout";
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

/** The answer an edge carries in place (the first that waits). */
export function PendingChip({ edge, compact = false }: { edge: CanvasEdgeInfo; compact?: boolean }): JSX.Element | null {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const actionable = edge.pending.filter((p) => p.kind !== "asked");
  const first = actionable[0];
  // One above the pool's level is answered on its sender's node.
  if (!first) return null;
  const hop = first.kind === "hop_limit";
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
  const label = hop ? "Paused at the hop limit" : actionable.length === 1 ? "1 message waiting" : `${actionable.length} messages waiting`;
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

/** Labels near one another move apart (down), so two edges between close
 * nodes never put their answers on top of each other. */
function apart<T extends { mid: Point }>(labels: T[]): T[] {
  const placed: Point[] = [];
  return labels.map((label) => {
    let at = label.mid;
    while (placed.some((other) => Math.abs(other.x - at.x) < 150 && Math.abs(other.y - at.y) < 58)) at = { x: at.x, y: at.y + 58 };
    placed.push(at);
    return { ...label, mid: at };
  });
}

type Drawn = { edge: CanvasEdgeInfo; key: string; d: string; mid: Point; width: number; waits: boolean; tasksOnly: boolean };

export const CanvasEdges = memo(function CanvasEdges({
  edges,
  points,
  names,
  selected,
  pulses,
  reducedMotion,
  zoomedOut,
  onSelect,
}: {
  edges: CanvasEdgeInfo[];
  points: ReadonlyMap<string, Point>;
  names: ReadonlyMap<string, string>;
  selected: string | null;
  pulses: ReadonlyMap<string, number>;
  reducedMotion: boolean;
  zoomedOut: boolean;
  onSelect: (key: string) => void;
}): JSX.Element {
  const [hover, setHover] = useState<string | null>(null);
  const keys = new Set(edges.map((edge) => edgeKey(edge.from_session_id, edge.to_session_id)));
  const drawn: Drawn[] = [];
  for (const edge of edges) {
    const from = points.get(edge.from_session_id);
    const to = points.get(edge.to_session_id);
    if (!from || !to) continue;
    const key = edgeKey(edge.from_session_id, edge.to_session_id);
    const curve = edgeCurve(from, to, keys.has(edgeKey(edge.to_session_id, edge.from_session_id)));
    const total = edge.messages + edge.tasks;
    drawn.push({ edge, key, d: curve.d, mid: curve.mid, width: Math.min(4, 1.5 + Math.log2(1 + total) * 0.6), waits: edge.pending.length > 0, tasksOnly: edge.messages === 0 && edge.tasks > 0 });
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
      {!zoomedOut &&
        apart(drawn.filter(({ key, waits }) => waits || selected === key || hover === key)).map(({ edge, key, mid, waits }) => {
          const words = edgeWords(edge, names.get(edge.from_session_id) ?? "A member", names.get(edge.to_session_id) ?? "a member");
          return (
            <div key={key} className="absolute left-0 top-0 z-10 flex flex-col items-center gap-1" style={{ transform: `translate(${mid.x}px, ${mid.y}px) translate(-50%, -50%)` }} data-testid="edge-label" data-edge={key}>
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
              {waits && <PendingChip edge={edge} compact={false} />}
            </div>
          );
        })}
    </>
  );
});

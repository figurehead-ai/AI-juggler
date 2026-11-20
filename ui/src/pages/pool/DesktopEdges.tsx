/**
 * Round 7 #7 (aij-test run 3): Lab and the aij-test session messaged each
 * other, and the diagram drew no line between them, so the line's own
 * talk rule couldn't be set and the driver blocked Member → Member for the
 * whole pool. A desktop session sits in the column (under its App, or as
 * its own card), not on the board; its lines are drawn the way the
 * column's App cards reach their sessions (RuleNodes): from the row's
 * left edge to the other end, measured after each draw. Each line has its
 * count (and its own rule) on a label that selects it, as a node's line
 * does on the board; the inspector shows it, with its talk rule.
 */
import { useLayoutEffect, useState, type JSX, type RefObject } from "react";

import type { CanvasEdgeInfo } from "../../lib/ipc/pool-canvas";
import { edgeWords, type LineRules } from "./CanvasEdges";
import type { Selection } from "./Canvas";
import { edgeKey } from "./useCanvas";

type Box = { x: number; y: number; w: number; h: number };
type End = { x: number; y: number; row: boolean };
export type DesktopLine = { key: string; d: string; label: { x: number; y: number }; count: string; aria: string; on: boolean };

/** A desktop session's row in the column, by its member ref. */
const rowOf = (column: HTMLElement, ref: string) => column.querySelector<HTMLElement>(`[data-desktop-ref="${CSS.escape(ref)}"]`);

/** The lines with a desktop session at one end at least (a line between
 * two board nodes is the board's own, CanvasEdges): from each end's place
 * (`endOf`: a row's left edge, a node's right edge), its count in words. */
export function desktopLines(input: { edges: readonly CanvasEdgeInfo[]; endOf: (id: string) => End | null; names: ReadonlyMap<string, string>; lineRules?: LineRules; selection: Selection }): DesktopLine[] {
  const { edges, endOf, names, lineRules, selection } = input;
  const out: DesktopLine[] = [];
  for (const edge of edges) {
    const from = endOf(edge.from_session_id);
    const to = endOf(edge.to_session_id);
    if (!from || !to || (!from.row && !to.row)) continue;
    // Both ways between one pair: a little apart.
    const back = edges.some((other) => other.from_session_id === edge.to_session_id && other.to_session_id === edge.from_session_id);
    const lift = back ? (edge.from_session_id < edge.to_session_id ? -5 : 5) : 0;
    const [a, b] = [{ x: from.x, y: from.y + lift }, { x: to.x, y: to.y + lift }];
    // Out to the left of the column when both ends are rows in it.
    const bend = from.row && to.row ? Math.min(a.x, b.x) - 70 : null;
    const mid = (a.x + b.x) / 2;
    const d = bend === null ? `M${a.x},${a.y} C${mid},${a.y} ${mid},${b.y} ${b.x},${b.y}` : `M${a.x},${a.y} C${bend},${a.y} ${bend},${b.y} ${b.x},${b.y}`;
    const key = edgeKey(edge.from_session_id, edge.to_session_id);
    const words = edgeWords(edge, names.get(edge.from_session_id) ?? "A member", names.get(edge.to_session_id) ?? "a member", lineRules?.get(key));
    // Each way's label on its own side of the pair, never on the other's.
    const apart = Math.sign(lift) * 12;
    out.push({ key, d, label: { x: bend === null ? mid : bend + 18, y: (a.y + b.y) / 2 + apart }, count: words.count, aria: words.aria, on: selection.kind === "edge" && selection.key === key });
  }
  return out;
}

export function DesktopEdges({
  column,
  edges,
  placeOf,
  names,
  lineRules,
  selection,
  onSelect,
}: {
  column: RefObject<HTMLDivElement | null>;
  edges: readonly CanvasEdgeInfo[];
  placeOf?: (id: string) => Box | null;
  names: ReadonlyMap<string, string>;
  lineRules?: LineRules;
  selection: Selection;
  onSelect: (selection: Selection) => void;
}): JSX.Element | null {
  const [drawn, setDrawn] = useState<DesktopLine[]>([]);
  // Measured after the frame that draws them: on a pan or zoom (a new
  // placeOf), new lines, the column's scroll and its rows arriving.
  useLayoutEffect(() => {
    const box = column.current;
    if (!box) return undefined;
    let frame = 0;
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const room = box.offsetParent?.getBoundingClientRect();
        if (!room) return;
        const endOf = (id: string): End | null => {
          const row = rowOf(box, id);
          if (row) {
            const at = row.getBoundingClientRect();
            return { x: at.left - room.left, y: at.top - room.top + at.height / 2, row: true };
          }
          const at = placeOf?.(id);
          return at ? { x: at.x + at.w, y: at.y + at.h / 2, row: false } : null;
        };
        const next = desktopLines({ edges, endOf, names, lineRules, selection });
        setDrawn((was) => (JSON.stringify(was) === JSON.stringify(next) ? was : next));
      });
    };
    measure();
    box.addEventListener("scroll", measure);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(box);
    return () => {
      cancelAnimationFrame(frame);
      box.removeEventListener("scroll", measure);
      observer?.disconnect();
    };
  }, [column, edges, placeOf, names, lineRules, selection]);
  if (drawn.length === 0) return null;
  return (
    <>
      <svg aria-hidden="true" className="pointer-events-none absolute inset-0 z-[6] h-full w-full" data-testid="canvas-desktop-lines">
        <defs>
          <marker id="desktop-line-end" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
            <path d="M0,0 L8,4 L0,8 z" className="fill-muted" />
          </marker>
        </defs>
        {drawn.map((line) => (
          <path key={line.key} d={line.d} fill="none" markerEnd="url(#desktop-line-end)" strokeWidth={line.on ? 2.25 : 1.5} className={line.on ? "stroke-accent" : "stroke-muted/70"} data-edge-key={line.key} />
        ))}
      </svg>
      {drawn.map((line) => (
        <button
          key={line.key}
          type="button"
          onClick={() => onSelect({ kind: "edge", key: line.key })}
          aria-label={line.aria}
          aria-pressed={line.on}
          style={{ left: line.label.x, top: line.label.y }}
          className={"pointer-events-auto absolute z-[11] max-w-[7.5rem] -translate-x-1/2 -translate-y-1/2 rounded-lg border px-2 py-0.5 text-center text-label font-medium leading-tight outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (line.on ? "border-accent bg-accent-soft text-accent-hi" : "border-hairline bg-surface text-ink-2 hover:bg-surface-2")}
          data-testid="canvas-desktop-line"
          data-edge={line.key}
        >
          {line.count}
        </button>
      ))}
    </>
  );
}

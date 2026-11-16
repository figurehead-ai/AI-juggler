/**
 * A panel's edge, to resize it by dragging or from the keyboard (WAI-ARIA's
 * window splitter): Tab to it, the arrows by 8 px, with ⇧ by 32, Home the
 * narrowest, End the widest; a screen reader hears its width; each change
 * is kept per viewer (debounced, and when the window goes). One component
 * for every resizable edge: the rail's (AJ-E2909) and the pool inspector's
 * (AJ-E3011 round 5, under D-55). `side` is where the edge is on its
 * panel: a right edge widens to the right (the rail), a left edge to the
 * left (the inspector).
 */
import { useCallback, useEffect, useRef, useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

export interface SplitterEdge {
  min: number;
  max: number;
  initial: number;
  /** Where its width is kept (localStorage). */
  store: string;
  /** Its accessible name ("Resize the list"). */
  label: string;
  side: "right" | "left";
  testId: string;
}

const STEP = 8;
const BIG_STEP = 32;

const clampTo = (edge: SplitterEdge, n: number) => Math.min(edge.max, Math.max(edge.min, n));

/** The stored width; nothing stored (or junk) is the default (`Number(null)`
 * is 0, which clamped every install to the minimum). */
export function readEdgeWidth(edge: SplitterEdge): number {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(edge.store);
  } catch {
    stored = null;
  }
  const raw = stored === null || stored.trim() === "" ? Number.NaN : Number(stored);
  if (!Number.isFinite(raw)) return edge.initial;
  return clampTo(edge, raw);
}

function keep(edge: SplitterEdge, width: number): void {
  try {
    localStorage.setItem(edge.store, String(width));
  } catch {
    // Kept for this visit only.
  }
}

/** The width a key moves `width` to, or null for a key it ignores. */
export function edgeWidthForKey(edge: SplitterEdge, width: number, key: string, shift: boolean): number | null {
  const step = shift ? BIG_STEP : STEP;
  const grow = edge.side === "right" ? "ArrowRight" : "ArrowLeft";
  const shrink = edge.side === "right" ? "ArrowLeft" : "ArrowRight";
  switch (key) {
    case grow:
      return clampTo(edge, width + step);
    case shrink:
      return clampTo(edge, width - step);
    case "Home":
      return edge.min;
    case "End":
      return edge.max;
    default:
      return null;
  }
}

export function Splitter({
  edge,
  width,
  onWidth,
  onDragging,
  onCycleDensity,
  controls,
}: {
  edge: SplitterEdge;
  width: number;
  onWidth: (width: number) => void;
  onDragging?: (dragging: boolean) => void;
  onCycleDensity?: () => void;
  /** The id of what it resizes. */
  controls: string;
}): JSX.Element {
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ startX: number; startWidth: number } | null>(null);
  const latest = useRef(width);
  useEffect(() => {
    latest.current = width;
  }, [width]);
  const save = useRef<number | null>(null);

  const keepSoon = useCallback(
    (next: number) => {
      if (save.current !== null) window.clearTimeout(save.current);
      save.current = window.setTimeout(() => {
        save.current = null;
        keep(edge, next);
      }, 200);
    },
    [edge],
  );
  // A change still waiting is kept when the panel goes and when the window
  // does (a quit or reload inside the 200 ms would lose it otherwise).
  useEffect(() => {
    const flush = () => {
      if (save.current !== null) {
        window.clearTimeout(save.current);
        save.current = null;
        keep(edge, latest.current);
      }
    };
    window.addEventListener("pagehide", flush);
    return () => {
      window.removeEventListener("pagehide", flush);
      flush();
    };
  }, [edge]);

  const begin = (event: PointerEvent) => {
    event.preventDefault();
    drag.current = { startX: event.clientX, startWidth: width };
    setDragging(true);
    onDragging?.(true);
  };
  useEffect(() => {
    if (!dragging) return () => undefined;
    const move = (event: globalThis.PointerEvent) => {
      const state = drag.current;
      const moved = event.clientX - (state?.startX ?? 0);
      if (state) onWidth(clampTo(edge, state.startWidth + (edge.side === "right" ? moved : -moved)));
    };
    const up = () => {
      setDragging(false);
      onDragging?.(false);
      drag.current = null;
      keep(edge, latest.current);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [dragging, onWidth, onDragging, edge]);

  const onKeyDown = (event: KeyboardEvent) => {
    const next = edgeWidthForKey(edge, width, event.key, event.shiftKey);
    if (next === null) return;
    event.preventDefault();
    onWidth(next);
    keepSoon(next);
  };
  const right = edge.side === "right";
  return (
    <div
      onPointerDown={begin}
      onDoubleClick={onCycleDensity}
      onKeyDown={onKeyDown}
      tabIndex={0}
      // eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- a window splitter (WAI-ARIA) is a focusable separator with a grip inside; <hr> holds neither
      role="separator"
      aria-orientation="vertical"
      aria-label={edge.label}
      aria-controls={controls}
      aria-valuenow={width}
      aria-valuemin={edge.min}
      aria-valuemax={edge.max}
      aria-valuetext={`${width} pixels wide`}
      data-testid={edge.testId}
      // Says the edge moves (design review #41): a tooltip, and a grip on
      // hover and focus.
      title={onCycleDensity ? "Drag, or ← →. Double-click for the list's density" : "Drag, or ← →"}
      className={"group/split absolute top-0 z-20 h-full w-1.5 cursor-col-resize outline-none " + (right ? "right-0 " : "left-0 ") + (dragging ? "" : "transition-colors")}
    >
      <div
        className={
          (right ? "ml-auto " : "mr-auto ") +
          "h-full w-px group-focus-visible/split:w-0.5 group-focus-visible/split:bg-accent " +
          (dragging ? "bg-accent/70" : "bg-transparent group-hover/split:bg-accent/50")
        }
      />
      <span
        aria-hidden="true"
        className={
          "pointer-events-none absolute top-1/2 flex h-8 w-2 -translate-y-1/2 flex-col items-center justify-center gap-[3px] rounded-full border border-hairline bg-surface opacity-0 shadow-card transition-opacity group-hover/split:opacity-100 group-focus-visible/split:opacity-100 " +
          (right ? "right-[-3px]" : "left-[-3px]")
        }
        data-testid={`${edge.testId}-grip`}
      >
        <span className="size-0.5 rounded-full bg-muted" />
        <span className="size-0.5 rounded-full bg-muted" />
        <span className="size-0.5 rounded-full bg-muted" />
      </span>
    </div>
  );
}

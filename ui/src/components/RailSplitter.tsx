/**
 * AJ-E2909: the list's edge, to resize it by dragging or from the
 * keyboard (WAI-ARIA's window splitter): Tab to it, ← and → by 8 px,
 * ⇧← and ⇧→ by 32, Home the narrowest, End the widest. A screen reader
 * hears its width. Each change is kept as a drag's is (debounced).
 * Double-click still cycles the density (its keyboard twin is ⌃D).
 */
import { useCallback, useEffect, useRef, useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

export const WIDTH_MIN = 208;
export const WIDTH_MAX = 360;
export const WIDTH_DEFAULT = 264;
export const STORE_WIDTH = "aij.rail.width";

const STEP = 8;
const BIG_STEP = 32;

const clamp = (n: number) => Math.min(WIDTH_MAX, Math.max(WIDTH_MIN, n));

/** The rail's stored width; nothing stored (or junk) is the default —
 * `Number(null)` is 0, which clamped every install to the minimum. */
export function readWidth(): number {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORE_WIDTH);
  } catch {
    stored = null;
  }
  const raw = stored === null || stored.trim() === "" ? Number.NaN : Number(stored);
  if (!Number.isFinite(raw)) return WIDTH_DEFAULT;
  return clamp(raw);
}

function keep(width: number): void {
  try {
    localStorage.setItem(STORE_WIDTH, String(width));
  } catch {
    // Kept for this visit only.
  }
}

/** The width a key moves `width` to, or null for a key it ignores. */
export function widthForKey(width: number, key: string, shift: boolean): number | null {
  const step = shift ? BIG_STEP : STEP;
  switch (key) {
    case "ArrowLeft":
      return clamp(width - step);
    case "ArrowRight":
      return clamp(width + step);
    case "Home":
      return WIDTH_MIN;
    case "End":
      return WIDTH_MAX;
    default:
      return null;
  }
}

export function RailSplitter({
  width,
  onWidth,
  onDragging,
  onCycleDensity,
  controls,
}: {
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
  latest.current = width;
  const save = useRef<number | null>(null);

  const keepSoon = useCallback((next: number) => {
    if (save.current !== null) window.clearTimeout(save.current);
    save.current = window.setTimeout(() => {
      save.current = null;
      keep(next);
    }, 200);
  }, []);
  // A change still waiting is kept when the list goes and when the window
  // does (a quit or reload inside the 200 ms would lose it otherwise).
  useEffect(() => {
    const flush = () => {
      if (save.current !== null) {
        window.clearTimeout(save.current);
        save.current = null;
        keep(latest.current);
      }
    };
    window.addEventListener("pagehide", flush);
    return () => {
      window.removeEventListener("pagehide", flush);
      flush();
    };
  }, []);

  const begin = (event: PointerEvent) => {
    event.preventDefault();
    drag.current = { startX: event.clientX, startWidth: width };
    setDragging(true);
    onDragging?.(true);
  };
  useEffect(() => {
    if (!dragging) return;
    const move = (event: globalThis.PointerEvent) => {
      const state = drag.current;
      if (state) onWidth(clamp(state.startWidth + (event.clientX - state.startX)));
    };
    const up = () => {
      setDragging(false);
      onDragging?.(false);
      drag.current = null;
      keep(latest.current);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [dragging, onWidth, onDragging]);

  const onKeyDown = (event: KeyboardEvent) => {
    const next = widthForKey(width, event.key, event.shiftKey);
    if (next === null) return;
    event.preventDefault();
    onWidth(next);
    keepSoon(next);
  };

  return (
    <div
      onPointerDown={begin}
      onDoubleClick={onCycleDensity}
      onKeyDown={onKeyDown}
      tabIndex={0}
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the list"
      aria-controls={controls}
      aria-valuenow={width}
      aria-valuemin={WIDTH_MIN}
      aria-valuemax={WIDTH_MAX}
      aria-valuetext={`${width} pixels wide`}
      data-testid="rail-splitter"
      // Says the edge moves (design review #41): a tooltip, and a grip on
      // hover and focus.
      title={onCycleDensity ? "Drag, or ← →. Double-click for the list's density" : "Drag, or ← →"}
      className={"group/split absolute right-0 top-0 z-20 h-full w-1.5 cursor-col-resize outline-none " + (dragging ? "" : "transition-colors")}
    >
      <div
        className={
          "ml-auto h-full w-px group-focus-visible/split:w-0.5 group-focus-visible/split:bg-accent " +
          (dragging ? "bg-accent/70" : "bg-transparent group-hover/split:bg-accent/50")
        }
      />
      <span
        aria-hidden="true"
        className="pointer-events-none absolute right-[-3px] top-1/2 flex h-8 w-2 -translate-y-1/2 flex-col items-center justify-center gap-[3px] rounded-full border border-hairline bg-surface opacity-0 shadow-card transition-opacity group-hover/split:opacity-100 group-focus-visible/split:opacity-100"
        data-testid="rail-splitter-grip"
      >
        <span className="size-0.5 rounded-full bg-muted" />
        <span className="size-0.5 rounded-full bg-muted" />
        <span className="size-0.5 rounded-full bg-muted" />
      </span>
    </div>
  );
}

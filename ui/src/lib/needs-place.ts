/**
 * D-103.6 — Home's Needs you holds its place while the needs load, sized
 * from the last look, so the same needs arriving move nothing: what it
 * showed last time (how many rows and how tall) is kept for this data
 * folder (lib/local-store), as the app keeps other last answers to paint
 * them at once. The same for the heading's line of "more things".
 */
import { useLayoutEffect, useRef, type RefObject } from "react";
import { keep, readKept } from "./local-store";

/** What a place showed last time: its rows (0: nothing) and its height. */
export type KeptShape = { rows: number; height: number };

/** Home's Needs you card. */
export const NEEDS_PLACE = "aij.home.needs-place";
/** The heading's line of "more things" (D-96 B). */
export const EXTRAS_PLACE = "aij.home.extras-place";

/** The shape kept under `key` (`null`: never shown here, a first run). */
export function keptShape(key: string): KeptShape | null {
  try {
    const kept: unknown = JSON.parse(readKept(key) ?? "null");
    if (typeof kept !== "object" || kept === null) return null;
    const rows: unknown = Reflect.get(kept, "rows");
    const height: unknown = Reflect.get(kept, "height");
    return typeof rows === "number" && typeof height === "number" ? { rows, height } : null;
  } catch {
    return null;
  }
}

function keepShape(key: string, shape: KeptShape): void {
  const text = JSON.stringify(shape);
  if (readKept(key) !== text) keep(key, text);
}

/** Keeps what the place shows once its rows are known (`rows`: 0 for
 * nothing), and its height as it changes; the ref goes on the shown box. */
export function useKeepShape<T extends HTMLElement>(key: string, known: boolean, rows: number): RefObject<T | null> {
  const box = useRef<T | null>(null);
  useLayoutEffect(() => {
    if (!known) return undefined;
    const save = () => keepShape(key, { rows, height: rows > 0 ? Math.round(box.current?.getBoundingClientRect().height ?? 0) : 0 });
    save();
    const node = box.current;
    if (!node || typeof ResizeObserver === "undefined") return undefined;
    const watch = new ResizeObserver(save);
    watch.observe(node);
    return () => watch.disconnect();
  }, [key, known, rows]);
  return box;
}

/** Few needs (two at most): "Pick up where you left off" moves up into
 * the left column (the 0.4 design review, 39). Before they're known, as
 * many as last time, so it doesn't change columns when they arrive. */
export function fewNeedsNow(known: boolean, count: number): boolean {
  return (known ? count : (keptShape(NEEDS_PLACE)?.rows ?? 0)) <= 2;
}

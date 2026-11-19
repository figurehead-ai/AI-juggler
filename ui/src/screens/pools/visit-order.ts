/**
 * F-rc3g-14: the Pools cards keep their places while you're on the screen.
 * Their order (the pools that ask you first, then the latest active: the
 * suggested ones, then all) is read when the screen opens; from your first
 * move, or a moment after the cards show, it holds for the visit. A card
 * never moves under a click: the rc3g walk aimed at pool A and opened B,
 * the two swapped by their sessions' activity. A pool made meanwhile goes
 * at the end; one gone leaves its place.
 */
import { useEffect, useRef, useState } from "react";

/** How long the order may still follow what's read (needs, sessions)
 * after the cards first show, when you haven't moved yet. */
export const SETTLE_MS = 1000;

/** What you do that holds the order: any move on the screen. */
const MOVES = ["pointerdown", "pointermove", "keydown", "wheel", "focusin"] as const;

/** `live` in the order this visit holds (`kept`), the ones it doesn't
 * hold yet at the end in their own order; `live` as it is before. */
export function visitOrder(kept: readonly string[] | null, live: readonly string[]): string[] {
  if (!kept) return [...live];
  const now = new Set(live);
  const held = new Set(kept);
  return [...kept.filter((id) => now.has(id)), ...live.filter((id) => !held.has(id))];
}

/** `live` (ordered as it suggests) held in place for the visit once
 * `ready` (the cards show) and you move, or after [`SETTLE_MS`]. */
export function useVisitOrder<T extends { id: string }>(live: readonly T[], ready: boolean): T[] {
  const [kept, setKept] = useState<string[] | null>(null);
  const shown = useRef<readonly string[]>([]);
  const ids = visitOrder(kept, live.map((item) => item.id));
  useEffect(() => {
    shown.current = ids;
  });
  const holding = ready && kept === null;
  useEffect(() => {
    const hold = () => setKept((was) => was ?? [...shown.current]);
    const timer = holding ? window.setTimeout(hold, SETTLE_MS) : 0;
    if (holding) for (const move of MOVES) window.addEventListener(move, hold, { capture: true, passive: true });
    return () => {
      window.clearTimeout(timer);
      for (const move of MOVES) window.removeEventListener(move, hold, { capture: true });
    };
  }, [holding]);
  const byId = new Map(live.map((item) => [item.id, item]));
  return ids.flatMap((id) => {
    const item = byId.get(id);
    return item ? [item] : [];
  });
}

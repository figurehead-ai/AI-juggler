/**
 * Which way a pool's page shows it: the canvas, or the List view (the
 * same members and edges as rows). A choice the user made is kept per
 * pool on this Mac and always wins; until then, a window under 1100 px
 * starts in the List view (the canvas and its inspector need the room)
 * and follows the window as it's resized — except a pool with no members
 * yet, which opens on the canvas at any width: setting one up is taught
 * there (AJ-E3008, the P10 walk).
 */
import { useCallback, useEffect, useState } from "react";

import type { Mode } from "./PoolHeader";

/** Below this window width a pool opens in the List view, unless chosen. */
export const LIST_BELOW = 1100;

const key = (poolId: string) => `aij.pool.mode.${poolId}`;

export function chosenMode(poolId: string): Mode | null {
  try {
    const saved = localStorage.getItem(key(poolId));
    return saved === "canvas" || saved === "list" ? saved : null;
  } catch {
    return null;
  }
}

export function defaultMode(width: number, empty = false): Mode {
  return empty || width >= LIST_BELOW ? "canvas" : "list";
}

/** The pool's mode, and the setter that remembers the user's choice. */
export function usePoolMode(poolId: string, empty = false): [Mode, (mode: Mode) => void] {
  // Empty when opened: the canvas holds for the visit, as members join
  // while it's being set up (never flipping to rows mid-way).
  const [openedEmpty] = useState(empty);
  const [mode, setState] = useState<Mode>(() => chosenMode(poolId) ?? defaultMode(window.innerWidth, openedEmpty));
  useEffect(() => {
    if (chosenMode(poolId)) return;
    const follow = () => {
      if (!chosenMode(poolId)) setState(defaultMode(window.innerWidth, openedEmpty));
    };
    window.addEventListener("resize", follow);
    return () => window.removeEventListener("resize", follow);
  }, [poolId, openedEmpty]);
  const choose = useCallback(
    (next: Mode) => {
      try {
        localStorage.setItem(key(poolId), next);
      } catch {
        // Storage unavailable: this visit only.
      }
      setState(next);
    },
    [poolId],
  );
  return [mode, choose];
}

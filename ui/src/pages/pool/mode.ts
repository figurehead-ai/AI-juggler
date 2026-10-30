/**
 * Which way a pool's page shows it: the canvas, or the List view (the
 * same members and edges as rows). A choice the user made is kept per
 * pool on this Mac and always wins; until then, a window under 1100 px
 * starts in the List view (the canvas and its inspector need the room)
 * and follows the window as it's resized.
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

export function defaultMode(width: number): Mode {
  return width < LIST_BELOW ? "list" : "canvas";
}

/** The pool's mode, and the setter that remembers the user's choice. */
export function usePoolMode(poolId: string): [Mode, (mode: Mode) => void] {
  const [mode, setState] = useState<Mode>(() => chosenMode(poolId) ?? defaultMode(window.innerWidth));
  useEffect(() => {
    if (chosenMode(poolId)) return;
    const follow = () => {
      if (!chosenMode(poolId)) setState(defaultMode(window.innerWidth));
    };
    window.addEventListener("resize", follow);
    return () => window.removeEventListener("resize", follow);
  }, [poolId]);
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

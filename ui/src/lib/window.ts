/**
 * The window's width, as the shell needs it (moved out of App.tsx).
 */
import { useEffect, useState } from "react";

/** D-25: below this width the rail folds to icons by itself. */
export const NARROW_RAIL_PX = 900;

/** True while the window is narrower than `px` (follows resizes). */
export function useNarrowWindow(px: number): boolean {
  const query = `(max-width: ${px - 1}px)`;
  const read = () => typeof window.matchMedia === "function" && window.matchMedia(query).matches;
  const [narrow, setNarrow] = useState(read);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const list = window.matchMedia(query);
    const follow = () => setNarrow(list.matches);
    follow();
    list.addEventListener("change", follow);
    return () => list.removeEventListener("change", follow);
  }, [query]);
  return narrow;
}

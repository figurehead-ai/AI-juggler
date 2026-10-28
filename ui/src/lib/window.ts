/**
 * The window's width, as the shell needs it (moved out of App.tsx).
 */
import { useEffect, useState } from "react";

/** D-25: below this width the rail folds to icons by itself. The screen
 * spec's breakpoint (deep/11 §3, 1100 px): the 0.2 walk (F14) found the
 * rail at full width in the 1024×680 minimum window. */
export const NARROW_RAIL_PX = 1100;

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

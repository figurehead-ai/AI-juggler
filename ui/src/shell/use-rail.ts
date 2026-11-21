/**
 * The shell's clock and the rail's width (AJ-E2018: moved out of App).
 */
import { useClock } from "../lib/clock";
import { useCallback, useEffect, useState } from "react";
import { toggledDensity, type Density } from "../lib/density";
import { useStoredDensity } from "../lib/use-density";
import { NARROW_RAIL_PX, useNarrowWindow } from "../lib/window";

/** Density (AJ-E403), stored and followed from Settings › Appearance, and
 * the rail's own width in a narrow window. */
export function useRailDensity() {
  const { density, cycleStoredDensity, setDensityAndStore } = useStoredDensity();
  // D-25: below 1100 px (lib/window.ts) the rail folds to its icon column by itself and
  // comes back when the window widens. In a narrow window ⌘B, the logo,
  // the density shortcut and a double-click on the rail only open or
  // close the list for now — they never write Settings › Appearance,
  // whatever is stored. Changing the density is for Settings and for a
  // wide window.
  const narrow = useNarrowWindow(NARROW_RAIL_PX);
  const [narrowOpen, setNarrowOpen] = useState(false);
  useEffect(() => {
    if (!narrow) setNarrowOpen(false);
  }, [narrow]);
  // Open in a narrow window: the stored density, or Comfortable when the
  // stored one is Minimal (the icons are what "closed" already shows).
  const railDensity: Density = narrow
    ? narrowOpen
      ? density === "minimal"
        ? "comfortable"
        : density
      : "minimal"
    : density;
  const toggleRail = useCallback(() => {
    if (narrow) setNarrowOpen((open) => !open);
    else setDensityAndStore(toggledDensity(density));
  }, [narrow, density, setDensityAndStore]);
  const cycleDensity = useCallback(() => {
    if (narrow) setNarrowOpen((open) => !open);
    else cycleStoredDensity();
  }, [narrow, cycleStoredDensity]);
  return { railDensity, toggleRail, cycleDensity };
}

/** The stale-data tick (AJ-E403 Req.5): the app's ONE shared clock
 * (`lib/clock`), so staleness captions and every relative time move on
 * the same tick, without any per-value timers. */
export function useNow(): number {
  return useClock();
}

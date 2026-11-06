/**
 * The window's density (AJ-E403), as App holds it: persisted in
 * `aij.density` (the legacy `aij.rail.collapsed` boolean migrates into
 * `minimal` on first read), following Settings › Appearance (D-25), and
 * reported with the theme as this week's display gauges (AJ-E1103).
 */
import { useCallback, useEffect, useState } from "react";
import { statsGauge } from "./ipc/usage-stats";
import { DENSITY_EVENT, nextDensity, readDensity, storeDensity, type Density } from "./density";

export function useStoredDensity(): {
  density: Density;
  /** The keyboard's cycle, stored. */
  cycleStoredDensity: () => void;
  setDensityAndStore: (next: Density) => void;
} {
  const [density, setDensity] = useState<Density>(() => readDensity());
  useEffect(() => {
    const follow = () => setDensity(readDensity());
    window.addEventListener(DENSITY_EVENT, follow);
    return () => window.removeEventListener(DENSITY_EVENT, follow);
  }, []);
  useEffect(() => {
    statsGauge("ui.density", density);
  }, [density]);
  useEffect(() => {
    const report = () => void import("../theme").then((theme) => statsGauge("ui.theme", theme.storedTheme()));
    report();
    window.addEventListener("aij:theme-changed", report);
    return () => window.removeEventListener("aij:theme-changed", report);
  }, []);
  const cycleStoredDensity = useCallback(() => {
    setDensity((current) => {
      const next = nextDensity(current);
      storeDensity(next);
      return next;
    });
    window.setTimeout(() => window.dispatchEvent(new Event(DENSITY_EVENT)), 0);
  }, []);
  const setDensityAndStore = useCallback((next: Density) => {
    setDensity(next);
    storeDensity(next);
    window.dispatchEvent(new Event(DENSITY_EVENT));
  }, []);
  return { density, cycleStoredDensity, setDensityAndStore };
}

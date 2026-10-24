/**
 * Tests of the project board (the rail's Project lens, AJ-E2104): the
 * View starts on Project for each test, and goes back to the defaults.
 */
import { afterEach, beforeEach } from "vitest";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";

export function useProjectLens(): void {
  beforeEach(() => {
    resetRailViewCache();
    setRailView({ ...DEFAULT_VIEW, group: "project" });
  });
  afterEach(() => {
    setRailView(DEFAULT_VIEW);
    resetRailViewCache();
  });
}

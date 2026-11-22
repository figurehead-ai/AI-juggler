/**
 * Tests of the project board (the rail's Project lens, AJ-E2104): the
 * View starts on Project for each test, and goes back to the defaults.
 * Needs you is known (none) as the backend's first read would have it:
 * the board holds its place until then (D-103.6); a test with needs hands
 * them over itself (test/needs-kit.tsx).
 */
import { afterEach, beforeEach } from "vitest";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetNoticeStandIn, standInNeeds } from "../lib/ipc/notices";

/** Each test on the Project lens (not a hook: a spec calls it at its top). */
export function projectLensEachTest(): void {
  beforeEach(() => {
    resetRailViewCache();
    setRailView({ ...DEFAULT_VIEW, group: "project" });
    resetNoticeStandIn();
    standInNeeds([]);
  });
  afterEach(() => {
    setRailView(DEFAULT_VIEW);
    resetRailViewCache();
  });
}

/** The kit's first name (its callers keep it). */
export function useProjectLens(): void {
  projectLensEachTest();
}

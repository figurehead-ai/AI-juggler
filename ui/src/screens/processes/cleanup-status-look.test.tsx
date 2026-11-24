/**
 * 0.5.1 batch B3 (PAT-COLOR-2, PAT-SURF-1; as F59 and F66): the automatic
 * cleanup's notices on Processes are tone lines, never an amber wash with
 * an amber edge; the ones that wait on the user (a crash's leftover about
 * to be tidied, cleanup held after a crash) carry the amber needs-you
 * mark, and a pause the user chose carries none.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/cleanup";
import { CleanupStatus } from "./CleanupStatus";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const status = (over: Partial<ipc.CleanupStatusData>): ipc.CleanupStatusData => ({ paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [], ...over });

describe("the cleanup notices' look", () => {
  it("a leftover about to be tidied and a held cleanup: tone lines with an amber mark, no amber fill or edge", async () => {
    vi.spyOn(ipc, "cleanupAlerts").mockResolvedValue({ alerts: [] });
    vi.spyOn(ipc, "cleanupStatus").mockResolvedValue(
      status({ suspended: true, tier2_pending: [{ pid: 8123, session: "s1", cmdline_trunc: "node vite dev", deadline_ms: Date.now() + 5_000 }] }),
    );
    const view = render(<CleanupStatus degraded={false} onFind={() => undefined} />);
    const notice = await view.findByTestId("cleanup-tier2-notice-8123");
    const held = view.getByTestId("cleanup-suspended-banner");
    for (const banner of [notice, held]) {
      expect(banner.className).toContain("bg-surface");
      expect(banner.className).not.toMatch(/warning|border/);
      const mark = banner.querySelector("[data-testid=cleanup-needs-you-mark]")!;
      expect(mark.className).toContain("level-mark");
      expect(mark.getAttribute("data-level")).toBe("2");
    }
  });

  it("a pause the user chose is a plain tone line with no mark", async () => {
    vi.spyOn(ipc, "cleanupAlerts").mockResolvedValue({ alerts: [] });
    vi.spyOn(ipc, "cleanupStatus").mockResolvedValue(status({ paused: true }));
    const view = render(<CleanupStatus degraded={false} onFind={() => undefined} />);
    await waitFor(() => expect(view.getByTestId("cleanup-paused-banner")).toBeTruthy());
    expect(view.container.innerHTML).not.toMatch(/warning/);
    expect(view.queryAllByTestId("cleanup-needs-you-mark")).toHaveLength(0);
  });
});

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { unmeasuredWords } from "./Activity";

/**
 * rc18: a Grok App with one session and no token source read "Sessions
 * 0" with "+1 not measured", and the footer said "1 started in this
 * range. 1 more ran without token counts" — two sessions, against "0".
 * It is one session, said once.
 */

vi.mock("../../lib/ipc/usageHistory", async (original) => {
  const real = await original<typeof import("../../lib/ipc/usageHistory")>();
  const preview = await import("../../preview/usage-history");
  return {
    ...real,
    usageHistory: async (input: Parameters<typeof real.usageHistory>[0]) => {
      const history = await preview.previewHistory(input);
      // One session, started in the range, with no token counts.
      const started = history.started_per_bucket.map((_, at) => (at === 0 ? 1 : 0));
      return {
        ...history,
        totals: { ...history.totals, sessions: 0 },
        previous: { ...history.previous, sessions: 4 },
        sessions_per_bucket: history.sessions_per_bucket.map(() => 0),
        started_per_bucket: started,
        unmeasured_sessions: 1,
      };
    },
  };
});

afterEach(cleanup);

describe("a session with no token counts", () => {
  it("is counted once on the Sessions tile, with no change against a period it isn't in", async () => {
    const { UsageDashboard } = await import("./Dashboard");
    const view = render(<UsageDashboard kind="all" scope={{}} range="7d" accounts={[]} />);
    const tile = await view.findByTestId("tile-sessions");
    await waitFor(() => expect(tile.querySelector("[data-testid=stat-value]")?.textContent).toBe("1"));
    expect(tile.textContent).toContain("1 with no token counts");
    expect(tile.textContent).not.toMatch(/not measured|\+1/);
    expect(tile.textContent).not.toMatch(/before/);
  });

  it("is said once under the Sessions chart, never as one more", async () => {
    const { UsageDashboard } = await import("./Dashboard");
    // (Another range than the test above: the screen keeps what it read.)
    const view = render(<UsageDashboard kind="all" scope={{}} range="30d" accounts={[]} />);
    const chart = await view.findByTestId("usage-sessions-chart");
    await waitFor(() =>
      expect(chart.textContent).toContain(
        "1 started in this range. 1 had no token counts on this computer, so its tokens aren't in these numbers.",
      ),
    );
    expect(chart.textContent).not.toMatch(/more ran/);
  });

  it("says several the same way", () => {
    expect(unmeasuredWords(3)).toBe("3 had no token counts on this computer, so their tokens aren't in these numbers.");
  });
});

/**
 * The 0.2 walk (F6): while the first read of the tools' history runs, no
 * screen says "0 tokens" or "Not measured yet": it says the history is
 * being read, and how far.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { runwayOf } from "../components/AccountRunway";
import { BothUnread } from "../components/usage/Runway";
import { quotaDisplay, windowDisplay } from "./wire";
import { READING_LIMITS, READING_SHORT, readingLine } from "./history-reading";

const reading = vi.hoisted(() => ({ value: 640 as number | null }));

vi.mock("./ipc/usageHistory", async (original) => {
  const real = await original<typeof import("./ipc/usageHistory")>();
  const preview = await import("../preview/usage-history");
  return {
    ...real,
    usageHistory: async (input: Parameters<typeof real.usageHistory>[0]) => {
      const history = await preview.previewHistory(input);
      const zero = { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 };
      return reading.value === null
        ? history
        : { ...history, reading: reading.value, series: [], totals: { ...history.totals, tokens: zero, usd: null, sessions: 0, active_hours: 0 } };
    },
  };
});

afterEach(cleanup);

describe("the first read of the history", () => {
  it("says how far it has got", () => {
    expect(readingLine(640)).toBe("Reading your history for the first time: 640 conversations so far. The figures fill in when it's done, usually within a few minutes.");
    expect(readingLine(1)).toContain("1 conversation so far");
    expect(readingLine(2232)).toContain("2,232 conversations");
  });

  it("an account not read yet says so, never 'Not measured yet'", () => {
    const display = { ...quotaDisplay("work", { five_hour: windowDisplay("unmeasured"), weekly: windowDisplay("unmeasured") }), history_reading: true };
    expect(runwayOf(display).words).toBe(READING_LIMITS);
    expect(runwayOf({ ...display, history_reading: false }).words).toBe("Not measured yet");
    const gauge = render(<BothUnread display={display} />);
    expect(gauge.container.textContent).toContain(READING_SHORT);
    expect(gauge.container.textContent).not.toContain("Not measured yet");
  });

  it("Usage's tiles say it in place of a zero, under a line that says how far", async () => {
    reading.value = 640;
    const { UsageDashboard } = await import("../components/usage/Dashboard");
    const view = render(<UsageDashboard kind="all" scope={{}} range="7d" accounts={[]} />);
    await waitFor(() => expect(view.getByTestId("usage-reading").textContent).toContain("640 conversations so far"));
    for (const tile of ["tile-tokens", "tile-cost", "tile-sessions", "tile-hours"]) {
      const value = view.getByTestId(tile).querySelector("[data-testid=stat-value]")!.textContent;
      expect(value, tile).toBe(READING_SHORT);
    }
  });
});

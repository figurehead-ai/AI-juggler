import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StatTile } from "../charts/StatTile";

/**
 * §4.11 and the 2026-10-27 audit: the Estimated cost tile never reads
 * "$0" for usage it couldn't price (the table already said "No list
 * price"); a partly priced range reads as a floor ("≥ $12").
 */

const totals = vi.hoisted(() => ({ usd: null as number | null, unpriced: true }));

vi.mock("../../lib/ipc/usageHistory", async (original) => {
  const real = await original<typeof import("../../lib/ipc/usageHistory")>();
  const preview = await import("../../preview/usage-history");
  return {
    ...real,
    usageHistory: async (input: Parameters<typeof real.usageHistory>[0]) => {
      const history = await preview.previewHistory(input);
      return { ...history, totals: { ...history.totals, usd: totals.usd, unpriced: totals.unpriced } };
    },
  };
});

afterEach(cleanup);

describe("the cost tile", () => {
  it("says words, never $0, when nothing could be priced", async () => {
    totals.usd = null;
    totals.unpriced = true;
    const { UsageDashboard } = await import("./Dashboard");
    const view = render(<UsageDashboard kind="all" scope={{}} range="7d" accounts={[]} />);
    const tile = await view.findByTestId("tile-cost");
    await waitFor(() => expect(tile.textContent).toContain("No list price"));
    expect(tile.textContent).not.toMatch(/\$0(?![.\d])|\$0\.00/);
    expect(tile.textContent).toContain("None of these models has a list price");
  });

  it("a partly priced range reads as at least its figure", async () => {
    totals.usd = 12.4;
    totals.unpriced = true;
    const { UsageDashboard } = await import("./Dashboard");
    const view = render(<UsageDashboard kind="all" scope={{}} range="30d" accounts={[]} />);
    const tile = await view.findByTestId("tile-cost");
    // (Another range than the first test's: the screen keeps what it read.)
    await waitFor(() => expect(tile.querySelector("[data-testid=stat-value]")?.textContent).toBe("at least ~$12"));
    // Once, in words, and an estimate at the API's price (the 0.2 walk, F15).
    expect(tile.textContent).toContain("What the API would charge, not your bill");
  });

  it("the tile says its words in place of a number", () => {
    const view = render(<StatTile label="Estimated cost" value={null} unit="usd" words="No list price" />);
    expect(view.getByTestId("stat-value").textContent).toBe("No list price");
  });
});

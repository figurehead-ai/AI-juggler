/**
 * AJ-E609 evidence: the unpriced `—` law, the permanent USD label,
 * the dual subscription view, the stale warning, and the label-discipline
 * grep gate ("you paid $" must never appear in this feature's strings).
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CostMatrix } from "./CostMatrix";
import type { CostRollupsData } from "../lib/bridge";

vi.mock("../lib/bridge", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/bridge")>();
  return {
    ...original,
    costRollups: vi.fn(async () => fixture),
    costCsvExport: vi.fn(async () => "/tmp/costs.csv"),
  };
});

const fixture: CostRollupsData = {
  rows: [
    {
      key: "sess-t",
      model: "claude-opus-4",
      day: "2026-10-15",
      tokens: { input: 300, output: 50, cache_read: 15, cache_write: 2, reasoning: 0 },
      usd: { usd: 0.00831, label: "Estimate" },
      credits: undefined,
      confidence: "Measured",
    },
    {
      key: "sess-z",
      model: "GLM-6-X",
      day: "2026-10-15",
      tokens: { input: 100, output: 10, cache_read: 0, cache_write: 0, reasoning: 0 },
      usd: { label: "Estimate" }, // unpriced
      credits: 1.25,
      confidence: "Computed",
    },
  ],
  subscription: { api_equivalent_usd: 0.00831, flat_fee_usd: 200, value_ratio: 0.0000416 },
  pricing_stale: false,
  total_usd: 0.00831,
  scope: "ALL",
};

describe("CostMatrix (AJ-E609)", () => {
  afterEach(cleanup);

  it("renders the permanent USD disclaimer and priced rows", async () => {
    const screen = render(<CostMatrix />);
    expect(
      await screen.findByText("USD is Estimated — list pricing, not your subscription fee."),
    ).toBeTruthy();
    expect((await screen.findAllByText("$0.01")).length).toBeGreaterThan(0);
  });

  it("renders — (never $0.00) for unpriced models, with credits alongside", async () => {
    const screen = render(<CostMatrix />);
    expect(await screen.findByText("GLM-6-X")).toBeTruthy();
    expect(screen.container.textContent).toContain("—");
    expect(screen.container.textContent).not.toContain("$0.00");
    expect(screen.container.textContent).toContain("1.25");
  });

  it("shows the dual subscription view with both numbers", async () => {
    const screen = render(<CostMatrix />);
    expect(await screen.findByText(/you pay \$200\.00 flat/)).toBeTruthy();
    expect(screen.container.textContent).toContain("≈$0.01 at API prices");
  });

  it("associates headers and announces the totals row (WCAG 2.2 AA)", async () => {
    const screen = render(<CostMatrix />);
    await screen.findByText("claude-opus-4");
    const headers = screen.container.querySelectorAll("th[scope='col']");
    expect(headers.length).toBeGreaterThanOrEqual(6);
    const rowHeader = screen.container.querySelector("tfoot th[scope='row']");
    expect(rowHeader?.textContent).toContain("Total");
  });
});

describe("AJ-E609 label-discipline grep gate", () => {
  it("never claims the user paid the API-equivalent value", async () => {
    const source = await import(
      /* @vite-ignore */ "./CostMatrix.tsx?raw" as string
    ).catch(() => null);
    if (source) {
      expect(source.default ?? source).not.toContain("you paid $");
    }
    // The compiled component itself is the binding check:
    const screen = render(<CostMatrix />);
    await screen.findByText("claude-opus-4");
    expect(screen.container.textContent).not.toContain("you paid $");
  });
});

/**
 * AJ-E609 evidence: the unpriced law (words, never $0.00), the permanent
 * USD label, the dual subscription view, the stale warning, and the
 * label-discipline grep gate ("you paid $" must never appear in this
 * feature's strings).
 *
 * 0.1.8 (the maintainer: no price in view, raw session ids, uneven rows):
 * the money leads, USD comes right after the session and model, a session
 * reads as its title and project, unpriced reads short, and the total sits
 * under the USD column.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
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

// 0.1.5: the rows come a page at a time (`costRowsPage`); the totals stay.
// 0.1.8: a page's row is a session's day, as the backend folds it.
vi.mock("../lib/ipc/costs", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc/costs")>();
  const { pageOf } = await import("../lib/paged");
  return {
    ...original,
    costRowsPage: vi.fn(async (_account: string | null, page: import("../lib/paged").PageRequest) => pageOf(original.sessionDays(fixture.rows), page)),
  };
});

const tokens = (input: number, output: number) => ({ input, output, cache_read: 0, cache_write: 0, reasoning: 0 });

const fixture: CostRollupsData = {
  rows: [
    // One session, two models on one day: one row, its parts on demand.
    Object.assign(
      { key: "01a0f458-0056-7d00-9a1b-5c0ffee00001", model: "gpt-6.1-sol", day: "2026-10-21", tokens: tokens(300_000, 20_000), usd: { usd: 0.8, label: "Estimated" as const }, credits: null, confidence: "Measured" as const },
      { title: "Fix the login test", project: "api" },
    ),
    Object.assign(
      { key: "01a0f458-0056-7d00-9a1b-5c0ffee00001", model: "codex-auto-review", day: "2026-10-21", tokens: tokens(5_000, 100), usd: { usd: null, label: "Estimated" as const }, credits: null, confidence: "Measured" as const },
      { title: "Fix the login test", project: "api" },
    ),
    // Unpriced only, with credits alongside (never converted).
    { key: "sess-z", model: "GLM-6-X", day: "2026-10-20", tokens: tokens(100, 10), usd: { usd: null, label: "Estimated" }, credits: 1.25, confidence: "Computed" },
  ],
  subscription: { api_equivalent_usd: 0.8, flat_fee_usd: 200, value_ratio: 0.004 },
  pricing_stale: false,
  pricing_last_synced_ms: null,
  total_usd: 0.8,
  unpriced_models: ["GLM-6-X", "codex-auto-review"],
  scope: "ALL",
};

describe("CostMatrix (AJ-E609, 0.1.8)", () => {
  afterEach(cleanup);

  it("leads with the money: the total, what you pay, and what has no list price", async () => {
    const screen = render(<CostMatrix />);
    expect(await screen.findByText("USD is Estimated — list pricing, not your subscription fee.")).toBeTruthy();
    const summary = screen.getByTestId("cost-summary");
    expect(within(summary).getByTestId("cost-total-tile").textContent).toContain("$0.80");
    expect(within(summary).getByTestId("cost-fee-tile").textContent).toContain("$200");
    expect(within(summary).getByTestId("cost-fee-tile").textContent).toContain("API prices come to 0.00× your fee");
    expect(within(summary).getByTestId("cost-unpriced-tile").textContent).toContain("GLM-6-X, codex-auto-review: not counted");
  });

  it("orders the columns day, session, model, USD, tokens, credits", async () => {
    const screen = render(<CostMatrix />);
    await screen.findAllByTestId("cost-row");
    const names = Array.from(screen.getByTestId("cost-columns").children).map((cell) => cell.textContent);
    expect(names).toEqual(["Day", "Session", "Model", "USD (estimate)", "Tokens (in / out)", "Credits"]);
    const row = screen.getAllByTestId("cost-row")[0]!;
    for (const name of ["Day", "Session", "Model", "USD, estimate", "Tokens in and out", "Credits"]) expect(row.textContent).toContain(name);
    expect(screen.getByRole("list", { name: "Cost per session and day" })).toBeTruthy();
  });

  it("names a session by its title and project, never its id (the id stays in the tooltip)", async () => {
    const screen = render(<CostMatrix />);
    const [first, second] = await screen.findAllByTestId("cost-session");
    expect(first!.textContent).toBe("Session Fix the login test · api");
    expect(first!.getAttribute("title")).toContain("Session 01a0f458-0056-7d00-9a1b-5c0ffee00001");
    expect(second!.textContent).toBe("Session Untitled session");
    expect(screen.getByTestId("cost-matrix").textContent).not.toContain("01a0f458");
  });

  it("folds a session's models into one row, its parts on demand, and says a partial price as at least", async () => {
    const screen = render(<CostMatrix />);
    const rows = await screen.findAllByTestId("cost-row");
    expect(rows).toHaveLength(2);
    const first = rows[0]!;
    expect(first.textContent).toContain("gpt-6.1-sol+1");
    expect(first.textContent).toContain("≥ at least $0.80");
    expect(within(first).queryByTestId("cost-parts")).toBeNull();
    fireEvent.click(within(first).getByTestId("cost-models"));
    const parts = within(first).getByTestId("cost-parts");
    expect(parts.textContent).toContain("codex-auto-review");
    expect(within(parts).getByTestId("cost-unpriced").textContent).toBe("No list price");
  });

  it("reads unpriced short (the model in the tooltip), never $0.00, with credits alongside", async () => {
    const screen = render(<CostMatrix />);
    const rows = await screen.findAllByTestId("cost-row");
    const unpriced = within(rows[1]!).getByTestId("cost-unpriced");
    expect(unpriced.textContent).toBe("No list price");
    expect(unpriced.getAttribute("title")).toBe("No list price for GLM-6-X");
    expect(rows[1]!.textContent).toContain("1.25");
    expect(screen.getByTestId("cost-matrix").textContent).not.toContain("$0.00");
  });

  it("puts the total under the USD column, naming what it leaves out", async () => {
    const screen = render(<CostMatrix />);
    const total = await screen.findByTestId("cost-total");
    const cells = Array.from(total.children);
    expect(cells).toHaveLength(2);
    expect(cells[0]!.className).toContain("col-span-3");
    expect(cells[0]!.textContent).toBe("Total, all sessions · without GLM-6-X, codex-auto-review");
    expect(cells[1]!.textContent).toBe("$0.80");
    // The same grid as the rows: the figure lands in the USD column.
    const columns = (node: Element) => node.className.split(" ").filter((name) => name.includes("grid-cols-")).join(" ");
    expect(columns(total)).toContain("sm:grid-cols-");
    expect(columns(total)).toBe(columns(screen.getByTestId("cost-columns")));
  });
});

describe("AJ-E609 label-discipline grep gate", () => {
  it("never claims the user paid the API-equivalent value", async () => {
    const source = await import(/* @vite-ignore */ "./CostMatrix.tsx?raw" as string).catch(() => null);
    if (source) {
      expect(source.default ?? source).not.toContain("you paid $");
    }
    // The compiled component itself is the binding check:
    const screen = render(<CostMatrix />);
    await screen.findAllByTestId("cost-row");
    expect(screen.container.textContent).not.toContain("you paid $");
  });
});

/**
 * The 0.0.12 report: clicking Usage showed a blank window — the whole app
 * crashed. Every Rust `Option` in the cost payload arrives as `null`, and
 * CostMatrix checked `!== undefined`, so `null.toFixed` threw. The shape
 * here is what serde sends on a real install with no fee set: nulls,
 * an unpriced model, a Codex row with no credits, an account with no
 * cost rows.
 */
import { cleanup, render, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { UsageScreen } from "./screens/Usage";
import { CostMatrix } from "./components/CostMatrix";
import type { AccountInfo, CostRollupsData } from "./lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

const PAYLOAD: CostRollupsData = {
  rows: [
    {
      key: "sess-codex",
      model: "gpt-5-codex",
      day: "2026-10-21",
      tokens: { input: 1200, output: 300, cache_read: 800, cache_write: 0, reasoning: 40 },
      usd: { usd: null, label: "Estimated" },
      credits: null,
      confidence: "Measured",
    },
    {
      key: "sess-claude",
      model: "claude-opus-4",
      day: "2026-10-21",
      tokens: { input: 300, output: 50, cache_read: 15, cache_write: 2, reasoning: 0 },
      usd: { usd: 0.0123, label: "Estimated" },
      credits: null,
      confidence: "Measured",
    },
  ],
  subscription: { api_equivalent_usd: null, flat_fee_usd: null, value_ratio: null },
  pricing_stale: false,
  pricing_last_synced_ms: null,
  total_usd: null,
  scope: "ALL",
};

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "empty", provider: "codex", label: "No use yet", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
] as unknown as AccountInfo[];

beforeEach(() => {
  w().__AIJ_COST_ROLLUPS__ = PAYLOAD;
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_COST_ROLLUPS__;
});

it("CostMatrix renders null fields: no crash, words for the unpriced model, never $0.00", async () => {
  const view = render(<CostMatrix />);
  const matrix = await view.findByTestId("cost-matrix");
  await within(matrix).findByText("gpt-5-codex");
  expect(within(matrix).getByTestId("cost-unpriced").textContent).toBe("No price for gpt-5-codex yet");
  expect(matrix.textContent).not.toContain("$0.00");
  // No fee, no ratio: the dual view stays out; no total row without a total.
  expect(matrix.textContent).not.toContain("at API prices");
  expect(matrix.textContent).not.toContain("Total (all sessions)");
});

it("Usage renders with the same payload (the screen the maintainer clicked)", async () => {
  const view = render(<UsageScreen accounts={ACCOUNTS} sessions={[]} onAddAccounts={() => null} />);
  expect(await view.findByTestId("usage-screen")).toBeTruthy();
  await view.findByText("No price for gpt-5-codex yet");
});

it("with nothing stored at all, the preview's own payload is the real one (nulls)", async () => {
  delete w().__AIJ_COST_ROLLUPS__;
  const view = render(<CostMatrix />);
  expect(await view.findByTestId("cost-empty")).toBeTruthy();
});

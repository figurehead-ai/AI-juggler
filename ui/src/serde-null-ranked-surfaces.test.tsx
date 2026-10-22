/**
 * QA batch 7 (P26, P26b, P27), on main through the Dev bridge, found the
 * same null class in four more places:
 * - Ranked rows and switch targets read a null forecast as "exhausts ≈0
 *   min · Estimate", and the ranked rows showed the raw account id.
 * - New session's "Which account?" said "0% Estimate … Room left unknown".
 * - The account view's rings said "0% · … · last measured 1m ago".
 *
 * Each is fed serde's shape here (every Option null): no 0%, no ≈0, no
 * raw id, no "% left", and one claim per row.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { RankedRows } from "./components/RankedRows";
import { SwitchTargetRows } from "./components/SwitchTargetRows";
import { WindowRing } from "./components/QuotaRings";
import { NewSessionModal } from "./screens/NewSession";
import type { AccountInfo, RankedRowData, SwitchTargetData, WindowDisplay } from "./lib/bridge";
import { quotaDisplay, windowDisplay } from "./lib/wire";

const w = () => window as unknown as Record<string, unknown>;
const RAW_ID = "claude-code-36858693000";
const PERCENT = /\d+\s*%/;

const ranked = (over: Partial<RankedRowData> = {}): RankedRowData => ({
  account: RAW_ID,
  account_label: "Work Max",
  score: 0.7,
  filtered_reason: null,
  top_factors: ["most room left"],
  factors: [],
  headroom_unknown: true,
  confidence: "Estimate",
  tte_min: null,
  suggested: true,
  admission: null,
  ...over,
});

const tracking = (): WindowDisplay => windowDisplay("tracking", { percentage: null, stale_note_ms: 60_000, burned_tokens: 4_000 });

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_RANKING__;
  delete w().__AIJ_QUOTA_DISPLAY__;
  delete w().__AIJ_RECIPES__;
  vi.restoreAllMocks();
});

it("ranked rows: the account's name, and no forecast when there isn't one", () => {
  render(<RankedRows rows={[ranked()]} onPick={() => null} />);
  const list = screen.getByTestId("ranked-rows").textContent ?? "";
  expect(list).toContain("Work Max");
  expect(list).not.toContain(RAW_ID);
  expect(list).toContain("Not enough use to forecast");
  expect(list).not.toContain("≈0");
});

it("switch targets: no forecast when there isn't one", () => {
  const target: SwitchTargetData = {
    recipe_id: "r-1",
    account_id: RAW_ID,
    account_label: "Work Max",
    badge: "Full story, fresh start · Beta",
    validated: false,
    cross_provider: false,
    drops: [],
    score: 0.7,
    filtered_reason: null,
    top_factors: [],
    factors: [],
    confidence: "Estimate",
    tte_min: null,
    suggested: true,
  };
  const { container } = render(<SwitchTargetRows targets={[target]} onPick={() => null} />);
  expect(container.textContent).toContain("Not enough use to forecast");
  expect(container.textContent).not.toContain("≈0");
});

it("the account view's rings: serde's tracking-with-null reads as not measured", () => {
  const { container } = render(
    <div>
      <WindowRing window={tracking()} kind="5h" />
      <WindowRing window={tracking()} kind="weekly" />
    </div>,
  );
  expect(container.textContent).not.toMatch(PERCENT);
  expect(container.querySelector("svg")).toBeNull();
  expect(container.textContent).toContain("Not measured yet");
});

it("New session: each account row makes one claim, from the one runway", async () => {
  const accounts = [
    { id: RAW_ID, provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  ] as unknown as AccountInfo[];
  w().__AIJ_RECIPES__ = [{ id: "p-1", name: "Main", account_id: RAW_ID, account_label: "Work Max", provider: "claude-code", model: null, default_cwd: null }];
  w().__AIJ_RANKING__ = [ranked()];
  w().__AIJ_QUOTA_DISPLAY__ = [quotaDisplay(RAW_ID, { five_hour: tracking(), weekly: tracking() })];
  const { container } = render(
    <NewSessionModal projects={[]} accounts={accounts} initialDirectory={null} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />,
  );
  await waitFor(() => expect(container.querySelector('[data-testid="launch-runway"]')).not.toBeNull());
  const text = container.textContent ?? "";
  expect(text).toContain("Not measured yet");
  expect(text).not.toMatch(PERCENT);
  expect(text).not.toContain("≈0");
  expect(text).not.toContain("Room left unknown");
  expect(text).not.toContain(RAW_ID);
});

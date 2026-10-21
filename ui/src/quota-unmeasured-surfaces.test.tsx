/**
 * The 0.0.12 report: every actively used Claude account read "100% left ·
 * resets Wed 03:07 PM", with full bars, on Home's Runway and the Accounts
 * cards. The windows had use (tokens) but no measured sample; a missing
 * percentage was read as 0% used, and a reset estimated from local use
 * was shown as the vendor's.
 *
 * One account with tokens but no sample, rendered on every surface that
 * shows a window: no "% left", no full bar or ring, the tokens it
 * measured, and the reset labelled as an estimate.
 */
import { cleanup, render, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AccountInfo, QuotaDisplayData, WindowDisplay } from "./lib/bridge";

const ACCOUNT = {
  id: "claude-work",
  provider: "claude-code",
  label: "Work Max",
  plan: "max-20x",
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
} as unknown as AccountInfo;

// No sessions: a session's own measured reading may say "% left"; this
// account has none, only use counted from its transcripts.
vi.hoisted(() => {
  (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = {
    accounts: [
      {
        id: "claude-work",
        provider: "claude-code",
        label: "Work Max",
        plan: "max-20x",
        login_state: "logged_in",
        cleanup_opt_in: false,
        renewal_date: null,
        preview_opt_in: false,
        in_place: false,
      },
    ],
    profiles: [],
    sessions: [],
    projects: [],
  };
});

import App from "./App";
import { UsageScreen } from "./screens/Usage";
import { QuotaChip, WindowRing } from "./components/QuotaRings";
import { windowView } from "./lib/quotaWindow";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

const RESETS = Date.now() + 26 * 3_600_000;
const unmeasured = (tokens: number): WindowDisplay => ({
  state: "unmeasured",
  burned_tokens: tokens,
  resets_at_ms: RESETS,
  reset_basis: "estimated",
});
const DISPLAY: QuotaDisplayData = {
  account: "claude-work",
  five_hour: unmeasured(12_000),
  weekly: unmeasured(12_000),
  renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
  forecast: { state: "learning", badge: "Learning" },
};

const PERCENT_LEFT = /\d+\s*% left/;

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
  (window as unknown as Record<string, unknown>).__AIJ_QUOTA_DISPLAY__ = [DISPLAY];
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  delete (window as unknown as Record<string, unknown>).__AIJ_QUOTA_DISPLAY__;
});

it("the one rule: no percentage, no '% left'; the tokens; the reset as an estimate", () => {
  const view = windowView(unmeasured(12_000));
  expect(view.left).toBeNull();
  expect(view.tone).toBe("none");
  expect(view.words).not.toMatch(PERCENT_LEFT);
  expect(view.words).toMatch(/^Not measured yet · 12k tokens used · resets about .+, estimated$/);
  // A tracking window that somehow lacks a percentage reads the same.
  expect(windowView({ state: "tracking", burned_tokens: 0 }).left).toBeNull();
});

it("Home's Runway and the Accounts cards", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  const runway = await view.findByText(/Not measured yet · 12k tokens used/);
  expect(runway).toBeTruthy();
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);

  await user.click(await view.findByRole("button", { name: "Accounts" }));
  const card = await view.findByTestId("account-runway");
  expect(card.textContent).toContain("Not measured yet · 12k tokens used");
  expect(card.textContent).toContain("estimated");
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);
  // The bars are the dashed empty kind, never filled.
  const fills = [...card.querySelectorAll<HTMLElement>("span[style]")].map((bar) => bar.style.width);
  expect(fills.length).toBeGreaterThan(0);
  expect(fills.every((width) => width === "0%")).toBe(true);
});

it("Usage", async () => {
  const view = render(<UsageScreen accounts={[ACCOUNT]} sessions={[]} onAddAccounts={() => null} />);
  const runway = await view.findByTestId("account-runway");
  expect(runway.textContent).toContain("Not measured yet");
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);
});

it("the inspector's rings and the New session chip", () => {
  const rings = render(
    <div>
      <WindowRing window={DISPLAY.five_hour} kind="5h" />
      <WindowRing window={DISPLAY.weekly} kind="weekly" />
      <QuotaChip display={DISPLAY} />
    </div>,
  );
  const five = rings.getByTestId("ring-5h");
  expect(five.getAttribute("data-state")).toBe("unmeasured");
  expect(five.querySelector("svg")).toBeNull();
  expect(within(five).getByText(/Not measured yet/)).toBeTruthy();
  expect(rings.getByText("not measured yet")).toBeTruthy();
  expect(rings.container.textContent).not.toMatch(PERCENT_LEFT);
  expect(rings.container.textContent).not.toContain("blocked");
});

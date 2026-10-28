/**
 * The 0.0.12 report: every actively used Claude account read "100% left ·
 * resets Wed 03:07 PM", with full bars, on Home's Runway and the Accounts
 * cards. The windows had use (tokens) but no measured sample; a missing
 * percentage was read as 0% used, and a reset estimated from local use
 * was shown as the vendor's.
 *
 * One account with tokens but no sample, rendered on every surface that
 * shows a window: no "% left", no full bar or ring, the tokens it
 * measured, and no reset time (P12: the backend sends none without a
 * vendor sample).
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
import { quotaDisplay, windowDisplay } from "./lib/wire";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

// Every kind counted (cache too), and the window's tokens in and out.
const unmeasured = (tokens: number): WindowDisplay => windowDisplay("unmeasured", { burned_tokens: tokens * 50, burned_io_tokens: tokens });
const DISPLAY: QuotaDisplayData = quotaDisplay("claude-work", {
  five_hour: unmeasured(12_000),
  weekly: unmeasured(12_000),
});

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

it("the one rule: no percentage, no '% left'; the tokens; no reset time", () => {
  const view = windowView(unmeasured(12_000));
  expect(view.left).toBeNull();
  expect(view.tone).toBe("none");
  expect(view.words).not.toMatch(PERCENT_LEFT);
  expect(view.words).toBe("Not measured yet · used 12k tokens in and out");
  expect(view.resets).toBeNull();
  // The 0.2 walk (F15): "tokens" means in and out, as on Usage; a reading
  // without that split says it counts cache too.
  expect(windowView(windowDisplay("unmeasured", { burned_tokens: 7_700_000_000 })).words).toBe("Not measured yet · used 7.7B tokens, cache included");
  // A tracking window that somehow lacks a percentage reads the same.
  expect(windowView(windowDisplay("tracking")).left).toBeNull();
});

it("Home's Runway and the Accounts cards", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  const runway = await view.findByText(/Not measured yet · used 12k tokens in and out/);
  expect(runway).toBeTruthy();
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);

  await user.click(await view.findByRole("button", { name: "Accounts" }));
  const card = await view.findByTestId("account-runway");
  expect(card.textContent).toContain("Not measured yet");
  expect(card.textContent).toContain("Used 12k tokens in and out");
  expect(card.textContent).not.toContain("resets");
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);
  // 0.1.5: "not measured" in words, never a filled ring, and said once
  // for both windows (one gauge), not once per window.
  const gauges = [...card.querySelectorAll<HTMLElement>("[data-testid='window-gauge']")];
  expect(gauges.length).toBe(1);
  expect(gauges[0]!.dataset.state).toBe("unmeasured");
  expect(card.textContent!.split("Not measured yet").length - 1).toBe(1);
  expect(card.querySelectorAll("circle[stroke-dasharray]").length).toBe(0);
});

/** QA walking main through the Dev bridge (2026-10-21): the backend sent
 * `state: "tracking"` with `percentage: null` (serde's shape), and Home's
 * own copy of the runway read it as "100% left" for all 6 accounts. Home
 * and Accounts now share one runway; with that payload neither says it. */
it("serde's tracking-with-null payload: no '% left' on Home or Accounts", async () => {
  const tracking = (tokens: number): WindowDisplay => windowDisplay("tracking", { percentage: null, burned_tokens: tokens, burned_io_tokens: tokens });
  (window as unknown as Record<string, unknown>).__AIJ_QUOTA_DISPLAY__ = [
    quotaDisplay("claude-work", { five_hour: tracking(9_000), weekly: tracking(9_000) }),
  ];
  const user = userEvent.setup();
  const view = render(<App />);
  expect(await view.findByText(/Not measured yet · used 9k tokens in and out/)).toBeTruthy();
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);
  await user.click(await view.findByRole("button", { name: "Accounts" }));
  const card = await view.findByTestId("account-runway");
  expect(card.textContent).toContain("Not measured yet");
  expect(card.textContent).toContain("9k tokens in and out");
  expect(view.container.textContent).not.toMatch(PERCENT_LEFT);
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

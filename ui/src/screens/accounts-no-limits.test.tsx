/**
 * 0.0.11 walk, Q3: a tool that doesn't report its usage limits shows no
 * 5-hour / This week rows that would never fill — it says so once, on
 * the account — and every account carries its own tool's mark.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

vi.hoisted(() => {
  (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = {
    accounts: [
      { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: true },
      { id: "codex-main", provider: "codex", label: "Codex", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: true },
    ],
    profiles: [],
    sessions: [],
    projects: [],
  };
});

import { ProfilesScreen } from "./Profiles";
import { limitProviders, reportsLimits } from "../lib/providers";

afterEach(() => {
  cleanup();
  delete (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_CAPABILITIES__;
});

it("a tool that doesn't report its limits says so once; Claude Code keeps its windows", async () => {
  // Codex as the example of such a tool (it reports them since 0.0.13).
  (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_CAPABILITIES__ = [
    { provider: "claude-code", can_use_in_place: true, reports_limits: true },
    { provider: "codex", can_use_in_place: true, reports_limits: false },
  ];
  const view = render(<ProfilesScreen onAddAccounts={() => {}} openTerminal={() => {}} />);
  const note = await view.findByTestId("account-no-limits");
  expect(note.textContent).toBe("Codex doesn't report its limits to AI Juggler yet.");
  expect(view.getAllByTestId("account-no-limits")).toHaveLength(1);
  const cards = view.getAllByTestId("account-card");
  const codex = cards.find((card) => card.textContent?.includes("Codex doesn't report"))!;
  expect(codex.textContent).not.toMatch(/5-hour|This week|Not measured yet/);
  const claude = cards.find((card) => card !== codex)!;
  await waitFor(() => expect(claude.textContent).toMatch(/5-hour|This week|Not measured yet/));
  // AJ-E3205 req. 4: no lone letter ("C" for both); with accounts on two
  // tools, each card names its tool in words.
  expect(view.queryByTestId("account-tool-mark")).toBeNull();
  expect(view.getAllByTestId("account-line").map((line) => line.textContent?.split(" · ")[0])).toEqual(["Claude Code", "Codex"]);
});

it("a failed capabilities call, or a tool it doesn't name, keeps the limit rows", () => {
  expect(reportsLimits("unknown", "codex")).toBe(true);
  const known = limitProviders([
    { provider: "claude-code", reports_limits: true },
    { provider: "codex", reports_limits: false },
  ]);
  expect(reportsLimits(known, "claude-code")).toBe(true);
  expect(reportsLimits(known, "codex")).toBe(false);
  // A tool the list doesn't name keeps its rows: only a "no" hides them.
  expect(reportsLimits(known, "fake-desktop")).toBe(true);
});

it("Codex reports its limits now (rollout rate_limits): no 'doesn't report' line", async () => {
  const view = render(<ProfilesScreen onAddAccounts={() => {}} openTerminal={() => {}} />);
  await waitFor(() => expect(view.getAllByTestId("account-card")).toHaveLength(2));
  expect(view.queryByTestId("account-no-limits")).toBeNull();
});

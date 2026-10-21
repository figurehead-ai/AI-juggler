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

afterEach(cleanup);

it("Codex says once that it doesn't report its limits; Claude Code keeps its windows", async () => {
  const view = render(<ProfilesScreen onAddAccounts={() => {}} openTerminal={() => {}} />);
  const note = await view.findByTestId("account-no-limits");
  expect(note.textContent).toBe("Codex doesn't report its limits to AI Juggler yet.");
  expect(view.getAllByTestId("account-no-limits")).toHaveLength(1);
  const cards = view.getAllByTestId("account-tool-mark").map((mark) => mark.closest("[class*='overflow-hidden']")!);
  const codex = cards.find((card) => card.textContent?.includes("Codex doesn't report"))!;
  expect(codex.textContent).not.toMatch(/5-hour|This week|Not measured yet/);
  const claude = cards.find((card) => card !== codex)!;
  await waitFor(() => expect(claude.textContent).toMatch(/5-hour|This week|Not measured yet/));
  // Each tool's own mark, not Claude's spark on every card.
  expect(view.getAllByTestId("account-tool-mark").map((mark) => mark.getAttribute("data-provider"))).toEqual(["claude-code", "codex"]);
});

it("a failed capabilities call keeps every tool's limit rows", () => {
  expect(reportsLimits("unknown", "codex")).toBe(true);
  const known = limitProviders([
    { provider: "claude-code", reports_limits: true },
    { provider: "codex", reports_limits: false },
  ]);
  expect(reportsLimits(known, "claude-code")).toBe(true);
  expect(reportsLimits(known, "codex")).toBe(false);
});

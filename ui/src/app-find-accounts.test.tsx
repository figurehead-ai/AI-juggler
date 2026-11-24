/**
 * 0.0.11 walk: with no account yet, Home's way in comes back to first
 * run — every tool's setups at once, as first run's Not now promises —
 * not to one tool's Add account. AJ-E1545: that way in is Get started,
 * and first run is the setup wizard's first step.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

// The bridge reads its fixture when it loads: no accounts on this Mac yet.
vi.hoisted(() => {
  (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
});

import App from "./App";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  // First run was left with Not now.
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
  // Two tools installed: Add account would ask which one first.
  (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_INSTALLS__ = [
    { provider: "claude-code", display_name: "Claude Code", install_url: "", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
    { provider: "codex", display_name: "Codex", install_url: "", status: "installed", version: "0.163.0", binary: "/opt/homebrew/bin/codex", notes: [] },
  ];
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  delete (window as unknown as Record<string, unknown>).__AIJ_PROVIDER_INSTALLS__;
});

it("Get started on an empty Home reopens the setup at first run, not one tool's Add account", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  const start = await view.findByRole("button", { name: "Get started" });
  await user.click(start);
  expect(await view.findByTestId("setup-wizard")).toBeTruthy();
  const frame = await view.findByTestId("first-run");
  expect(frame.getAttribute("data-mode")).toBe("first-run");
  expect(view.queryByText("Which tool is it for?", { exact: false })).toBeNull();
  // Not now leaves it again, back to the same Home.
  await user.click(view.getByTestId("setup-not-now"));
  await waitFor(() => expect(view.queryByTestId("setup-wizard")).toBeNull());
  expect(view.getByTestId("home-empty-accounts")).toBeTruthy();
});

// F20 (0.5.1 PL1 C1): an empty data folder (no accounts, never dismissed)
// opened straight on the wizard's step 1, so the Welcome was never seen.
it("an empty data folder opens on the Welcome, and the wizard only on Get started", async () => {
  localStorage.clear();
  const user = userEvent.setup();
  const view = render(<App />);
  const welcome = await view.findByTestId("home-empty-accounts");
  expect(welcome.textContent).toContain("Let's get you set up");
  expect(view.getByTestId("welcome-bot")).toBeTruthy();
  // A beat later still the Welcome: nothing opens the wizard by itself.
  await new Promise((done) => setTimeout(done, 50));
  expect(view.queryByTestId("setup-wizard")).toBeNull();
  await user.click(view.getByRole("button", { name: "Get started" }));
  expect(await view.findByTestId("setup-wizard")).toBeTruthy();
});

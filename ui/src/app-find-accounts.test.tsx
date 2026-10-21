/**
 * 0.0.11 walk: with no account yet, Home's "Find my accounts" comes back
 * to first run — every tool's setups at once, as first run's Not now
 * promises — not to one tool's Add account.
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

it("Find my accounts on an empty Home reopens first run, not one tool's Add account", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  const find = await view.findByRole("button", { name: "Find my accounts" });
  await user.click(find);
  const frame = await view.findByTestId("first-run");
  expect(frame.getAttribute("data-mode")).toBe("first-run");
  expect(view.queryByText("Which tool is it for?", { exact: false })).toBeNull();
  // Not now leaves it again, back to the same Home.
  await user.click(view.getByTestId("first-run-close"));
  await waitFor(() => expect(view.queryByTestId("first-run")).toBeNull());
  expect(view.getByTestId("home-empty-accounts")).toBeTruthy();
});

/**
 * The 0.0.11 report: opened from Finder, Codex's `--version` failed (its
 * `node` wasn't on the app's PATH), so Codex counted as missing and Add
 * account went straight to Claude. A tool AI Juggler found whose version
 * it couldn't read is found: Add account asks which tool, and first run
 * names it — "found; couldn't check its version", never "not installed".
 */
import { cleanup, render, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { FirstRun } from "./screens/FirstRun";
import type { DiscoveredInfo } from "./lib/bridge";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

const w = () => window as unknown as Record<string, unknown>;

const INSTALLS = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
  // Found at its path; `--version` exited 127 ("env: node: No such file").
  { provider: "codex", display_name: "Codex", install_url: "https://developers.openai.com/codex", status: "probe_failed", version: null, binary: "/opt/homebrew/bin/codex", notes: [] },
];

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
  w().__AIJ_PROVIDER_INSTALLS__ = INSTALLS;
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  delete w().__AIJ_PROVIDER_INSTALLS__;
  delete w().__AIJ_SCAN_REPORT__;
});

it("Add account offers a tool whose version check failed, in plain words", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  await user.click(await view.findByRole("button", { name: "Accounts" }));
  await user.click(await view.findByTestId("add-account"));
  const chooser = await view.findByRole("dialog", { name: "Add an account: which tool?" });
  const cards = within(chooser).getAllByTestId("provider-card");
  const codex = cards.find((card) => card.textContent?.includes("Codex"))!;
  expect(within(codex).getByTestId("provider-state").textContent).toBe("Found; couldn't check its version");
  expect(codex.textContent).not.toContain("Not installed");
  await user.click(within(codex).getByTestId("provider-choose"));
  const frame = await view.findByTestId("first-run");
  expect(frame.getAttribute("data-mode")).toBe("add");
});

it("first run lists that tool's setups and doesn't call it missing", async () => {
  const now = Math.floor(Date.now() / 1000);
  w().__AIJ_SCAN_REPORT__ = {
    installs: [
      {
        harness: "codex",
        path: "/Users/you/.codex",
        origin: "vendor-default",
        login: "logged_in",
        bytes: 1000,
        last_used_secs: now - 3600,
        managed: false,
        notes: [],
        signals: ["vendor-default"],
      } as unknown as DiscoveredInfo,
    ],
    probes: [],
    seeds: [],
    desktop_apps: [],
    cancelled: false,
  };
  const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  const group = await view.findByTestId("first-run-group-codex");
  expect(within(group).getByTestId("first-run-row").textContent).toContain("~/.codex");
  expect((await view.findByTestId("first-run-cli-codex")).textContent).toBe("Codex found; couldn't check its version");
  expect(view.queryByTestId("first-run-no-codex")).toBeNull();
});

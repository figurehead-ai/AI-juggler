/**
 * D-97 (1): the end-of-setup Home lists what's next — each tool first
 * run's scan found and the user didn't bring in ("Codex is installed: sign
 * in to add it"), one click to its Add flow, and "Measure its limits" per
 * Account not measured yet — until a session has run.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { FirstRun } from "./FirstRun";
import { renderWithNeeds } from "../test/needs-kit";
import { ADD_APP, foundToolsOf, rememberedFound, rememberFound } from "../lib/found-tools";
import { account, clearSeams, detailOf, instance, setSeam } from "../test/fixtures";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ScanReportData } from "../lib/ipc/providers";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";

vi.mock("../components/TerminalView", () => ({ TerminalView: () => null }));

const NOW = 1_728_800_000_000;
const ACCOUNTS = [
  account({ id: "work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" }),
  account({ id: "personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" }),
];
const install: DiscoveredInfo = {
  harness: "claude-code", path: "/Users/you/.claude", origin: "vendor-default", set_by: null, login: "logged_in", bytes: 1000, last_used_secs: null,
  managed: false, notes: [], signals: [], in_use: [],
  inference: {
    purpose: "Main", purpose_confidence: "found", purpose_rule: "usual-folder", login: "signed-in", login_confidence: "found", login_rule: "sign-in-present",
    activity: "unused", activity_confidence: "guessed", activity_rule: "no-conversations", activity_words: "Not used yet", keychain_bound: false,
  },
};
const SCAN: ScanReportData = { installs: [install], probes: [], seeds: [], desktop_apps: [], not_set_up: ["codex"], cancelled: false };

const renderHome = (sessions: InstanceInfo[] = []) =>
  renderWithNeeds(
    { sessions, accounts: ACCOUNTS, projects: [] },
    <HomeScreen sessions={sessions} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={() => null} onNewSession={() => null} onAddAccounts={() => null} />,
  );

afterEach(cleanup);
beforeEach(() => {
  localStorage.clear();
  clearSeams("__AIJ_QUOTA_DISPLAY__", "__AIJ_SCAN_REPORT__", "__AIJ_PROBE__");
});

describe("D-97 Home's Next steps", () => {
  it("lists a found tool left out, one click to its Add flow, and Measure its limits per Account not measured", async () => {
    rememberFound(SCAN);
    // 0.4.4: Measure its limits asks the tool here (D-39), when it's on.
    const on = { offered: true, state: "on", last_run_ms: null, last_outcome: null };
    setSeam("__AIJ_PROBE__", { work: on, personal: on });
    const opened: string[] = [];
    const listen = (event: Event) => opened.push(JSON.stringify(detailOf(event)));
    window.addEventListener(ADD_APP, listen);
    const view = renderHome();
    const next = await view.findByTestId("home-next");
    // Claude Code came in; Codex was left unticked: it doesn't just disappear.
    const tools = within(next).getAllByTestId("home-next-tool");
    expect(tools.map((row) => row.textContent)).toEqual(["Codex is installed: sign in to add itAdd"]);
    fireEvent.click(within(tools[0]!).getByRole("button", { name: "Add Codex" }));
    expect(opened).toEqual([JSON.stringify({ tool: "codex" })]);
    window.removeEventListener(ADD_APP, listen);
    await waitFor(() => expect(within(next).getAllByTestId("home-next-measure")).toHaveLength(2));
    await waitFor(() => expect(within(next).getAllByRole("button", { name: /^Measure its limits — / })).toHaveLength(2));
  });

  it("D-103.4: an Account whose tool has no check offers How it's measured, opening where its limits come from under the row", async () => {
    rememberFound({ ...SCAN, not_set_up: [] });
    // No ask offered for either folder: nothing to measure here.
    setSeam("__AIJ_PROBE__", {});
    const view = renderHome();
    const next = await view.findByTestId("home-next");
    await waitFor(() => expect(within(next).getAllByRole("button", { name: /^How it's measured — / })).toHaveLength(2));
    expect(within(next).queryAllByRole("button", { name: /^Measure its limits/ })).toHaveLength(0);
    const row = within(next).getAllByTestId("home-next-measure")[0]!;
    fireEvent.click(within(row).getByRole("button", { name: /^How it's measured — / }));
    expect(within(row).getByTestId("measure-limits-how-words").textContent).toMatch(/^No current reading\. Limits come from /);
  });

  it("leaves a measured Account out, and goes once a session has run", async () => {
    setSeam("__AIJ_QUOTA_DISPLAY__", ["work", "personal"].map((id) => ({ account: id, five_hour: { percentage: 30, state: "measured", resets_at_ms: null }, weekly: { percentage: 20, state: "measured", resets_at_ms: null } })));
    rememberFound({ ...SCAN, not_set_up: [] });
    const view = renderHome();
    await waitFor(() => expect(view.getByTestId("home").textContent).toContain("70% left"));
    expect(view.queryByTestId("home-next")).toBeNull();
    cleanup();
    rememberFound(SCAN);
    const running = renderHome([instance({ id: "s", account_id: "work", status: "running" })]);
    await waitFor(() => expect(running.getByTestId("home")).toBeTruthy());
    expect(running.queryByTestId("home-next")).toBeNull();
  });

  it("first run's scan is what it lists from: the found tools, set up or only installed", async () => {
    expect(foundToolsOf(SCAN)).toEqual([
      { tool: "claude-code", setUp: true },
      { tool: "codex", setUp: false },
    ]);
    setSeam("__AIJ_SCAN_REPORT__", SCAN);
    const view = render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-found");
    expect(rememberedFound()).toEqual(foundToolsOf(SCAN));
  });
});

import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";
import { forgetProcessRows } from "../screens/processes/useProcessRows";

/**
 * AJ-E2404/E2407: a session's Processes tab is the Processes view for
 * that session alone, read from the one graph (here the preview's
 * made-up Mac, as the dev preview shows it): its tree with roles and
 * states, what it left behind with Stop… (the verified sheet), and the
 * picked-up notice for a session AI Juggler took back after a restart.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));

const NOW = 1_728_800_000_000;

function instance(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    adopted: false,
    launch_recipe_id: "p-1",
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW,
    machine_id: "this-mac",
    cwd: "/w/x",
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
    ...overrides,
  };
}

const base = {
  projects: [] as ProjectInfo[],
  accounts: [
    { id: "a1", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null },
  ] as AccountInfo[],
  profiles: [] as LaunchRecipeInfo[],
  terminalTitle: null,
  freshness: {},
  now: NOW,
  onOpenProject: () => {},
  onOpenSession: () => {},
  onLaunch: async () => {},
  onStopped: () => {},
  onHandover: () => {},
  onNew: () => {},
};

function processesShell(row: InstanceInfo) {
  const view = render(
    <RightInspector
      selection={{ kind: "instance", id: row.id }}
      data={{ instance: row, project: null, account: null, provider: null, machineId: "this-mac" }}
      sessions={[row]}
      {...base}
    />,
  );
  fireEvent.click(view.container.querySelector("#page-tab-processes")!);
  return view;
}

const names = (container: HTMLElement) =>
  [...container.querySelectorAll('[data-testid^="process-row-p:"]')].map((row) => row.textContent ?? "");

afterEach(cleanup);
beforeEach(() => {
  forgetProcessRows();
  delete (window as unknown as { __AIJ_PROCESSES__?: unknown }).__AIJ_PROCESSES__;
});

describe("AJ-E2407 a session's Processes tab is the one view", () => {
  it("shows that session's tree alone, with roles in words and what started them", async () => {
    const { container } = processesShell(instance("s-web"));
    await waitFor(() => expect(names(container).length).toBeGreaterThan(3));
    const text = names(container).join("\n");
    expect(text).toContain("node (vite)");
    expect(text).toContain("probably a dev server");
    expect(text).toContain("Started by Bash: npm run dev");
    // Another session's processes are not here.
    expect(text).not.toContain("webpack");
    // AI Juggler's own tools are hidden unless asked for.
    expect(text).not.toContain("AI Juggler's tools");
  });

  it("lists what a session left behind with Stop…, which opens the verified sheet", async () => {
    const { container } = processesShell(instance("s-pay"));
    await waitFor(() => expect(container.querySelector('[data-testid="process-stop-8123"]')).not.toBeNull());
    const row = container.querySelector('[data-testid^="process-row-p:8123"]') as HTMLElement;
    expect(within(row).getByTestId("process-state").textContent).toBe("Left behind");
    fireEvent.click(container.querySelector('[data-testid="process-stop-8123"]')!);
    await waitFor(() => expect(document.querySelector('[role="dialog"]')).not.toBeNull());
  });

  it("explains a process waiting for its parent and offers no stop", async () => {
    const { container } = processesShell(instance("s-odd"));
    await waitFor(() => expect(container.querySelector('[data-testid^="process-row-p:6610"]')).not.toBeNull());
    const row = container.querySelector('[data-testid^="process-row-p:6610"]') as HTMLElement;
    expect(row.textContent).toContain("Waiting for its parent");
    expect(row.textContent).toContain("only codex (6600) can clear it");
    expect(container.querySelector('[data-testid="process-stop-6610"]')).toBeNull();
  });

  it("a session with nothing running says so", async () => {
    const { container } = processesShell(instance("s-none"));
    await waitFor(() => expect(container.querySelector('[data-testid="session-processes"]')?.textContent).toContain("No sessions are running processes now."));
  });
});

describe("AJ-E510 re-adopted Instances (crash recovery)", () => {
  it("shows the picked-up chip and the detached notice leading the tab", async () => {
    const { container } = processesShell(instance("s-docs", { adopted: true }));
    await waitFor(() => expect(container.querySelector('[data-testid="instance-detached-notice"]')).not.toBeNull());
    const chip = container.querySelector('[data-testid="instance-adopted-chip"]');
    expect(chip?.textContent).toBe("↻ picked up after restart");
    const notice = container.querySelector('[data-testid="instance-detached-notice"]')!;
    expect(notice.textContent).toContain("The terminal is detached: output from before the app closed is not available.");
    expect(container.querySelector('[data-testid="processes-tab"]')!.firstElementChild).toBe(notice);
  });

  it("a supervised (never-adopted) Instance shows neither chip nor notice", async () => {
    const { container } = processesShell(instance("s-web", { adopted: false }));
    await waitFor(() => expect(names(container).length).toBeGreaterThan(0));
    expect(container.querySelector('[data-testid="instance-adopted-chip"]')).toBeNull();
    expect(container.querySelector('[data-testid="instance-detached-notice"]')).toBeNull();
  });
});

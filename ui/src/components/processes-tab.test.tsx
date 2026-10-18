import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProcessesInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E410 evidence: the Processes tab — the live tree table per §7.3,
 * the orphan section with PID-named confirms and verified stop states,
 * the manual Retry on read failure, and the good empty states.
 */

// The seam (ProcessesBridge) is injected — vitest 5's mocker did not
// intercept this tab's dynamic-import site, so the component exposes
// its data source as a prop (production default = the real bridge).
const e410 = {
  processes: {} as Record<string, ProcessesInfo | null>,
  stopImpl: (_sessionId: string, _pid: number) => "gone" as string,
  stopCalls: [] as [string, number][],
  stopAllCalls: [] as string[],
  verifyCalls: [] as [string, number][],
  throws: false,
};

const processesBridge = {
  load: async (id: string): Promise<ProcessesInfo | null> => {
    if (e410.throws) throw new Error("sysinfo probe failed");
    return e410.processes[id] ?? null;
  },
  stop: async (sessionId: string, pid: number) => {
    e410.stopCalls.push([sessionId, pid]);
    return { pid, state: e410.stopImpl(sessionId, pid) };
  },
  stopAll: async (sessionId: string) => {
    e410.stopAllCalls.push(sessionId);
    const orphans = e410.processes[sessionId]?.orphans ?? [];
    return orphans.map((orphan) => ({
      pid: orphan.pid,
      state: orphan.pid === 39890 ? "still alive - escalated to SIGKILL" : "gone",
    }));
  },
  verify: async (sessionId: string, pid: number) => {
    e410.verifyCalls.push([sessionId, pid]);
    return {
      pid,
      alive: true,
      parent_pid: 1,
      note: "alive — re-parented to init/launchd (its parent exited)",
    };
  },
};

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));

const NOW = 1_728_800_000_000;
const REFUSED_PID = 99_999;

const fixtures: Record<string, ProcessesInfo> = {
  tree: {
    rows: [
      { pid: 41201, parent_pid: 40100, name: "claude", cpu_pct: 11.0, rss_bytes: 890_000_000, age_ms: 42 * 60_000, child_count: 2, depth: 0 },
      { pid: 41233, parent_pid: 41201, name: "node (mcp)", cpu_pct: 1.0, rss_bytes: 96_000_000, age_ms: 40 * 60_000, child_count: 1, depth: 1 },
      { pid: 41302, parent_pid: 41233, name: "rg (search)", cpu_pct: null, rss_bytes: 120_000_000, age_ms: 12_000, child_count: 0, depth: 2 },
    ],
    spark: [{ at_ms: NOW - 2_000, cpu_pct: 7.5, rss_bytes: 1_106_000_000 }],
    orphans: [
      { session_id: "tree", pid: 39871, name: "node dev-server", rss_bytes: 210_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: null },
      { session_id: "tree", pid: 39890, name: "vite", rss_bytes: 88_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: null },
    ],
    unavailable_reason: null,
  },
  clean: {
    rows: [],
    spark: [],
    orphans: [],
    unavailable_reason: null,
  },
  unavailable: {
    rows: [],
    spark: [],
    orphans: [],
    unavailable_reason: "resources unavailable",
  },
};

Object.assign(e410.processes, fixtures);

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
      processesBridge={processesBridge}
      {...base}
    />,
  );
  fireEvent.click(view.container.querySelector("#inspector-tab-processes")!);
  return view;
}

afterEach(cleanup);
beforeEach(() => {
  e410.stopCalls.length = 0;
  e410.stopAllCalls.length = 0;
  e410.verifyCalls.length = 0;
  e410.stopImpl = () => "gone";
  e410.throws = false;
});

describe("AJ-E410 Processes tab (§7.3 + orphan contract)", () => {
  it("renders the live tree with PID/name/CPU/RAM/age/⑂ and honest first-sample CPU", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-tab"]')).not.toBeNull());
    const table = container.querySelector('[data-testid="processes-tab"] table')!;
    expect(table.textContent).toContain("41201");
    expect(table.textContent).toContain("claude");
    expect(table.textContent).toContain("node (mcp)");
    expect(table.textContent).toContain("rg (search)");
    expect(table.textContent).toContain("890 MB"); // RSS formatted
    expect(table.textContent).toContain("42m");
    expect(table.textContent).toContain("— first sample"); // null CPU: honest, never 0%
    // Tree depth: the rg row carries the indent glyph.
    expect(table.innerHTML).toContain("└");
    // Headers are real table headers (a11y).
    expect(container.querySelectorAll('[data-testid="processes-tab"] th[scope="col"]').length).toBe(6);
  });

  it("renders the orphan section with copy-exact heading, parents, and PID-labelled stop buttons", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-orphans"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="orphans-heading"]')!.textContent).toBe(
      "Orphaned - left behind (2)",
    );
    expect(container.querySelector('[data-testid="orphan-row-39871"]')!.textContent).toContain("node dev-server");
    expect(container.querySelector('[data-testid="orphan-row-39871"]')!.textContent).toContain("parent: claude");
    expect(container.querySelector('[data-testid="orphan-row-39871"]')!.textContent).toContain("since exit 18m");
    const stopButton = [
      ...container.querySelectorAll("button"),
    ].find((button) => button.textContent === "Stop process 39871")!;
    expect(stopButton).toBeDefined();
    expect(stopButton.getAttribute("aria-label") ?? "").toBe(""); // labelled by its visible text
  });

  it("confirms before stopping, names the PID, then shows gone ✓ on the row", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="orphan-row-39871"]')).not.toBeNull());
    fireEvent.click([...container.querySelectorAll("button")].find((b) => b.textContent === "Stop process 39871")!);
    const confirm = container.querySelector('[data-testid="orphan-confirm-39871"]')!;
    expect(confirm.getAttribute("role")).toBe("alertdialog");
    expect(confirm.textContent).toContain("39871");
    expect(confirm.textContent).toContain("SIGTERM");
    fireEvent.click([...container.querySelectorAll("button")].find((b) => b.textContent === "Stop 39871")!);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="orphan-state-39871"]')!.textContent).toBe("gone ✓"),
    );
    expect(e410.stopCalls).toContainEqual(["tree", 39871]);
  });

  it("Stop all lists every PID in its confirm and lands the escalated state verbatim", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-orphans"]')).not.toBeNull());
    fireEvent.click([...container.querySelectorAll("button")].find((b) => b.textContent === "Stop all orphans…")!);
    const confirm = container.querySelector('[data-testid="orphans-confirm-all"]')!;
    expect(confirm.textContent).toContain("39871");
    expect(confirm.textContent).toContain("39890");
    fireEvent.click([...container.querySelectorAll("button")].find((b) => b.textContent === "Stop all")!);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="orphan-state-39890"]')!.textContent).toBe(
        "still alive - escalated to SIGKILL",
      ),
    );
    expect(e410.stopAllCalls).toContainEqual("tree");
  });

  it("Verify owner shows the re-checked ppid evidence", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="orphan-row-39871"]')).not.toBeNull());
    fireEvent.click([...container.querySelectorAll("button")].find((b) => b.textContent === "Verify owner")!);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="orphan-verify-39871"]')!.textContent).toContain(
        "re-parented to init/launchd",
      ),
    );
    expect(e410.verifyCalls.length).toBeGreaterThan(0);
  });

  it("good empty states: No subprocesses / No orphaned processes", async () => {
    const { container } = processesShell(instance("clean"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-empty"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="processes-empty"]')!.textContent).toBe("No subprocesses");
    expect(container.querySelector('[data-testid="orphans-heading"]')!.textContent).toBe("No orphaned processes");
  });

  it("adopted/desktop-kind sessions render resources unavailable — never zeroed rows", async () => {
    const { container } = processesShell(instance("unavailable"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-unavailable"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="processes-unavailable"]')!.textContent).toContain(
      "resources unavailable",
    );
    expect(container.querySelector('[data-testid="processes-tab"] table')).toBeNull();
  });

  it("tree-read failure renders an alert banner with a manual Retry", async () => {
    e410.throws = true;
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-error"]')).not.toBeNull());
    const banner = container.querySelector('[data-testid="processes-error"]')!;
    expect(banner.getAttribute("role")).toBe("alert");
    expect(banner.textContent).toContain("Process tree read failed");
    e410.throws = false;
    fireEvent.click([...banner.querySelectorAll("button")].find((b) => b.textContent === "Retry")!);
    await waitFor(() => expect(container.querySelector('[data-testid="processes-tab"] table')).not.toBeNull());
  });

  it("a refused stop surfaces the guardrail text on the row", async () => {
    e410.stopImpl = (_sessionId: string, pid: number) =>
      pid === REFUSED_PID ? "refused - not a recorded descendant" : "gone";
    e410.processes.tree = {
      ...(e410.processes.tree as ProcessesInfo),
      orphans: [
        ...(e410.processes.tree as ProcessesInfo).orphans,
        { session_id: "tree", pid: REFUSED_PID, name: "stray", rss_bytes: 1_000, parent_label: "claude", age_since_parent_exit_ms: 5_000, stop_state: null },
      ],
    };
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector(`[data-testid="orphan-row-${REFUSED_PID}"]`)).not.toBeNull());
    fireEvent.click(
      [...container.querySelectorAll("button")].find((b) => b.textContent === `Stop process ${REFUSED_PID}`)!,
    );
    fireEvent.click(
      [...container.querySelectorAll("button")].find((b) => b.textContent === `Stop ${REFUSED_PID}`)!,
    );
    await waitFor(() =>
      expect(container.querySelector(`[data-testid="orphan-state-${REFUSED_PID}"]`)!.textContent).toBe(
        "refused - not a recorded descendant",
      ),
    );
    // restore the fixture for later tests
    e410.processes.tree = {
      ...(e410.processes.tree as ProcessesInfo),
      orphans: (e410.processes.tree as ProcessesInfo).orphans.filter((orphan) => orphan.pid !== REFUSED_PID),
    };
  });
});

describe("AJ-E510 re-adopted Instances (crash recovery)", () => {
  it("shows the ↻ re-adopted chip and the detached-notice card leading the processes tab", async () => {
    const { container } = processesShell(instance("tree", { adopted: true }));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-tab"] table')).not.toBeNull());
    const chip = container.querySelector('[data-testid="instance-adopted-chip"]');
    expect(chip?.textContent).toBe("↻ re-adopted");
    const notice = container.querySelector('[data-testid="instance-detached-notice"]');
    expect(notice?.textContent).toContain("Running — re-adopted after restart");
    expect(notice?.textContent).toContain(
      "The terminal is detached: output from before the app closed is not available.",
    );
    expect(notice?.textContent).toContain("verified sheet");
    // The notice LEADS the tab — the user reads why output is missing
    // before any row.
    expect(container.querySelector('[data-testid="processes-tab"]')!.firstElementChild).toBe(notice);
  });

  it("a supervised (never-adopted) Instance shows neither chip nor notice", async () => {
    const { container } = processesShell(instance("tree", { adopted: false }));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-tab"] table')).not.toBeNull());
    expect(container.querySelector('[data-testid="instance-adopted-chip"]')).toBeNull();
    expect(container.querySelector('[data-testid="instance-detached-notice"]')).toBeNull();
  });
});

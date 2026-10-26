import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProcessesInfo, ProjectInfo } from "../lib/bridge";
import type { CleanupRowInfo } from "./ProcessRow";
import type { KillBridge, KillVerifyData } from "./KillSheet";

/**
 * AJ-E410 evidence: the Processes tab — the live tree table per §7.3,
 * the manual Retry on read failure, and the good empty states. G-1:
 * what the session left behind is the SHARED Cleanup row, and every
 * Stop opens the verified sheet (verify first) — never a direct stop.
 */

// The seam (ProcessesBridge) is injected — vitest 5's mocker did not
// intercept this tab's dynamic-import site, so the component exposes
// its data source as a prop (production default = the real bridge).
const e410 = {
  processes: {} as Record<string, ProcessesInfo | null>,
  cleanup: {} as Record<string, CleanupRowInfo[]>,
  verifyCalls: [] as [string, number][],
  executeCalls: [] as [string, number][],
  verified: true,
  throws: false,
};

function leftover(pid: number, overrides: Partial<CleanupRowInfo> = {}): CleanupRowInfo {
  return {
    machine_id: "this-mac",
    pid,
    pgid: 40100,
    role: "helper",
    state: "orphaned",
    state_label: "Still running after its session closed",
    role_guess: "dev server",
    exe_path: "/usr/local/bin/node",
    cmdline: "node node_modules/.bin/vite dev --port 5173",
    age_ms: 18 * 60_000,
    rss_bytes: 210 * 1024 * 1024,
    cpu_pct: 0.4,
    ports: "5173",
    hog: false,
    hog_rules: [],
    evidence: {
      root_pid: 40100,
      parent_pid: 40100,
      pgid: 40100,
      escaped_group: false,
      verification: { verdict: "verified", detail: "start-time and exe match the live process" },
      first_seen: 1,
      last_seen: 2,
    },
    ...overrides,
  };
}

function verifyData(session: string, pid: number): KillVerifyData {
  return {
    pid,
    session,
    verified: e410.verified,
    refusal: e410.verified ? null : "This PID now belongs to a different program — Juggler will not touch it.",
    exe_path: "/usr/local/bin/node",
    cmdline: "node node_modules/.bin/vite dev --port 5173",
    cwd: "/w/x",
    ports: [5173],
    pgid: 40100,
    escaped_group: false,
    members: [],
    tree_pids: [pid],
    platform: "TERM → grace → KILL",
    grace_secs: 5,
    provider: "claude-code",
    account_id: "a1",
    project_id: null,
  };
}

const kill: KillBridge = {
  verify: async (session, pid) => {
    e410.verifyCalls.push([session, pid]);
    return verifyData(session, pid);
  },
  execute: async (session, pid) => {
    e410.executeCalls.push([session, pid]);
    return { refusal: null, members: [{ pid, outcome: "exited on TERM" }], cancelled: false, all_succeeded: true, still_holding: [], reoffer_pid: null };
  },
  cancel: async () => undefined,
  liveness: async (pids) => pids.map(() => false),
};

const processesBridge = {
  load: async (id: string): Promise<ProcessesInfo | null> => {
    if (e410.throws) throw new Error("sysinfo probe failed");
    return e410.processes[id] ?? null;
  },
  cleanup: async (id: string) => ({ rows: e410.cleanup[id] ?? [], localMachine: "this-mac" }),
  kill,
};

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));

const NOW = 1_728_800_000_000;

const fixtures: Record<string, ProcessesInfo> = {
  tree: {
    rows: [
      { pid: 41201, parent_pid: 40100, name: "claude", cpu_pct: 11.0, rss_bytes: 890_000_000, age_ms: 42 * 60_000, child_count: 2, depth: 0 },
      { pid: 41233, parent_pid: 41201, name: "node (mcp)", cpu_pct: 1.0, rss_bytes: 96_000_000, age_ms: 40 * 60_000, child_count: 1, depth: 1 },
      { pid: 41302, parent_pid: 41233, name: "rg (search)", cpu_pct: null, rss_bytes: 120_000_000, age_ms: 12_000, child_count: 0, depth: 2 },
    ],
    spark: [{ at_ms: NOW - 2_000, cpu_pct: 7.5, rss_bytes: 1_106_000_000 }],
    unavailable_reason: null,
  },
  clean: {
    rows: [],
    spark: [],
    unavailable_reason: null,
  },
  unavailable: {
    rows: [],
    spark: [],
    unavailable_reason: "resources unavailable",
  },
};

Object.assign(e410.processes, fixtures);
e410.cleanup.tree = [
  // The live tree already shows this one: it is not listed twice.
  leftover(41233, { state: "running", state_label: "Running" }),
  leftover(39871),
  leftover(39890, { cmdline: "node esbuild --watch src/index.ts", role_guess: "watcher", ports: null }),
  leftover(39900, {
    state: "unverifiable",
    state_label: "Unverifiable — will not be touched",
    cmdline: "postgres -D /var/pg",
    evidence: { ...leftover(1).evidence, verification: { verdict: "unverifiable", detail: "differs" } },
  }),
];
e410.cleanup.unavailable = [leftover(55120, { state: "adopted", state_label: "Running — picked up after restart" })];

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
  fireEvent.click(view.container.querySelector("#page-tab-processes")!);
  return view;
}

afterEach(cleanup);
beforeEach(() => {
  e410.verifyCalls.length = 0;
  e410.executeCalls.length = 0;
  e410.verified = true;
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
    expect(table.textContent).toContain("measuring…"); // null CPU: honest, never 0%
    // Tree depth: the rg row carries the indent glyph.
    expect(table.innerHTML).toContain("└");
    // Headers are real table headers (a11y).
    expect(container.querySelectorAll('[data-testid="processes-tab"] th[scope="col"]').length).toBe(6);
  });

  it("lists what was left behind as the shared Cleanup rows, without the live tree's processes", async () => {
    const { container } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="cleanup-row-39871"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="leftovers-heading"]')!.textContent).toBe("Left behind (3)");
    // The same row component as Cleanup, with its column headings.
    const section = container.querySelector('[data-testid="session-leftovers"]')!;
    expect(section.querySelector('[data-testid="process-rows-header"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="cleanup-name-39871"]')!.textContent).toBe("vite");
    expect(container.querySelector('[data-testid="cleanup-role-39871"]')!.textContent).toBe("probably a dev server");
    // 41233 is in the live tree above: never listed twice.
    expect(container.querySelector('[data-testid="cleanup-row-41233"]')).toBeNull();
    // No second list, no ledger dump, no machine-wide port list here.
    const tab = container.querySelector('[data-testid="processes-tab"]')!.textContent ?? "";
    expect(tab).not.toContain("Ledger rows");
    expect(tab).not.toContain("Stop all orphans");
    expect(tab).not.toContain("Verify owner");
    expect(container.querySelector('[data-testid^="port-row-"]')).toBeNull();
    // An unverifiable row offers no Stop at all.
    expect(container.querySelector('[data-testid="cleanup-kill-39900"]')).toBeNull();
  });

  it("Stop opens the verified sheet — it verifies first, and nothing is signalled in one click", async () => {
    const { container, findByRole } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="cleanup-kill-39871"]')).not.toBeNull());
    fireEvent.click(container.querySelector('[data-testid="cleanup-kill-39871"]')!);
    const sheet = await findByRole("dialog");
    await waitFor(() => expect(e410.verifyCalls).toContainEqual(["tree", 39871]));
    // Verified: the consequence-named confirm is offered, not run.
    await waitFor(() => expect(sheet.querySelector('[data-testid="kill-confirm-39871"]')).not.toBeNull());
    expect(e410.executeCalls).toEqual([]);
  });

  it("a recycled pid is refused by the sheet: no confirm button exists", async () => {
    e410.verified = false;
    const { container, findByRole } = processesShell(instance("tree"));
    await waitFor(() => expect(container.querySelector('[data-testid="cleanup-kill-39871"]')).not.toBeNull());
    fireEvent.click(container.querySelector('[data-testid="cleanup-kill-39871"]')!);
    const sheet = await findByRole("dialog");
    await waitFor(() => expect(sheet.textContent ?? "").toContain("belongs to a different program"));
    expect(sheet.querySelector('[data-testid="kill-confirm-39871"]')).toBeNull();
    expect(e410.executeCalls).toEqual([]);
  });

  it("good empty states: No subprocesses / Nothing left behind, and a way to Cleanup", async () => {
    const { container } = processesShell(instance("clean"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-empty"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="processes-empty"]')!.textContent).toBe("No subprocesses");
    await waitFor(() =>
      expect(container.querySelector('[data-testid="leftovers-heading"]')!.textContent).toBe("Nothing left behind"),
    );
    const section = container.querySelector('[data-testid="session-leftovers"]')!.textContent ?? "";
    expect(section).toContain("Nothing this session started is still running after it.");
    const opened: string[] = [];
    const onOpen = (event: Event) => opened.push((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener("aij:open-screen", onOpen);
    fireEvent.click(container.querySelector('[data-testid="leftovers-open-cleanup"]')!);
    window.removeEventListener("aij:open-screen", onOpen);
    expect(opened).toEqual(["cleanup"]);
  });

  it("adopted/desktop-kind sessions render resources unavailable — never zeroed rows", async () => {
    const { container } = processesShell(instance("unavailable"));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-unavailable"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="processes-unavailable"]')!.textContent).toContain(
      "resources unavailable",
    );
    expect(container.querySelector('[data-testid="processes-tab"] table')).toBeNull();
    // With no tree to read, its processes are the shared rows, stoppable
    // through the verified sheet.
    await waitFor(() => expect(container.querySelector('[data-testid="cleanup-kill-55120"]')).not.toBeNull());
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

});

describe("AJ-E510 re-adopted Instances (crash recovery)", () => {
  it("shows the ↻ picked-up chip and the detached-notice card leading the processes tab", async () => {
    const { container } = processesShell(instance("tree", { adopted: true }));
    await waitFor(() => expect(container.querySelector('[data-testid="processes-tab"] table')).not.toBeNull());
    const chip = container.querySelector('[data-testid="instance-adopted-chip"]');
    expect(chip?.textContent).toBe("↻ picked up after restart");
    const notice = container.querySelector('[data-testid="instance-detached-notice"]');
    expect(notice?.textContent).toContain("Running — picked up after restart");
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

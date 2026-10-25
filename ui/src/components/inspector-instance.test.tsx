import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector, type InspectorSelection } from "./RightInspector";
import { isHandoverShortcut, isStopShortcut } from "../lib/shortcuts";
import type {
  AccountInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";

/**
 * AJ-E407 evidence (Instance tabs): every §5.1 field present on the
 * populated fixture or its honest empty on the bare one, asserted by
 * `data-testid`; actions disabled WITH reason, never hidden; shortcuts
 * operable through the mocked bridge.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => (
    <div data-terminal-id={id} data-testid="instance-terminal" />
  ),
}));

const { launchMock, terminalKill } = vi.hoisted(() => ({
  launchMock: vi.fn(async () => ({ session_id: "sess-new", pid: 1 })),
  terminalKill: vi.fn(async (_id: string) => {}),
}));
vi.mock("../lib/bridge", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/bridge")>();
  return { ...original, terminalKill, launchSession: launchMock };
});

afterEach(cleanup);
beforeEach(() => {
  localStorage.clear();
  terminalKill.mockClear();
  launchMock.mockClear();
});

const NOW = 1_728_800_000_000;

function mk(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-1",
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: { used_percentage: 40, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
    tokens: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    cpu_pct: 12,
    rss_bytes: 1_100_000_000,
    child_count: 3,
    subtask_count: null,
    git_branch: "main",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 42 * 60_000,
    last_activity_ms: NOW - 5_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: "822e4125-e07d",
    ...overrides,
  };
}

const project: ProjectInfo = {
  id: "proj-1", root: "/Users/dev/ai-juggler", name: "ai-juggler", last_activity_ms: null,
  active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null,
  missing: false,
};
const account: AccountInfo = {
  id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false, in_place: false,
};
const recipe: LaunchRecipeInfo = {
  id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max",
  provider: "claude-code", model: "opus", default_cwd: null, restart_on_crash: null,
};

function shell(instance: InstanceInfo | null, selection?: InspectorSelection) {
  return render(
    <RightInspector
      selection={selection ?? { kind: "instance", id: instance?.id ?? "sess-1" }}
      data={{
        instance,
        project,
        account,
        provider: null,
        machineId: "this-mac",
      }}
      sessions={instance ? [instance] : []}
      projects={[project]}
      accounts={[account]}
      profiles={[recipe]}
      terminalTitle="Work Max · instance"
      freshness={{}}
      now={NOW}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={async () => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
    />,
  );
}

const open = (container: HTMLElement, tab: string) =>
  fireEvent.click(container.querySelector(`#page-tab-${tab}`)!);

describe("AJ-E407 Instance Overview (§5.1, Req.1)", () => {
  it("renders every populated field", () => {
    const { container } = shell(mk());
    expect(container.querySelector('[data-testid="instance-overview-fields"]')).not.toBeNull();
    // D-25: the header carries the state and how long (no duplicate chip).
    expect(container.textContent).toContain("started 42m ago");
    expect(container.textContent).toContain("Claude Code");
    expect(container.textContent).toContain("Work Max");
    expect(container.textContent).toContain("Deep work");
    expect(container.textContent).toContain("opus");
    expect(container.querySelector('[data-testid="instance-cwd"]')?.textContent).toContain("/Users/dev/ai-juggler");
    expect(container.querySelector('[data-testid="instance-native-id"]')?.textContent).toContain("822e4125-e07d");
    expect(container.querySelector('[data-testid="instance-quota"]')?.textContent).toContain("60% left");
    expect(container.querySelector('[data-testid="instance-tokens"]')?.textContent).toContain("19,134");
    expect(container.querySelector('[data-testid="instance-resources"]')?.textContent).toContain("helpers 3");
    expect(container.querySelector('[data-testid="handover-section"]')).not.toBeNull();
    // Lineage: OMITTED while the Switch Graph does not exist (empty rule).
    expect(container.textContent).not.toContain("lineage");
  });

  it("renders the honest empties on a bare instance", () => {
    const { container } = shell(
      mk({ quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, model: null, conversation_id: null }),
    );
    expect(container.querySelector('[data-testid="instance-native-id"]')?.textContent).toContain("not found yet");
    expect(container.querySelector('[data-testid="instance-tokens"]')?.textContent).toContain("No usage recorded yet");
    expect(container.querySelector('[data-testid="instance-quota"]')?.textContent).toContain("Not measured yet");
    expect(container.textContent).toContain("default model");
  });
});

describe("AJ-E407 Instance Usage (Req.2)", () => {
  it("renders the real token breakdown with per-series confidence", () => {
    const { container } = shell(mk());
    open(container, "usage");
    expect(container.querySelector('[data-testid="instance-usage"]')).not.toBeNull();
    expect(container.textContent).toContain("12,345");
    expect(container.textContent).toContain("6,789");
    expect(container.textContent).toContain("Measured");
  });

  it("without tokens says so; the over-time chart waits for a time series (D-25)", () => {
    const { container } = shell(mk({ tokens: null }));
    open(container, "usage");
    expect(container.textContent).toContain("No usage recorded yet");
    expect(container.textContent).not.toContain("Tokens / min");
    expect(container.textContent).not.toMatch(/no data|arrives with|statusline/i);
  });

  it("names the tool, not its id, and the state in the rail's words", () => {
    const { container } = shell(mk({ status: "quota_blocked" }));
    expect(container.textContent).not.toContain("claude-code");
    expect(container.textContent).toContain("Limit reached");
    expect(container.textContent).not.toContain("quota_blocked");
  });
});

describe("AJ-E1533 a session's page: no empty tabs, actions in the header", () => {
  it("has no History tab until a session records events", () => {
    const { container } = shell(mk());
    expect(container.querySelector("#page-tab-history")).toBeNull();
    expect(container.querySelector("#page-tab-actions")).toBeNull();
  });

  it("a running session's header offers Switch account…; an ended one's, Relaunch", () => {
    const running = shell(mk());
    expect(running.queryByTestId("header-switch")).not.toBeNull();
    expect(running.queryByTestId("header-relaunch")).toBeNull();
    cleanup();
    const ended = shell(mk({ status: "finished" }));
    expect(ended.getByTestId("header-relaunch").textContent).toBe("Relaunch");
  });

  it("no tab of the session view shows a task or decision code (D-25)", () => {
    for (const status of ["running", "finished", "quota_blocked"]) {
      const { container } = shell(mk({ status }));
      for (const tab of ["terminal", "usage", "processes"]) {
        open(container, tab);
        expect(container.textContent ?? "", `${status} › ${tab}`).not.toMatch(/\bAJ-E\d+|\bE\d+\+|\(E\d+\)|\bD-\d{2}\b/);
      }
      cleanup();
    }
  });
});

describe("AJ-E407 shortcuts (real predicates, mocked bridge)", () => {
  it("⌘. stops the selected live instance through the bridge", async () => {
    // The App handler, verbatim wiring: predicate → bridge kill.
    const onKey = (event: KeyboardEvent) => {
      if (isStopShortcut(event)) {
        event.preventDefault();
        void terminalKill("sess-1");
      }
    };
    window.addEventListener("keydown", onKey);
    fireEvent.keyDown(window, { key: ".", metaKey: true });
    window.removeEventListener("keydown", onKey);
    await vi.waitFor(() => expect(terminalKill).toHaveBeenCalledWith("sess-1"));
    // Shifted/plain "." never stops anything.
    expect(isStopShortcut({ key: ".", metaKey: false, ctrlKey: false, shiftKey: false, altKey: false })).toBe(false);
  });

  it("⌘⇧H jumps to the hand-over section on the session's body", () => {
    const { container } = shell(mk());
    open(container, "usage");
    expect(isHandoverShortcut({ key: "h", metaKey: true, ctrlKey: false, shiftKey: true, altKey: false })).toBe(true);
    expect(isHandoverShortcut({ key: "h", metaKey: true, ctrlKey: false, shiftKey: false, altKey: false })).toBe(false);
    // App's translation, verbatim: predicate → the inspector event.
    const onKey = (event: KeyboardEvent) => {
      if (isHandoverShortcut(event)) {
        event.preventDefault();
        window.dispatchEvent(new Event("aij:focus-handover"));
      }
    };
    window.addEventListener("keydown", onKey);
    fireEvent.keyDown(window, { key: "h", metaKey: true, shiftKey: true });
    window.removeEventListener("keydown", onKey);
    expect(container.querySelector("#page-tab-terminal")!.getAttribute("aria-selected")).toBe("true");
  });
});

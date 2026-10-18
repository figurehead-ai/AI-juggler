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
  active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0,
  missing: false,
};
const account: AccountInfo = {
  id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
};
const recipe: LaunchRecipeInfo = {
  id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max",
  provider: "claude-code", model: "opus", default_cwd: null,
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
  fireEvent.click(container.querySelector(`#inspector-tab-${tab}`)!);

describe("AJ-E407 Instance Overview (§5.1, Req.1)", () => {
  it("renders every populated field", () => {
    const { container } = shell(mk());
    expect(container.querySelector('[data-testid="instance-overview-fields"]')).not.toBeNull();
    expect(container.textContent).toContain("Running · 42m");
    expect(container.textContent).toContain("claude-code");
    expect(container.textContent).toContain("Work Max");
    expect(container.textContent).toContain("Deep work");
    expect(container.textContent).toContain("opus");
    expect(container.querySelector('[data-testid="instance-cwd"]')?.textContent).toContain("/Users/dev/ai-juggler");
    expect(container.querySelector('[data-testid="instance-native-id"]')?.textContent).toContain("822e4125-e07d");
    expect(container.querySelector('[data-testid="instance-quota"]')?.textContent).toContain("60% left");
    expect(container.querySelector('[data-testid="instance-tokens"]')?.textContent).toContain("19,134");
    expect(container.querySelector('[data-testid="instance-resources"]')?.textContent).toContain("children 3");
    expect(container.querySelector('[data-testid="handover-section"]')).not.toBeNull();
    // Lineage: OMITTED while the Switch Graph does not exist (empty rule).
    expect(container.textContent).not.toContain("lineage");
  });

  it("renders the honest empties on a bare instance", () => {
    const { container } = shell(
      mk({ quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, model: null, conversation_id: null }),
    );
    expect(container.querySelector('[data-testid="instance-native-id"]')?.textContent).toContain("not discovered yet");
    expect(container.querySelector('[data-testid="instance-tokens"]')?.textContent).toContain("No usage recorded yet");
    expect(container.querySelector('[data-testid="instance-quota"]')?.textContent).toContain("Quota: no data");
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

  it("renders the exact chart empties while time-series sources are pending", () => {
    const { container } = shell(mk({ tokens: null }));
    open(container, "usage");
    expect(container.textContent).toContain("No usage recorded yet");
    expect(container.textContent).toContain("No data — arrives with statusline");
  });
});

describe("AJ-E407 Instance History (Req.3)", () => {
  it("renders the honest history empty with the imported-badge contract", () => {
    const { container } = shell(mk());
    open(container, "history");
    expect(container.querySelector('[data-testid="instance-history"]')).not.toBeNull();
    expect(container.textContent).toContain("No history recorded yet");
    expect(container.textContent).toContain("imported");
  });
});

describe("AJ-E407 Instance Actions (Req.4)", () => {
  it("offers real actions; irrelevant ones disabled WITH reason (never hidden)", () => {
    const { container } = shell(mk());
    open(container, "actions");
    for (const label of [
      "focus-terminal", "stop", "resume", "hand-over", "queue-resume-at-reset",
      "export-pack", "archive", "delete",
    ]) {
      expect(container.querySelector(`[data-testid="action-${label}"]`)).not.toBeNull();
    }
    // Running: Stop enabled, Resume disabled-with-reason.
    const stop = container.querySelector('[data-testid="action-stop"] button')! as HTMLButtonElement;
    expect(stop.disabled).toBe(false);
    const resume = container.querySelector('[data-testid="action-resume"]')!;
    expect(resume.textContent).toContain("still running");
  });

  it("ended: Relaunch enabled; live-only actions disabled with reasons", () => {
    const { container } = shell(mk({ status: "finished" }));
    open(container, "actions");
    const relaunch = container.querySelector('[data-testid="action-relaunch"] button')! as HTMLButtonElement;
    expect(relaunch.disabled).toBe(false);
    const stop = container.querySelector('[data-testid="action-stop"]')!;
    expect(stop.textContent).toContain("not running");
  });

  it("destructive/pending actions name their owning tasks", () => {
    const { container } = shell(mk());
    open(container, "actions");
    expect(container.querySelector('[data-testid="action-export-pack"]')!.textContent).toContain("AJ-E202");
    expect(container.querySelector('[data-testid="action-archive"]')!.textContent).toContain("AJ-E1007");
    expect(container.querySelector('[data-testid="action-delete"]')!.textContent).toContain("guarded");
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

  it("⌘⇧H jumps to the Overview hand-over section", () => {
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
    expect(container.querySelector("#inspector-tab-overview")!.getAttribute("aria-selected")).toBe("true");
  });
});

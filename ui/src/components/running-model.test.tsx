import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import { InstanceBoard, modelDrift } from "./WorkflowRail";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E412 evidence: running-model telemetry — I-03 prefers the
 * observation, drift renders as text (`Launched with X - now running
 * Y`), unobservable providers are named, and launch-only never poses
 * as verified.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} />,
}));

const NOW = 1_728_800_000_000;

function instance(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-m",
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
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

afterEach(cleanup);

describe("AJ-E412 I-03 running model (Req.1/3/5)", () => {
  it("renders the running model once observed, with the drift marker text as aria-label", () => {
    const drift = instance({
      model: "sonnet-5",
      launch_model: "opus-4.8",
      running_model: "sonnet-5",
      model_source: "in-session",
    });
    expect(modelDrift(drift)).toBe("Launched with opus-4.8 - now running sonnet-5");
    const plain = instance({ model: "opus-4.8", launch_model: "opus-4.8", running_model: "opus-4.8", model_source: "transcript" });
    expect(modelDrift(plain)).toBeNull(); // launch == running: no drift
    const launchOnly = instance({ model: "opus-4.8", launch_model: "opus-4.8", model_source: "launch" });
    expect(modelDrift(launchOnly)).toBeNull(); // never fabricated from intent
  });

  it("the rail row shows the drift marker (keyboard-focusable, text-labelled)", () => {
    const row = instance({
      model: "sonnet-5",
      launch_model: "opus-4.8",
      running_model: "sonnet-5",
      model_source: "in-session",
    });
    const { container } = render(
      <InstanceBoard
        sessions={[row]}
        loginTerminals={[]}
        selectedId={row.id}
        onSelect={() => {}}
        density="comfortable"
        now={NOW}
        projects={[]}
        freshness={{}}
        snooze={{}}
        onSnooze={() => {}}
      />,
    );
    const marker = container.querySelector('[data-testid="model-drift"]')!;
    expect(marker).not.toBeNull();
    expect(marker.getAttribute("aria-label")).toBe("Launched with opus-4.8 - now running sonnet-5");
    expect(marker.getAttribute("tabIndex")).toBe("0"); // keyboard reachable (WCAG)
  });

  it("no drift marker when launch == running or launch-only", () => {
    const rows = [
      instance({ id: "a", model: "opus-4.8", launch_model: "opus-4.8", running_model: "opus-4.8", model_source: "transcript" }),
      instance({ id: "b", model: "opus-4.8", launch_model: "opus-4.8", model_source: "launch" }),
    ];
    const { container } = render(
      <InstanceBoard
        sessions={rows}
        loginTerminals={[]}
        selectedId={null}
        onSelect={() => {}}
        density="comfortable"
        now={NOW}
        projects={[]}
        freshness={{}}
        snooze={{}}
        onSnooze={() => {}}
      />,
    );
    expect(container.querySelector('[data-testid="model-drift"]')).toBeNull();
  });
});

describe("AJ-E412 Instance Overview model line (Req.3/5)", () => {
  function overviewShell(row: InstanceInfo) {
    const view = render(
      <RightInspector
        selection={{ kind: "instance", id: row.id }}
        data={{ instance: row, project: null, account: null, provider: null, machineId: "this-mac" }}
        sessions={[row]}
        {...base}
      />,
    );
    return view;
  }

  it("drift renders the dual-model line verbatim", () => {
    const { container } = overviewShell(instance({
      model: "sonnet-5",
      launch_model: "opus-4.8",
      running_model: "sonnet-5",
      model_source: "in-session",
    }));
    const line = container.querySelector('[data-testid="instance-model-line"]')!;
    expect(line.textContent).toBe("Launched with opus-4.8 - now running sonnet-5 (observed in transcript)");
  });

  it("launch-only shows the overlay + not verified live, never posing as running", () => {
    const { container } = overviewShell(instance({
      model: "opus-4.8",
      launch_model: "opus-4.8",
      model_source: "launch",
    }));
    expect(container.textContent).toContain("not verified live");
    expect(container.querySelector('[data-testid="instance-model-line"]')).toBeNull();
  });

  it("no launch and no observation renders Model: not reported by {Provider}", () => {
    const { container } = overviewShell(instance({ model_source: "unknown" }));
    expect(container.querySelector('[data-testid="instance-model-line"]')!.textContent).toBe(
      "Model: not reported by claude-code",
    );
  });
});

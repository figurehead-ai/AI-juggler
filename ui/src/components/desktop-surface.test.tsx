import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type {
  AccountInfo,
  FocusOutcomeInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";

/**
 * AJ-E413 evidence (desktop Instance surface, Req.3): the desktop
 * surface replaces the terminal card — status/account/project/
 * resources with `Open / Focus window` primary, the per-Provider
 * isolation claim verbatim (single-account warning), focus outcome
 * text after the attempt, no hand-over section — while terminal
 * Instances keep the terminal card and hand-over (regression guard).
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => (
    <div data-terminal-id={id} data-testid="instance-terminal" />
  ),
}));

afterEach(cleanup);
beforeEach(() => {
  localStorage.clear();
  focus.calls.length = 0;
});

// AJ-E413 seam (E410 pattern): the focus source is injected, so the
// statically-imported bridge never needs mocking.
const FOCUSED: FocusOutcomeInfo = {
  outcome: "Focused window",
  permission_missing: false,
  window_title: "notes-a",
};
const focus = {
  calls: [] as string[],
  next: FOCUSED as FocusOutcomeInfo,
  bridge: async (id: string) => {
    focus.calls.push(id);
    return focus.next;
  },
};

const NOW = 1_728_800_000_000;
const LABEL = "Single-account app — Juggler tracks, cannot isolate";

function mk(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-desktop-1",
    launch_recipe_id: "p-3",
    adopted: false,
    provider: "fake-desktop",
    account_id: "fake-desk",
    account_label: "Fake Desk",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: 2.4,
    rss_bytes: 88_000_000,
    child_count: 1,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 42 * 60_000,
    last_activity_ms: NOW - 5_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/sacrificial",
    kind: "desktop",
    isolation_label: LABEL,
    project_id: "proj-3",
    conversation_id: null,
    ...overrides,
  };
}

const project: ProjectInfo = {
  id: "proj-3", root: "/Users/dev/sacrificial", name: "sacrificial", last_activity_ms: null,
  active_instance: null, accounts_used: ["Fake Desk"], instance_count: 1, history_sessions: 0,
  missing: false,
};
const account: AccountInfo = {
  id: "fake-desk", provider: "fake-desktop", label: "Fake Desk", plan: null, login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
};
const recipe: LaunchRecipeInfo = {
  id: "p-3", name: "App", account_id: "fake-desk", account_label: "Fake Desk",
  provider: "fake-desktop", model: null, default_cwd: null,
};

function shell(instance: InstanceInfo) {
  return render(
    <RightInspector
      selection={{ kind: "instance", id: instance.id }}
      data={{
        instance,
        project,
        account,
        provider: null,
        machineId: "this-mac",
      }}
      sessions={[instance]}
      projects={[project]}
      accounts={[account]}
      profiles={[recipe]}
      terminalTitle="Fake Desk · instance"
      freshness={{}}
      now={NOW}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={async () => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
      focusBridge={focus.bridge}
    />,
  );
}

const open = (container: HTMLElement, tab: string) =>
  fireEvent.click(container.querySelector(`#inspector-tab-${tab}`)!);

describe("AJ-E413 desktop Instance surface (Req.3)", () => {
  it("renders the surface with focus primary, honest resources, and NO terminal card", () => {
    const { container } = shell(mk());
    const surface = container.querySelector('[data-testid="desktop-surface"]')!;
    expect(surface).not.toBeNull();
    // Focus is the primary action; the terminal card is not rendered.
    const focus = surface.querySelector('[data-testid="desktop-focus"]') as HTMLButtonElement;
    expect(focus).not.toBeNull();
    expect(focus.textContent).toContain("Open / Focus window");
    expect(container.querySelector('[data-testid="instance-terminal"]')).toBeNull();
    // Status, account, project, resources — measured values, verbatim.
    expect(surface.textContent).toContain("Running");
    expect(surface.textContent).toContain("fake-desktop");
    expect(surface.textContent).toContain("Fake Desk · sacrificial");
    expect(surface.textContent).toContain("84 MB"); // 88e6 bytes, 1024-based
    expect(surface.textContent).toContain("No usage recorded yet");
  });

  it("shows the single-account isolation claim verbatim as a warning", () => {
    const { container } = shell(mk());
    expect(container.querySelector('[data-testid="desktop-isolation"]')?.textContent).toContain(LABEL);
  });

  it("hides the isolation callout when the provider has no label (null, never guessed)", () => {
    const { container } = shell(mk({ isolation_label: null }));
    expect(container.querySelector('[data-testid="desktop-isolation"]')).toBeNull();
  });

  it("focus click calls the bridge and reports the outcome tier verbatim", async () => {
    focus.next = { outcome: "Focused window", permission_missing: false, window_title: "notes-a" };
    const { container } = shell(mk());
    fireEvent.click(container.querySelector('[data-testid="desktop-focus"]')!);
    await vi.waitFor(() =>
      expect(container.querySelector('[data-testid="desktop-focus-outcome"]')?.textContent)
        .toContain("Focused window"),
    );
    expect(container.querySelector('[data-testid="desktop-focus-outcome"]')?.textContent)
      .toContain("notes-a");
    expect(focus.calls).toEqual(["sess-desktop-1"]);
  });

  it("missing permission shows the one-time dismissible explainer (AJ-E414 Req.3)", async () => {
    localStorage.removeItem("aij.a11y-explainer.dismissed");
    focus.next = { outcome: "App opened — window not found", permission_missing: true, window_title: null };
    const { container } = shell(mk());
    fireEvent.click(container.querySelector('[data-testid="desktop-focus"]')!);
    const explainer = await vi.waitFor(() => {
      const node = container.querySelector('[data-testid="desktop-a11y-explainer"]');
      expect(node).not.toBeNull();
      return node!;
    });
    expect(explainer.textContent).toContain(
      "Juggler can open the app; exact-window focus needs Accessibility permission",
    );
    // Copy-exact explainer + a deep-link button; dismissal persists and
    // the next degraded attempt stays quiet.
    expect(explainer.querySelector('[data-testid="open-a11y-settings"]')).not.toBeNull();
    fireEvent.click(explainer.querySelector("button:not([data-testid])")!);
    expect(localStorage.getItem("aij.a11y-explainer.dismissed")).toBe("1");
    await vi.waitFor(() =>
      expect(container.querySelector('[data-testid="desktop-a11y-explainer"]')).toBeNull(),
    );
    fireEvent.click(container.querySelector('[data-testid="desktop-focus"]')!);
    await vi.waitFor(() =>
      expect(container.querySelector('[data-testid="desktop-focus-outcome"]')?.textContent)
        .toContain("App opened — window not found"),
    );
    expect(container.querySelector('[data-testid="desktop-a11y-explainer"]')).toBeNull();
  });

  it("an earlier dismissal keeps the explainer hidden across mounts", async () => {
    localStorage.setItem("aij.a11y-explainer.dismissed", "1");
    focus.next = { outcome: "App opened — window not found", permission_missing: true, window_title: null };
    const { container } = shell(mk());
    fireEvent.click(container.querySelector('[data-testid="desktop-focus"]')!);
    await vi.waitFor(() =>
      expect(container.querySelector('[data-testid="desktop-focus-outcome"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="desktop-a11y-explainer"]')).toBeNull();
  });

  it("no hand-over section on a desktop Instance (there is no terminal context to move)", () => {
    const { container } = shell(mk());
    expect(container.querySelector('[data-testid="handover-section"]')).toBeNull();
  });

  it("exited and crashed states read as app states, not terminal states", () => {
    const exited = shell(mk({ status: "finished" }));
    expect(exited.container.textContent).toContain("app exited · relaunch from Actions");
    expect(exited.container.textContent).not.toContain("scrollback replayed");
    cleanup();
    const crashed = shell(mk({ status: "failed" }));
    expect(crashed.container.textContent).toContain("app crashed · exit details on the Processes tab");
    expect(crashed.container.textContent).not.toContain("scrollback replayed");
  });

  it("Actions row for desktop opens the window (labelled honestly, not 'Focus terminal')", () => {
    const { container } = shell(mk());
    open(container, "actions");
    const row = container.querySelector('[data-testid="action-open-focus-window"] button')! as HTMLButtonElement;
    expect(row.disabled).toBe(false);
    fireEvent.click(row);
    expect(focus.calls).toEqual(["sess-desktop-1"]);
  });

  it("terminal Instances keep the terminal card and hand-over (regression guard)", () => {
    const { container } = shell(
      mk({
        id: "sess-2",
        kind: "terminal",
        provider: "claude-code",
        isolation_label: null,
        model: "opus",
        model_source: "launch",
      }),
    );
    expect(container.querySelector('[data-testid="instance-terminal"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="desktop-surface"]')).toBeNull();
    expect(container.querySelector('[data-testid="handover-section"]')).not.toBeNull();
  });
});

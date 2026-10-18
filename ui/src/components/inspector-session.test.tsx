import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E407 evidence (Session/conversation tabs, §5.2): ended sessions
 * immutable with the Ended stamp; unreadable transcripts render the
 * exact copy, never zeroed stats.
 */

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => (
    <div data-terminal-id={id} data-testid="instance-terminal" />
  ),
}));

afterEach(cleanup);
beforeEach(() => localStorage.clear());

const NOW = 1_728_800_000_000;

const ended: InstanceInfo = {
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
  status: "finished",
  quota: null,
  tokens: { input: 1_000, output: 500, cache_read: 0, cache_write: 0, reasoning: 0 },
  cpu_pct: null,
  rss_bytes: null,
  child_count: 0,
  subtask_count: null,
  git_branch: null,
  git_dirty_count: null,
  needs_attention: null,
  attention: null,
  started_at_ms: NOW - 8_600_000,
  last_activity_ms: NOW - 8_000_000,
  machine_id: "this-mac",
  cwd: "/Users/dev/ai-juggler",
  kind: "terminal",
  isolation_label: null,
  project_id: "proj-1",
  conversation_id: "conv-1",
};

const project: ProjectInfo = {
  id: "proj-1", root: "/Users/dev/ai-juggler", name: "ai-juggler", last_activity_ms: null,
  active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0,
  missing: false,
};
const account: AccountInfo = {
  id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
};
const recipe: LaunchRecipeInfo = {
  id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max",
  provider: "claude-code", model: "opus", default_cwd: null,
};

function sessionShell(instance: InstanceInfo) {
  return render(
    <RightInspector
      selection={{ kind: "session", id: instance.conversation_id ?? "conv-1" }}
      data={{ instance, project, account, provider: null, machineId: "this-mac" }}
      sessions={[instance]}
      projects={[project]}
      accounts={[account]}
      profiles={[recipe]}
      terminalTitle={null}
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

describe("AJ-E407 Session Overview (§5.2, Req.5/6)", () => {
  it("an ended session shows the immutable Ended stamp and real aggregates", () => {
    const { container } = sessionShell(ended);
    expect(container.querySelector('[data-testid="session-overview"]')).not.toBeNull();
    const stamp = container.querySelector('[data-testid="session-ended-stamp"]');
    expect(stamp?.textContent).toMatch(/Ended \d{2}:\d{2}/);
    expect(container.querySelector('[data-testid="session-tokens"]')?.textContent).toContain("1,500");
    expect(container.textContent).toContain("owning Instance");
    // Never zeroed stats: unreadable transcript renders the exact copy.
    expect(container.querySelector('[data-testid="session-transcript"]')?.textContent).toContain(
      "Transcript not readable yet",
    );
  });

  it("a live session has no Ended stamp", () => {
    const { container } = sessionShell({ ...ended, status: "running", last_activity_ms: NOW });
    expect(container.querySelector('[data-testid="session-ended-stamp"]')).toBeNull();
  });

  it("Usage renders the per-turn honest empty (no fabricated tables)", () => {
    const withTokens = sessionShell(ended);
    open(withTokens.container, "usage");
    expect(withTokens.container.textContent).toContain("per-turn rows arrive with transcript indexing");
    cleanup();
    const bare = sessionShell({ ...ended, tokens: null });
    open(bare.container, "usage");
    expect(bare.container.textContent).toContain("No usage recorded yet");
  });

  it("History renders the lineage honest empty (Switch Graph = E2)", () => {
    const { container } = sessionShell(ended);
    open(container, "history");
    expect(container.textContent).toContain("No lineage yet");
    expect(container.textContent).toContain("Switch Graph");
  });

  it("Actions offers Resume primary + disabled-with-reason rows", () => {
    const { container } = sessionShell(ended);
    open(container, "actions");
    const resume = container.querySelector('[data-testid="action-resume"] button')! as HTMLButtonElement;
    expect(resume.disabled).toBe(false);
    expect(container.querySelector('[data-testid="action-export-pack"]')!.textContent).toContain("AJ-E202");
    expect(container.querySelector('[data-testid="action-delete"]')!.textContent).toContain("guarded");
  });
});

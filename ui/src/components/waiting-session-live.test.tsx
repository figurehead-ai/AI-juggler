/**
 * The rc20 walk: a session waiting at Claude Code's folder-trust question
 * (stored as waiting for an answer) showed "ran 41s · scrollback replayed"
 * and no Stop, though its process ran; after Stop, its rail row still said
 * "Asks you". A live process is live whatever it says, and an ended one
 * never keeps what it said before it ended.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import { InspectorStop } from "../shell/pages";
import { sessionItem } from "../lib/rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { RuntimeInfo } from "../types/generated/RuntimeInfo";

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string; title: string }) => <div data-terminal-id={id} data-testid="instance-terminal" />,
}));

afterEach(cleanup);

const NOW = 1_800_000_000_000;

function row(status: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-trust", launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "lab", account_label: "Lab", model: null, launch_model: null,
    running_model: null, model_source: "launch", status, quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: null, git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 41_000,
    last_activity_ms: NOW - 1_000, machine_id: "this-mac", cwd: "/Users/dev/lab/cont", kind: "terminal", isolation_label: null, project_id: null, conversation_id: null,
    ...over,
  };
}

function inspector(session: InstanceInfo) {
  return render(
    <RightInspector
      selection={{ kind: "instance", id: session.id }}
      data={{ instance: session, project: null, account: null, provider: null, machineId: "this-mac" }}
      sessions={[session]}
      projects={[]}
      accounts={[]}
      profiles={[]}
      terminalTitle="Lab · session"
      freshness={{}}
      now={NOW}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={() => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
    />,
  );
}

describe("a live process is live, whatever it says", () => {
  it.each(["waiting_user", "stuck", "quota_blocked", "running", "starting"])("%s: its terminal is live and it has Stop", (status) => {
    const session = row(status);
    const page = inspector(session);
    expect(page.container.textContent).not.toContain("scrollback replayed");
    cleanup();
    const onStop = vi.fn();
    render(<InspectorStop session={session} onStop={onStop} />);
    fireEvent.click(screen.getByTestId("inspector-stop"));
    expect(onStop).toHaveBeenCalledWith(session);
  });

  it.each(["stopped", "finished", "failed"])("%s: replayed, and no Stop", (status) => {
    const session = row(status);
    expect(inspector(session).container.textContent).toContain("scrollback replayed");
    cleanup();
    render(<InspectorStop session={session} onStop={() => {}} />);
    expect(screen.queryByTestId("inspector-stop")).toBeNull();
  });

  it("an ended session never keeps the Asks you it said before it ended", () => {
    const runtime: RuntimeInfo = { kind: "terminal", label: null, beta: false, state: "asks_you", asks: [], can_steer: false, fell_back: null, error: null, limits: [], cut_off: null };
    expect(sessionItem(row("waiting_user", { runtime }), () => null, false).state).toBe("asks_you");
    expect(sessionItem(row("stopped", { runtime }), () => null, false).state).toBe("ended");
  });
});

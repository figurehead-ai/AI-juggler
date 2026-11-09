import { focusHandover } from "../lib/handover-focus";
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { asElement, clearSeams, setSeam } from "../test/fixtures";
import { refreshPools } from "../lib/pools";
import { MOCK_SESSIONS } from "../preview/fixtures";
import { resetPreviewPools } from "../lib/ipc/pools";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector, type InspectorSelection } from "./RightInspector";
import { isHandoverShortcut, isStopShortcut } from "../lib/shortcuts";
import { clock } from "../lib/time-words";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

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
vi.mock("../lib/ipc/terminal-io", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc/terminal-io")>();
  return { ...original, terminalKill };
});
vi.mock("../lib/ipc/launch", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/ipc/launch")>();
  return { ...original, launchSession: launchMock };
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
    expect(container.querySelector('[data-testid="instance-resources"]')?.textContent).toContain("more processes 3");
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
  it("a session on this Mac has a door to This Mac on its facts line (AJ-E3104 R1)", () => {
    const view = shell(mk());
    const door = within(view.getByTestId("page-header")).getByTestId("door-mac");
    const opened = vi.fn();
    const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:open-screen", listen);
    fireEvent.click(door);
    window.removeEventListener("aij:open-screen", listen);
    expect(opened).toHaveBeenCalledWith({ id: "thismac" });
  });

  it("its Pools tab carries its own count, never its project's (AJ-E3104 R1)", async () => {
    const session = mk({ id: MOCK_SESSIONS[2]!.id });
    // In two pools of yours; its project's page would count three (its own pool too).
    setSeam("__AIJ_POOLS__", {
      pools: [{ id: "pool-a", name: "A" }, { id: "pool-b", name: "B" }],
      members: [
        { session_id: session.id, pool_id: "pool-a", role: "lead" },
        { session_id: session.id, pool_id: "pool-b", role: "member" },
      ],
    });
    resetPreviewPools();
    await act(async () => {
      await refreshPools();
    });
    const view = shell(session);
    const pools = asElement(view.container.querySelector("#page-tab-pools"));
    await waitFor(() => expect(pools.textContent).toBe("Pools 2"));
    clearSeams("__AIJ_POOLS__");
    resetPreviewPools();
  });

  it("has no History tab until a session records events", () => {
    const { container } = shell(mk());
    expect(container.querySelector("#page-tab-history")).toBeNull();
    expect(container.querySelector("#page-tab-actions")).toBeNull();
  });

  it("a running session's header offers Continue in another app…; an ended one's, Relaunch", () => {
    const running = shell(mk());
    expect(running.queryByTestId("header-switch")).not.toBeNull();
    expect(running.queryByTestId("header-relaunch")).toBeNull();
    cleanup();
    const ended = shell(mk({ status: "finished" }));
    expect(ended.getByTestId("header-relaunch").textContent).toBe("Relaunch");
  });

  it("an ended session's header: Relaunch, then Resume last, each with its line under it (the 0.4 review, 26)", () => {
    const ended = shell(mk({ status: "finished" }));
    const relaunch = ended.getByTestId("header-relaunch");
    const resume = ended.getByTestId("header-resume");
    // The labels stay (AJ-E2908); Resume, the primary, ends the row.
    expect(resume.textContent).toBe("Resume");
    expect(relaunch.compareDocumentPosition(resume) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(ended.getByTestId("header-relaunch-hint").textContent).toBe("Fresh conversation");
    expect(ended.getByTestId("header-resume-hint").textContent).toBe("Same conversation");
  });

  it("CI #218: Run in AI Juggler sits beside Resume on the row, never stacked in Resume's hint column", () => {
    const ended = shell(mk({ status: "finished", conversation_id: "conv-1" }));
    const resume = ended.getByTestId("header-resume");
    const here = ended.getByTestId("header-run-here");
    // The column holding Resume and its hint holds nothing else.
    const column = resume.parentElement!;
    expect(column.contains(ended.getByTestId("header-resume-hint"))).toBe(true);
    expect(column.contains(here)).toBe(false);
    expect(here.parentElement).toBe(column.parentElement);
  });

  it("AJ-E2908 R1: Resume is the primary, Relaunch the secondary, each saying what it starts", () => {
    const ended = shell(mk({ status: "finished" }));
    const relaunch = ended.getByTestId("header-relaunch");
    const resume = ended.getByTestId("header-resume");
    expect(relaunch.getAttribute("title")).toBe("Relaunch — fresh session, same launch settings and folder.");
    expect(resume.getAttribute("title")).toBe("Resume — same session, same account.");
    // The filled button is the page's one primary (principle 9).
    expect(resume.className).toContain("shadow-glow-accent");
    expect(relaunch.className).not.toContain("shadow-glow-accent");
    expect(relaunch.className).toContain("bg-surface-2");
  });

  it("AJ-E2908 R6: a running session's header has no Resume and no Relaunch", () => {
    for (const status of ["running", "starting", "waiting_user"]) {
      const view = shell(mk({ status }));
      expect(view.queryByTestId("header-resume"), status).toBeNull();
      expect(view.queryByTestId("header-relaunch"), status).toBeNull();
      cleanup();
    }
  });

  it("an ended session says when it ended once, in its state chip (the 0.4 review, 46)", () => {
    const at = NOW - 5_000;
    const ended = shell(mk({ status: "finished", last_activity_ms: at }));
    expect(ended.getByTestId("session-state-chip").textContent).toBe(`Ended ${clock(at)}`);
    expect(ended.queryByTestId("session-ended-stamp")).toBeNull();
    expect(ended.container.textContent!.match(/Ended/g)).toHaveLength(1);
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
    // App's translation, verbatim: predicate → the page showing now.
    const onKey = (event: KeyboardEvent) => {
      if (isHandoverShortcut(event)) {
        event.preventDefault();
        focusHandover();
      }
    };
    window.addEventListener("keydown", onKey);
    fireEvent.keyDown(window, { key: "h", metaKey: true, shiftKey: true });
    window.removeEventListener("keydown", onKey);
    expect(container.querySelector("#page-tab-terminal")!.getAttribute("aria-selected")).toBe("true");
  });

  it("a session that couldn't run through its runtime says why above its terminal (AJ-E2302)", () => {
    const runtime = {
      kind: "terminal", label: null, beta: false, state: null, asks: [], can_steer: false,
      fell_back: "Codex 0.150.0 is older than 0.163.0, the first AI Juggler drives", error: null, limits: [], cut_off: null,
    };
    const { container } = shell(mk({ provider: "codex", runtime }));
    open(container, "terminal");
    expect(container.querySelector('[data-testid="runtime-fell-back"]')?.textContent).toBe(
      "In its terminal: Codex 0.150.0 is older than 0.163.0, the first AI Juggler drives.",
    );
    cleanup();
    const plain = shell(mk());
    open(plain.container, "terminal");
    expect(plain.container.querySelector('[data-testid="runtime-fell-back"]')).toBeNull();
  });
});

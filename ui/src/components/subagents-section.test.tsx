/**
 * AJ-E1520: the inspector's subagent list and the row's count words.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SubtasksInfo } from "../lib/ipc/subtasks";
import type { InstanceInfo } from "../lib/bridge";

const list = vi.fn<(sessionId: string) => Promise<SubtasksInfo>>();
vi.mock("../lib/ipc/subtasks", () => ({ subtasksList: (sessionId: string) => list(sessionId) }));

const { SubagentsSection, subagentsSummary } = await import("./SubagentsSection");
const { subagentsRunning } = await import("./indicator");
const { RailRow } = await import("./WorkflowRail");

const NOW = 1_728_800_000_000;

function session(subtask_count: number | null): InstanceInfo {
  return {
    id: "s1",
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count,
    git_branch: "feat/parsers",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/app",
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
  } as InstanceInfo;
}

const FOUR: SubtasksInfo = {
  live: true,
  items: [
    { id: "t1", label: "Map the parsers", kind: "Explore", state: "done" },
    { id: "t2", label: "Review error handling", kind: "code-reviewer", state: "failed" },
    { id: "t3", label: "Run the slow suite", kind: null, state: "running" },
    { id: "t4", label: "Write tests for the tail reader", kind: "general-purpose", state: "running" },
  ],
};

afterEach(() => {
  cleanup();
  list.mockReset();
});

describe("AJ-E1520 subagents", () => {
  it("lists each subagent with what it was asked and where it stands, behind a disclosure", async () => {
    list.mockResolvedValue(FOUR);
    render(<SubagentsSection sessionId="s1" running={2} />);
    const section = await screen.findByTestId("subagents-section");
    expect(section.tagName).toBe("DETAILS");
    expect(section.hasAttribute("open")).toBe(false);
    expect(within(section).getByText("2 running · 4 in all")).toBeTruthy();
    const rows = within(section).getAllByTestId("subagent-row");
    expect(rows.map((row) => row.textContent)).toEqual([
      "Map the parsersExploredone",
      "Review error handlingcode-reviewerfailed",
      "Run the slow suiterunning",
      "Write tests for the tail readergeneral-purposerunning",
    ]);
    expect(list).toHaveBeenCalledWith("s1");
  });

  it("says a subagent its ended session never saw finish didn't finish", async () => {
    list.mockResolvedValue({ live: false, items: [{ id: "t1", label: "Map the parsers", kind: null, state: "unfinished" }] });
    render(<SubagentsSection sessionId="s1" running={null} />);
    expect(await screen.findByText("didn't finish")).toBeTruthy();
    expect(screen.getByText("1, none running")).toBeTruthy();
  });

  it("shows nothing for a session that started none", async () => {
    list.mockResolvedValue({ live: true, items: [] });
    const { container } = render(<SubagentsSection sessionId="s1" running={null} />);
    await waitFor(() => expect(list).toHaveBeenCalled());
    expect(container.innerHTML).toBe("");
  });

  it("reads the list again when the row's count moves", async () => {
    list.mockResolvedValue(FOUR);
    const { rerender } = render(<SubagentsSection sessionId="s1" running={2} />);
    await screen.findByTestId("subagents-section");
    expect(list).toHaveBeenCalledTimes(1);
    list.mockResolvedValue({ ...FOUR, items: FOUR.items.map((item) => ({ ...item, state: item.state === "running" ? "done" : item.state })) });
    rerender(<SubagentsSection sessionId="s1" running={0} />);
    expect(await screen.findByText("4, none running")).toBeTruthy();
    expect(list).toHaveBeenCalledTimes(2);
  });

  it("the rail row notes running subagents quietly, and only when there are some", () => {
    const { rerender } = render(<RailRow session={session(2)} selected={false} onSelect={vi.fn()} density="comfortable" now={NOW} />);
    const note = screen.getByTestId("rail-subagents");
    expect(note.textContent).toBe("· 2 subagents");
    expect(note.getAttribute("title")).toBe("2 subagents running");
    expect(screen.getByRole("button").textContent).toContain(", 2 subagents running");
    rerender(<RailRow session={session(1)} selected={false} onSelect={vi.fn()} density="comfortable" now={NOW} />);
    expect(screen.getByTestId("rail-subagents").textContent).toBe("· 1 subagent");
    for (const none of [0, null]) {
      rerender(<RailRow session={session(none)} selected={false} onSelect={vi.fn()} density="comfortable" now={NOW} />);
      expect(screen.queryByTestId("rail-subagents")).toBeNull();
    }
  });

  it("words the count as subagents running", () => {
    expect(subagentsRunning(1)).toBe("1 subagent running");
    expect(subagentsRunning(3)).toBe("3 subagents running");
    expect(subagentsSummary(FOUR.items)).toBe("2 running · 4 in all");
  });
});

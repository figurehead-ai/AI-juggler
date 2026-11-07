/**
 * AJ-E2811 R1 and AJ-E2810 R8: a subagent's page goes back one step on
 * Esc, and / goes to the list's search.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/subagents";
import type { SubagentDetailInfo } from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { SubagentsTab } from "./SubagentsTab";
import { NOW, TREE, agent, listed, model } from "./subagents.test-kit";

afterEach(() => {
  cleanup();
  forgetPaged();
  vi.restoreAllMocks();
});

describe("a subagent's page and the list's keys", () => {
  it("Esc goes back one step: from a parent opened on a subagent's page, to that page, then the list (AJ-E2811 R1)", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    const read = vi.spyOn(ipc, "subagentsDetail").mockImplementation((_, __, id) =>
      Promise.resolve({
        state: "shown",
        reason: null,
        agent: agent(id, { parent: id === "n1" ? "a1" : null }),
        chain: id === "n1" ? [{ id: "a1", title: "Audit the cart store" }] : [],
        prompt: null,
        result: null,
        failure: null,
        messages_off: false,
        asked: null,
        tools: null,
        lines: null,
        files: [],
        files_total: 0,
        children: [],
        children_total: 0,
        tokens: null,
        counted: "in_session",
        cost_usd: null,
        share_pct: null,
        processes: [],
        process_session: null,
        commands: 0,
        stop_hint: null,
        worktree: null,
        branch: null,
        stopped_by_user: null,
        tool: "Claude Code",
      } as SubagentDetailInfo),
    );
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    fireEvent.click((await screen.findAllByTestId("subagent-row"))[1]!);
    expect((await screen.findByTestId("subagent-title")).textContent).toBe("Task n1");
    // its parent, from the breadcrumb
    fireEvent.click(within(screen.getByRole("navigation", { name: "Where you are" })).getByRole("button", { name: "Audit the cart store" }));
    await waitFor(() => expect(screen.getByTestId("subagent-title").textContent).toBe("Task a1"));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.getByTestId("subagent-title").textContent).toBe("Task n1"));
    expect(read.mock.calls.map((call) => call[2])).toEqual(["n1", "a1", "n1"]);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(await screen.findByTestId("session-subagents")).toBeTruthy();
  });

  it("a quiet running one says when it last did something; its processes box says how it matches and links the tab (AJ-E2809 R8, AJ-E2811 R7, R9)", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    vi.spyOn(ipc, "subagentsDetail").mockResolvedValue({
      state: "shown",
      reason: null,
      agent: agent("n1", { status: "running", ended_ms: null, duration_ms: 600_000, last_activity_ms: NOW - 5 * 60_000 }),
      chain: [],
      prompt: null,
      result: null,
      failure: null,
      messages_off: false,
      asked: null,
      tools: null,
      lines: null,
      files: [],
      files_total: 0,
      children: [],
      children_total: 0,
      tokens: null,
      counted: "in_session",
      cost_usd: null,
      share_pct: null,
      processes: [],
      process_session: null,
      commands: 4,
      stop_hint: null,
      worktree: null,
      branch: null,
      stopped_by_user: null,
      tool: "Claude Code",
    });
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    fireEvent.click((await screen.findAllByTestId("subagent-row"))[1]!);
    expect((await screen.findByTestId("subagent-last-activity")).textContent).toBe("last activity 5 min ago");
    const box = screen.getByTestId("subagent-processes-box").textContent ?? "";
    expect(box).toContain("No process matched its commands.");
    expect(box).toContain("Every subagent runs inside its tool's one process");
    const asked: string[] = [];
    const hear = (event: Event) => {
      if (event instanceof CustomEvent) asked.push(String(event.detail));
    };
    window.addEventListener("aij:page-tab", hear);
    fireEvent.click(screen.getByTestId("subagent-processes-tab"));
    window.removeEventListener("aij:page-tab", hear);
    expect(asked).toEqual(["processes"]);
  });

  it("a parent's own tokens are in what a screen reader hears, not only on hover (AJ-E2810 R5)", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    const parent = (await screen.findAllByTestId("subagent-row"))[0]!;
    expect(parent.textContent).toContain("412k, its own 390k");
  });

  it("/ goes to the search from the list (AJ-E2810 R8)", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    await screen.findAllByTestId("subagent-row");
    fireEvent.keyDown(document.body, { key: "/" });
    expect(document.activeElement).toBe(screen.getByTestId("subagents-search"));
  });

});

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

  it("/ goes to the search from the list (AJ-E2810 R8)", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    await screen.findAllByTestId("subagent-row");
    fireEvent.keyDown(document.body, { key: "/" });
    expect(document.activeElement).toBe(screen.getByTestId("subagents-search"));
  });

});

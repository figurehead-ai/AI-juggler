/**
 * AJ-E2810 R8 and AJ-E2811 R8: the Subagents tree from the keyboard (↓
 * moves, ↩ opens a subagent's page), and a tool with no
 * way to stop one subagent says what stops them all, with no Stop button.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { SubagentsTab } from "./SubagentsTab";
import { NOW, TREE, agent, detailOf, listed, model } from "./subagents.test-kit";

afterEach(() => {
  cleanup();
  forgetPaged();
  vi.restoreAllMocks();
});

const focused = () => document.activeElement?.getAttribute("data-row-index");

describe("the Subagents tree from the keyboard (AJ-E2810 R8)", () => {
  it("↓ moves from its group to the subagent under it, and ↩ opens that subagent's page", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    const read = vi.spyOn(ipc, "subagentsDetail").mockImplementation((_, __, id) => Promise.resolve(detailOf(agent(id))));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    await screen.findAllByTestId("subagent-row");
    const tree = screen.getByRole("treegrid");
    const first = tree.querySelector<HTMLElement>('[data-row-index="0"]')!;
    first.focus();
    fireEvent.keyDown(first, { key: "ArrowDown" });
    await waitFor(() => expect(focused()).toBe("1"));
    fireEvent.keyDown(document.activeElement!, { key: "Enter" });
    await waitFor(() => expect(read).toHaveBeenCalledWith("work", "c1", "a1"));
  });
});

describe("Stop, when the tool has no way to stop one (AJ-E2811 R8)", () => {
  it("says stopping the session stops them all, and offers no Stop button", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    vi.spyOn(ipc, "subagentsDetail").mockResolvedValue(detailOf(agent("a1", { status: "running", ended_ms: null })));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    fireEvent.click((await screen.findAllByTestId("subagent-row"))[0]!);
    const stop = await screen.findByTestId("subagent-stop-box");
    expect(stop.textContent).toContain("AI Juggler can't stop one subagent inside a Kit Tool session it doesn't drive. Stopping the session stops all of them.");
    expect(stop.querySelector("button")).toBeNull();
    // It ran no commands: no "Stop it" link where nothing was started.
    expect(screen.getByTestId("subagent-processes-box").textContent).toContain("It ran no commands, so it started no processes.");
    expect(screen.queryByTestId("subagent-processes-tab")).toBeNull();
  });
});

/**
 * AJ-E1520 (0.2) vitest — a session page's Subagents tab: shown only when
 * its tool records subagents and it started some; a skeleton at once;
 * rows newest first with the nested ones indented under their parent, the
 * status as a word, the whole task on hover, the background badge; a page
 * at a time; a row opens its conversation in the same chat view, with a
 * way back; a tool that keeps none says why.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/subagents";
import type { SubagentInfo, SubagentsInfo } from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { pageTabs } from "../registry";
import { SubagentsTab, durationWords } from "./SubagentsTab";
import { useHasSubagents } from "./useSubagents";
import type { SessionModel } from "./model";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  forgetPaged();
});

const NOW = 1_792_600_000_000;

const model: SessionModel = {
  tool: "claude-code",
  profileId: "work",
  profileLabel: "Work",
  conversationId: "c1",
  switchSource: "conversation:work:c1",
  sessionId: null,
  live: true,
  state: { words: "Working", tone: "live", asks: false, working: true, since: 0 },
  places: [],
  viewedMs: null,
  cwd: "/w/app",
  model: null,
};

const agent = (id: string, extra: Partial<SubagentInfo> = {}): SubagentInfo => ({
  id,
  kind: "frontend-worker",
  task: `Task ${id}`,
  status: "done",
  started_ms: NOW - 600_000,
  ended_ms: NOW - 360_000,
  parent: null,
  depth: 1,
  background: true,
  has_conversation: true,
  ...extra,
});

const listed = (items: SubagentInfo[], extra: Partial<SubagentsInfo> = {}, next: string | null = null, total: number | null = null): SubagentsInfo => ({
  state: "listed",
  reason: null,
  page: { items, next_cursor: next, total: total ?? items.length },
  running: items.filter((item) => item.status === "running").length,
  reading: false,
  live: true,
  conversations_note: null,
  ...extra,
});

describe("AJ-E1520 the Subagents tab", () => {
  it("shows only when the session started some, for any session page", () => {
    expect(pageTabs("session", { hasSubagents: true }).map((tab) => tab.id)).toContain("subagents");
    expect(pageTabs("session", {}).map((tab) => tab.id)).not.toContain("subagents");
    expect(pageTabs("conversation", { hasSubagents: true }).map((tab) => tab.id)).toEqual(["overview", "subagents"]);
    expect(pageTabs("conversation", {}).map((tab) => tab.id)).toEqual(["overview"]);
  });

  it("the page asks once whether there are any, and a tool that keeps none hides it", async () => {
    const list = vi.spyOn(ipc, "subagentsList").mockResolvedValueOnce(listed([agent("a1")])).mockResolvedValueOnce({ ...listed([]), state: "unsupported", reason: "No record." });
    const Probe = ({ id }: { id: string }) => <span data-testid="has">{String(useHasSubagents("work", id, false))}</span>;
    const view = render(<Probe id="c1" />);
    await waitFor(() => expect(screen.getByTestId("has").textContent).toBe("true"));
    expect(list).toHaveBeenCalledWith("work", "c1", { cursor: null, limit: 1 });
    view.rerender(<Probe id="c2" />);
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(screen.getByTestId("has").textContent).toBe("false");
  });

  it("lists them newest first with a skeleton at once, nested under their parent, words not colour alone", async () => {
    let answer: (info: SubagentsInfo) => void = () => undefined;
    vi.spyOn(ipc, "subagentsList").mockImplementation(() => new Promise((resolve) => (answer = resolve)));
    render(<SubagentsTab model={model} visible now={NOW} />);
    expect(screen.getByTestId("subagents-loading")).toBeTruthy();
    const long = "Port the settings panes to the new layout and keep every old shortcut working, then write the tests";
    await act(async () =>
      answer(
        listed([
          agent("a2", { status: "running", ended_ms: null, task: long, started_ms: NOW - 120_000 }),
          agent("n1", { parent: "a2", depth: 2, kind: "Explore", background: false, status: "failed" }),
          agent("a1", { status: "unknown", ended_ms: null, kind: null, task: null, has_conversation: false, background: null }),
        ]),
      ),
    );
    const rows = screen.getAllByTestId("subagent-row");
    expect(rows.map((row) => row.getAttribute("data-status"))).toEqual(["running", "failed", "unknown"]);
    expect(rows[1]!.getAttribute("data-depth")).toBe("2");
    expect(within(rows[0]!).getByTestId("subagent-task").getAttribute("title")).toBe(long);
    expect(within(rows[0]!).getByTestId("subagent-status").textContent).toBe("Running");
    expect(within(rows[0]!).getByTestId("subagent-background").textContent).toBe("in the background");
    expect(within(rows[0]!).getByText("running 2m")).toBeTruthy();
    expect(within(rows[1]!).queryByTestId("subagent-background")).toBeNull();
    expect(within(rows[2]!).getByTestId("subagent-task").textContent).toBe("A subagent");
    // Without a conversation of its own it isn't a button.
    expect(within(rows[2]!).queryByRole("button")).toBeNull();
    expect(screen.getByTestId("subagents-summary").textContent).toBe("3 subagents · 1 running");
  });

  it("reads a page at a time", async () => {
    const list = vi
      .spyOn(ipc, "subagentsList")
      .mockResolvedValueOnce(listed([agent("a3"), agent("a2")], {}, "k1:next", 3))
      .mockResolvedValueOnce(listed([agent("a1")], {}, null, 3));
    render(<SubagentsTab model={model} visible now={NOW} />);
    await waitFor(() => expect(screen.getAllByTestId("subagent-row")).toHaveLength(2));
    fireEvent.click(screen.getByTestId("subagents-more"));
    await waitFor(() => expect(screen.getAllByTestId("subagent-row")).toHaveLength(3));
    expect(list.mock.calls[1]![2]).toEqual({ cursor: "k1:next", limit: 50 });
  });

  it("a row opens its conversation in the chat view, and back returns to the list", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed([agent("a1")]));
    const read = vi.spyOn(ipc, "subagentsConversation").mockResolvedValue({
      state: "shown",
      redacted: 0,
      earlier: null,
      stamp: "s",
      turns: [
        { speaker: "you", blocks: [{ kind: "text", text: "Find the parser", steps: [], hidden: 0 }], at_ms: null, model: null },
        { speaker: "assistant", blocks: [{ kind: "text", text: "Found it in src/parse.ts", steps: [], hidden: 0 }], at_ms: null, model: null },
      ],
    });
    render(<SubagentsTab model={model} visible now={NOW} />);
    const row = (await screen.findAllByTestId("subagent-row"))[0]!;
    fireEvent.click(within(row).getByRole("button"));
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    expect(read).toHaveBeenCalledWith("work", "c1", "a1", undefined);
    expect(screen.getByText("The session asked")).toBeTruthy();
    // It asks no one: no question card, no Go to it.
    expect(screen.queryByTestId("last-messages-asks")).toBeNull();
    fireEvent.click(screen.getByTestId("subagent-back"));
    expect(await screen.findByTestId("session-subagents")).toBeTruthy();
  });

  it("a tool that keeps none says why, and a long session says it is still reading", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValueOnce({ ...listed([]), state: "unsupported", reason: "This tool keeps no record of its subagents." });
    render(<SubagentsTab model={model} visible now={NOW} />);
    expect((await screen.findByTestId("subagents-unlisted")).textContent).toBe("This tool keeps no record of its subagents.");
    cleanup();
    forgetPaged();
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed([agent("a1")], { reading: true, conversations_note: "Kimi Code keeps it as a journal." }));
    render(<SubagentsTab model={model} visible now={NOW} />);
    expect(await screen.findByTestId("subagents-reading")).toBeTruthy();
    expect(screen.getByTestId("subagents-note").textContent).toContain("journal");
  });

  it("says how long each took, and never a time it doesn't know", () => {
    expect(durationWords(agent("a"), NOW)).toBe("took 4m");
    expect(durationWords(agent("a", { status: "running", ended_ms: null, started_ms: NOW - 30_000 }), NOW)).toBe("running 30s");
    expect(durationWords(agent("a", { status: "unknown", ended_ms: null }), NOW)).toBeNull();
    expect(durationWords(agent("a", { started_ms: null }), NOW)).toBeNull();
  });
});

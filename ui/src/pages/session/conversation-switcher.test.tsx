/**
 * 0.3.4: the Overview's switcher. Beside a session's conversation, on its
 * left: its subagents running now, the sessions it started and the latest
 * that ended; one click shows that conversation in place, Esc (or Back)
 * comes back, and the session's own pane is never unmounted meanwhile (a
 * terminal is never rebuilt). Nothing besides the conversation: no list.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as subagents from "../../lib/ipc/subagents";
import * as conversations from "../../lib/ipc/conversations";
import type { SubagentInfo, SubagentRowInfo, SubagentsInfo } from "../../lib/ipc/subagents";
import type { SessionModel } from "./model";

vi.mock("./useSubagents", () => ({
  useHeardSubagentCounts: () => counts,
}));
let counts: { total: number; running: number } | null = null;

const { ConversationSwitcher } = await import("./ConversationSwitcher");

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  counts = null;
});

const NOW = Date.now();

const model: SessionModel = {
  tool: "claude-code",
  profileId: "work",
  profileLabel: "Work",
  conversationId: "c1",
  sessionId: "s1",
  title: "Fix the totals",
  live: true,
  places: [],
  state: { words: "Working", tone: "live", asks: false, working: true, since: NOW },
  viewedMs: null,
  cwd: "/w/app",
  model: null,
} as unknown as SessionModel;

const agent = (id: string, status: string, task: string): SubagentInfo =>
  ({ id, kind: "Explore", task, status, started_ms: NOW - 60_000, ended_ms: status === "running" ? null : NOW, duration_ms: 1000, parent: null, depth: 1, background: true, has_conversation: true, model: null, tokens: null, own_tokens: null, children: 0 }) as unknown as SubagentInfo;

const page = (agents: SubagentInfo[]): SubagentsInfo =>
  ({
    state: "listed",
    reason: null,
    page: { items: agents.map((a) => ({ key: `a:${a.id}`, row: "agent", group: null, agent: a, guides: [], last: false, context: false }) as SubagentRowInfo), next_cursor: null, total: agents.length },
    summary: { total: 0, running: 0, failed: 0, stopped: 0, unknown: 0, attention: 0, tokens: null, share_pct: null, counted: "none" },
    kinds: [],
    matched: agents.length,
    running: 0,
    reading: false,
    live: true,
    conversations_note: null,
    tool: "Claude Code",
  }) as SubagentsInfo;

function seed(running: SubagentInfo[], done: SubagentInfo[], started: conversations.StartedSessionInfo[] = []) {
  counts = { total: running.length + done.length, running: running.length };
  vi.spyOn(subagents, "subagentsList").mockImplementation(async (_a, _c, _p, query) => page(query?.view === "running" ? running : done));
  vi.spyOn(subagents, "subagentsConversation").mockResolvedValue({ state: "shown", reason: null, turns: [], earlier: null, redacted: 0, stamp: null } as never);
  vi.spyOn(conversations, "conversationsStarted").mockResolvedValue(started);
}

describe("0.3.4 the Overview's switcher", () => {
  it("shows no list when there's nothing besides the conversation", async () => {
    seed([], []);
    render(<ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />);
    await act(async () => undefined);
    expect(screen.getByTestId("own")).toBeTruthy();
    expect(screen.queryByTestId("switcher-list")).toBeNull();
  });

  it("drills into a running subagent in place, keeps the session's own pane mounted, and Esc comes back", async () => {
    seed([agent("a1", "running", "Read the tax tables")], [agent("a2", "done", "Plan the split")]);
    let mounts = 0;
    const Own = () => {
      // Counts mounts: a terminal must never be rebuilt.
      useState(() => (mounts += 1));
      return <p data-testid="own">own</p>;
    };
    render(<ConversationSwitcher model={model} visible own={<Own />} ownWords="Its terminal" />);
    const list = await screen.findByTestId("switcher-list");
    expect(within(list).getByTestId("switcher-own").textContent).toContain("Its terminal");
    expect(within(screen.getByTestId("switcher-running")).getAllByTestId("switcher-subagent")).toHaveLength(1);
    expect(screen.getByTestId("switcher-done-toggle").textContent).toContain("Done · 1");
    fireEvent.click(within(screen.getByTestId("switcher-running")).getByTestId("switcher-subagent"));
    await waitFor(() => expect(screen.getByTestId("switcher-back")).toBeTruthy());
    expect(screen.getByTestId("switcher-own-pane").hidden).toBe(true);
    expect(screen.getByTestId("own")).toBeTruthy();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.getByTestId("switcher-own-pane").hidden).toBe(false));
    expect(screen.queryByTestId("switcher-back")).toBeNull();
    expect(mounts).toBe(1);
  });

  it("lists the sessions it started and shows one in place, with its page a click away", async () => {
    seed([], [agent("a2", "done", "Plan the split")], [{ tool: "claude-code", conversation_id: "kid", title: "Nightly: skip docs-only", last_activity_ms: NOW - 60_000 }]);
    render(<ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />);
    const item = await screen.findByTestId("switcher-started-session");
    expect(item.textContent).toContain("Nightly: skip docs-only");
    fireEvent.click(item);
    // Not in this computer's list of conversations (none in the test): said so.
    expect((await screen.findByTestId("switcher-started-missing")).textContent).toContain("isn't in this computer's list");
  });
});

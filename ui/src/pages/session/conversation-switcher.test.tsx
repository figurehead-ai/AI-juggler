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
import { resetConversationStore } from "../../lib/conversation-store";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { placesConversations } from "../../preview/places";
import { clearSeams, setSeam } from "../../test/fixtures";

vi.mock("./useSubagents", () => ({
  useHeardSubagentCounts: () => counts,
}));
let counts: { total: number; running: number } | null = null;

const { ConversationSwitcher } = await import("./ConversationSwitcher");

afterEach(() => {
  resetConversationStore();
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
  vi.spyOn(subagents, "subagentsFollow").mockResolvedValue({ state: "none", subscription: null, generation: 0, revision: 0, layout: "stretches", turns: [], earlier: null, status: null, skipped: 0 });
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
    expect(screen.getByTestId("switcher-done-toggle").textContent).toContain("Finished subagents · 1");
    // 0.3.5: a tree that says what each branch is, the session its root.
    expect(screen.getByTestId("switcher-own").textContent).toContain("This session");
    expect(screen.getByTestId("switcher-running").textContent).toContain("Subagents running");
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

  it("lists the sessions it started ten at a time, the rest a click away (AJ-E3106 R5)", async () => {
    const started = Array.from({ length: 23 }, (_, n) => ({ tool: "claude-code", conversation_id: `kid-${n}`, title: `Started ${n}`, last_activity_ms: NOW - n * 60_000 }));
    seed([], [], started);
    render(<ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />);
    await waitFor(() => expect(screen.getAllByTestId("switcher-started-session")).toHaveLength(10));
    const more = screen.getByTestId("switcher-started-more");
    expect(more.textContent).toBe("Show 10 more of 13");
    fireEvent.click(more);
    fireEvent.click(screen.getByTestId("switcher-started-more"));
    expect(screen.getAllByTestId("switcher-started-session")).toHaveLength(23);
    expect(screen.queryByTestId("switcher-started-more")).toBeNull();
  });

  it("more running than it lists: a last line opens them all in Subagents (AJ-E3106 R5)", async () => {
    seed([agent("r1", "running", "Read the tables"), agent("r2", "running", "Run the tests")], []);
    counts = { total: 42, running: 42 };
    const asked: string[] = [];
    const hear = (event: Event) => event instanceof CustomEvent && asked.push(String(event.detail));
    window.addEventListener("aij:page-tab", hear);
    render(<ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />);
    const all = await screen.findByTestId("switcher-running-all");
    expect(all.textContent).toBe("All 42 running in Subagents ›");
    fireEvent.click(all);
    window.removeEventListener("aij:page-tab", hear);
    expect(asked).toEqual(["subagents"]);
  });

  it("the finished ones' last line opens the Subagents tab (AJ-E3106 R1)", async () => {
    seed([], [agent("d1", "done", "Plan the split")]);
    const asked: string[] = [];
    const hear = (event: Event) => event instanceof CustomEvent && asked.push(String(event.detail));
    window.addEventListener("aij:page-tab", hear);
    render(<ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />);
    fireEvent.click(await screen.findByTestId("switcher-done-toggle"));
    fireEvent.click(screen.getByTestId("switcher-all"));
    window.removeEventListener("aij:page-tab", hear);
    expect(asked).toEqual(["subagents"]);
  });

  it("a started session on this computer's list shows in place and opens its page (AJ-E3106 R2)", async () => {
    const kid = placesConversations(NOW)[0]!;
    setSeam("__AIJ_FIXTURE__", { conversations: [kid] });
    seed([], [], [{ tool: kid.tool, conversation_id: kid.id, title: "Nightly: skip docs-only", last_activity_ms: NOW }]);
    const openConversation = vi.fn();
    render(
      <ConversationsProvider>
        <ActionHostProvider host={{ openConversation }}>
          <ConversationSwitcher model={model} visible own={<p data-testid="own">own</p>} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    fireEvent.click(await screen.findByTestId("switcher-started-session"));
    fireEvent.click(await screen.findByTestId("switcher-open-page"));
    expect(openConversation).toHaveBeenCalledWith(expect.objectContaining({ id: kid.id, tool: kid.tool }));
    clearSeams("__AIJ_FIXTURE__");
  });
});

/**
 * AJ-E1520 vitest — a session page's Subagents tab (0.2: shown only when
 * its tool records subagents and it started some; 0.3: what's running,
 * what failed, what it cost first). The summary line and its links, the
 * views (opening on Running when anything runs), the search, type and
 * order sent as the query, groups under the message that started them
 * with real tree lines, a skeleton at once, a subagent's own page (and
 * Esc back), its conversation, the Usage tab's line, and the session's
 * quiet line to the tab.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/subagents";
import type { SubagentDetailInfo, SubagentInfo, SubagentRowInfo, SubagentsInfo } from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { pageTabs } from "../registry";
import { SubagentsTab } from "./SubagentsTab";
import { useHasSubagents, useHeardSubagentCounts } from "./useSubagents";
import type { SessionModel } from "./model";
import { SessionNotes, subagentsLineWords } from "../../components/SessionNotes";
import { MOCK_SESSIONS } from "../../lib/bridge";
import { pick } from "../../test/pick";

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
  kind: "general-purpose",
  task: `Task ${id}`,
  status: "done",
  started_ms: NOW - 600_000,
  ended_ms: NOW - 360_000,
  duration_ms: 240_000,
  last_activity_ms: null,
  parent: null,
  depth: 1,
  background: true,
  has_conversation: true,
  tokens: 12_000,
  own_tokens: 12_000,
  model: "claude-haiku-4-5",
  children: 0,
  session: null,
  ...extra,
});

const row = (a: SubagentInfo, guides: boolean[] = [], last = false, context = false): SubagentRowInfo => ({ key: `a:${a.id}`, row: "agent", group: null, agent: a, guides, last, context });

const group = (key: string, asked: string | null, count: number, open = true): SubagentRowInfo => ({
  key: `g:${key}`,
  row: "group",
  agent: null,
  guides: [],
  last: false,
  context: false,
  group: { key, asked, at_ms: NOW - 900_000, count, running: 1, failed: 0, tokens: 1_200_000, open, session: null },
});

const listed = (items: SubagentRowInfo[], extra: Partial<SubagentsInfo> = {}): SubagentsInfo => {
  const agents = items.filter((r) => r.agent).map((r) => r.agent!);
  const running = agents.filter((a) => a.status === "running").length;
  return {
    state: "listed",
    reason: null,
    page: { items, next_cursor: null, total: items.length },
    summary: { total: agents.length, running, failed: agents.filter((a) => a.status === "failed").length, attention: agents.filter((a) => a.status === "failed").length, tokens: 36_000, share_pct: 41, counted: "in_session" },
    kinds: [
      { kind: "general-purpose", count: 2 },
      { kind: "Explore", count: 1 },
    ],
    matched: agents.length,
    running,
    reading: false,
    live: true,
    conversations_note: null,
    tool: "Claude Code",
    ...extra,
  };
};

const TREE = [
  group("u1", "fix the totals and add tests", 3),
  row(agent("a1", { status: "running", ended_ms: null, task: "Audit the cart store", children: 1, tokens: 412_000, own_tokens: 390_000 })),
  row(agent("n1", { parent: "a1", depth: 2, kind: "Explore", background: false, status: "failed" }), [true], true),
  row(agent("a2", { task: "Review the diff" }), [], true),
  group("u0", "plan the totals epic", 41, false),
];

describe("AJ-E1520 the Subagents tab", () => {
  it("shows only when the session started some, for any session page", () => {
    expect(pageTabs("session", { hasSubagents: true }).map((tab) => tab.id)).toContain("subagents");
    expect(pageTabs("session", {}).map((tab) => tab.id)).not.toContain("subagents");
    expect(pageTabs("conversation", { hasSubagents: true }).map((tab) => tab.id)).toEqual(["overview", "subagents"]);
  });

  it("the page asks once whether there are any (the summary's count), and a tool that keeps none hides it", async () => {
    const list = vi.spyOn(ipc, "subagentsList").mockResolvedValueOnce(listed(TREE)).mockResolvedValueOnce({ ...listed([]), state: "unsupported", reason: "No record." });
    const Probe = ({ id }: { id: string }) => <span data-testid="has">{String(useHasSubagents("work", id, false))}</span>;
    const view = render(<Probe id="c1" />);
    await waitFor(() => expect(screen.getByTestId("has").textContent).toBe("true"));
    expect(list).toHaveBeenCalledWith("work", "c1", { cursor: null, limit: 1 });
    view.rerender(<Probe id="c2" />);
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(screen.getByTestId("has").textContent).toBe("false");
  });

  it("says what runs, failed and costs first, groups under the message, with real tree lines", async () => {
    let answer: (info: SubagentsInfo) => void = () => undefined;
    vi.spyOn(ipc, "subagentsList").mockImplementation(() => new Promise((resolve) => (answer = resolve)));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    expect(screen.getByTestId("subagents-loading")).toBeTruthy();
    await act(async () => answer(listed(TREE)));
    expect(screen.getByTestId("subagents-summary").textContent).toBe("3 subagents·1 running·1 failed·41% of this session's tokens");
    const groups = screen.getAllByTestId("subagent-group");
    expect(groups.map((g) => g.getAttribute("data-open"))).toEqual(["true", "false"]);
    expect(groups[0]!.textContent).toContain("“fix the totals and add tests”");
    const rows = screen.getAllByTestId("subagent-row");
    expect(rows.map((r) => r.getAttribute("data-status"))).toEqual(["running", "failed", "done"]);
    // Lines for who started whom; the message's own (the first level) none.
    expect(rows.map((r) => r.textContent?.slice(0, 2))).toEqual(["Au", "└ ", "Re"]);
    expect(rows[1]!.textContent?.startsWith("└ Task n1")).toBe(true);
    expect(within(rows[0]!).getByTestId("subagent-status").textContent).toBe("Running");
    expect(rows[0]!.textContent).toContain("412k");
    expect(rows[0]!.textContent).toContain("Haiku 4.5");
    expect(rows[0]!.textContent).toContain("General purpose");
    expect(screen.getByRole("treegrid", { name: "Subagents" })).toBeTruthy();
  });

  it("sends the view, search, type, order and opened groups as the query", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const list = vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    await screen.findAllByTestId("subagent-row");
    const last = () => list.mock.calls[list.mock.calls.length - 1]![3]!;
    fireEvent.click(screen.getByTestId("subagents-summary-failed"));
    await waitFor(() => expect(last().view).toBe("attention"));
    fireEvent.click(screen.getByRole("radio", { name: "All" }));
    fireEvent.change(screen.getByTestId("subagents-search"), { target: { value: "file:tax.ts" } });
    await act(async () => vi.advanceTimersByTime(250));
    await waitFor(() => expect(last().search).toBe("file:tax.ts"));
    pick(screen.getByTestId("subagents-sort"), "Most tokens");
    await waitFor(() => expect(last().sort).toBe("tokens"));
    pick(screen.getByTestId("subagents-type"), "Explore");
    await waitFor(() => expect(last().kind).toBe("Explore"));
    fireEvent.click(screen.getAllByTestId("subagent-group")[1]!);
    await waitFor(() => expect(last().open).toEqual(["u0"]));
  });

  it("opens on Running when anything runs, and says plainly when nothing matches", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValueOnce(listed(TREE)).mockResolvedValue({ ...listed([]), summary: listed(TREE).summary });
    const Page = () => {
      useHasSubagents("work", "c-run", false);
      const heard = useHeardSubagentCounts("work", "c-run");
      return heard ? <SubagentsTab model={{ ...model, conversationId: "c-run" }} title="Fix the cart totals" visible now={NOW} /> : null;
    };
    render(<Page />);
    const running = await screen.findByRole("radio", { name: "Running 1" });
    expect(running.getAttribute("aria-checked")).toBe("true");
    expect((await screen.findByTestId("subagents-none-match")).textContent).toContain("Nothing is running now.");
  });

  it("a subagent's page: its task, result, work, usage, processes and how to stop it; Esc goes back", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    const detail: SubagentDetailInfo = {
      state: "shown",
      reason: null,
      agent: agent("n1", { parent: "a1", status: "running", ended_ms: null, duration_ms: 60_000 }),
      chain: [{ id: "a1", title: "Audit the cart store" }],
      prompt: "Find every place the cart reads tax rates.",
      result: null,
      failure: null,
      messages_off: false,
      asked: "fix the totals",
      tools: { total: 38, reads: 12, searches: 9, commands: 6, edits: 4, other: 7 },
      lines: { added: 120, removed: 34 },
      files: ["/w/app/tax.ts", "/w/app/cart/store.ts", "/w/app/a.ts", "/w/app/b.ts", "/w/app/c.ts", "/w/app/d.ts"],
      files_total: 6,
      children: [],
      children_total: 0,
      tokens: { input: 1_000, output: 2_000, cache_read: 18_000, cache_write: 1_000, reasoning: 0 },
      counted: "in_session",
      cost_usd: 0.04,
      share_pct: 0.6,
      processes: [{ key: "p:9:1", pid: 9, name: "node (vite)", command: "", rss_bytes: 212_000_000, cpu_pct: 3, alive: true }],
      process_session: "c1",
      commands: 6,
      stop_hint: "In Claude Code: /tasks, select it, press x.",
      worktree: null,
      branch: null,
      stopped_by_user: null,
      tool: "Claude Code",
    };
    const read = vi.spyOn(ipc, "subagentsDetail").mockResolvedValue(detail);
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    fireEvent.click((await screen.findAllByTestId("subagent-row"))[1]!);
    expect((await screen.findByTestId("subagent-title")).textContent).toBe("Task n1");
    expect(read).toHaveBeenCalledWith("work", "c1", "n1");
    // The breadcrumb starts at the session, by its own title.
    expect(screen.getByRole("navigation", { name: "Where you are" }).textContent).toMatch(/^Fix the cart totals›Subagents›/);
    expect(screen.getByTestId("subagent-detail-status").textContent).toBe("Running · 1m");
    expect(screen.getByTestId("subagent-prompt").textContent).toBe("Find every place the cart reads tax rates.");
    expect(screen.getByTestId("subagent-result-box").textContent).toContain("It's still running.");
    const work = screen.getByTestId("subagent-work-box").textContent!;
    expect(work).toContain("38 · 12 reads · 9 searches · 6 commands · 4 edits · 7 other");
    expect(work).toContain("+120 −34");
    expect(work).toContain("tax.ts, store.ts, a.ts, b.ts, c.ts, +1 more");
    expect(screen.getByTestId("subagent-usage-box").textContent).toContain("~$0.04");
    expect(screen.getByTestId("subagent-usage-box").textContent).toContain("0.6%");
    expect(screen.getByTestId("subagent-stop-box").textContent).toContain("/tasks, select it, press x");
    const asked: string[] = [];
    const hear = (event: Event) => asked.push(String((event as CustomEvent<string>).detail));
    window.addEventListener("aij:page-tab", hear);
    fireEvent.click(screen.getByTestId("subagent-process"));
    window.removeEventListener("aij:page-tab", hear);
    expect(asked).toEqual(["processes"]);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(await screen.findByTestId("session-subagents")).toBeTruthy();
  });

  it("from a subagent's page, its conversation, and back", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    vi.spyOn(ipc, "subagentsDetail").mockResolvedValue({ ...(await ipc.subagentsDetail("x", "y", "z")), state: "shown", agent: agent("a2") });
    const read = vi.spyOn(ipc, "subagentsConversation").mockResolvedValue({
      state: "shown",
      redacted: 0,
      earlier: null,
      stamp: "s",
      turns: [{ speaker: "you", blocks: [{ kind: "text", text: "Review it", steps: [], hidden: 0 }], at_ms: null, model: null }],
    });
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    fireEvent.click((await screen.findAllByTestId("subagent-row"))[2]!);
    fireEvent.click(await screen.findByTestId("subagent-open-conversation"));
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(1));
    expect(read).toHaveBeenCalledWith("work", "c1", "a2", undefined);
    fireEvent.click(screen.getByTestId("subagent-back"));
    expect(await screen.findByTestId("subagent-detail")).toBeTruthy();
  });

  it("a tool that keeps none says why, and a long session says it is still reading", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValueOnce({ ...listed([]), state: "unsupported", reason: "This tool keeps no record of its subagents." });
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    expect((await screen.findByTestId("subagents-unlisted")).textContent).toBe("This tool keeps no record of its subagents.");
    cleanup();
    forgetPaged();
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE, { reading: true, conversations_note: "Kimi Code keeps it as a journal." }));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    expect(await screen.findByTestId("subagents-reading")).toBeTruthy();
    expect(screen.getByTestId("subagents-note").textContent).toContain("journal");
  });

  it("with none, says so in the session's words, with no bar of dead controls", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed([]));
    render(<SubagentsTab model={model} title="Fix the cart totals" visible now={NOW} />);
    expect((await screen.findByTestId("subagents-empty")).textContent).toBe("This session hasn't started any subagents.");
    expect(screen.queryByTestId("subagents-toolbar")).toBeNull();
  });
});

describe("AJ-E1520 the session's line to its Subagents tab", () => {
  const Page = ({ conversation }: { conversation: string }) => {
    const session = { ...MOCK_SESSIONS[0]!, conversation_id: conversation };
    useHasSubagents(session.account_id, conversation, false);
    return <SessionNotes session={session} live now={NOW} />;
  };

  it("hears the tab's one read: how many, how many run, and opens the tab", async () => {
    const list = vi.spyOn(ipc, "subagentsList").mockResolvedValue(listed(TREE));
    const shown: string[] = [];
    const hear = (event: Event) => shown.push(String((event as CustomEvent<string>).detail));
    window.addEventListener("aij:page-tab", hear);
    render(<Page conversation="c-line-1" />);
    const line = await screen.findByTestId("session-subagents-line");
    expect(line.textContent).toBe("3 subagents · 1 running› Subagents");
    expect(list).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Open its subagents: 3 subagents · 1 running" }));
    window.removeEventListener("aij:page-tab", hear);
    expect(shown).toEqual(["subagents"]);
  });

  it("is quiet when there are none, or before they are read", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue({ ...listed([]), summary: { ...listed([]).summary, share_pct: null } });
    const Counts = () => <span data-testid="heard">{JSON.stringify(useHeardSubagentCounts(MOCK_SESSIONS[0]!.account_id, "c-line-2"))}</span>;
    render(
      <>
        <Counts />
        <Page conversation="c-line-2" />
      </>,
    );
    expect(screen.getByTestId("heard").textContent).toBe("null");
    await waitFor(() => expect(screen.getByTestId("heard").textContent).toBe('{"total":0,"running":0,"share":null}'));
    expect(screen.queryByTestId("session-subagents-line")).toBeNull();
    expect(subagentsLineWords({ total: 1, running: 0 })).toBe("1 subagent");
    expect(subagentsLineWords({ total: 2, running: 2 })).toBe("2 subagents · 2 running");
  });
});

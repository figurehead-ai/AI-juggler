/**
 * The Subagents tab's test fixtures, shared by its test files.
 */
import type { SubagentDetailInfo, SubagentInfo, SubagentRowInfo, SubagentsInfo } from "../../lib/ipc/subagents";
import type { SessionModel } from "./model";
import { blankDetail } from "../../preview/subagents";

export const NOW = 1_792_600_000_000;

export const model: SessionModel = {
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

export const agent = (id: string, extra: Partial<SubagentInfo> = {}): SubagentInfo => ({
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

export const group = (key: string, asked: string | null, count: number, open = true): SubagentRowInfo => ({
  key: `g:${key}`,
  row: "group",
  agent: null,
  guides: [],
  last: false,
  context: false,
  group: { key, asked, at_ms: NOW - 900_000, count, running: 1, failed: 0, tokens: 1_200_000, open, session: null },
});

export const listed = (items: SubagentRowInfo[], extra: Partial<SubagentsInfo> = {}): SubagentsInfo => {
  const agents = items.filter((r) => r.agent).map((r) => r.agent!);
  const running = agents.filter((a) => a.status === "running").length;
  return {
    state: "listed",
    reason: null,
    page: { items, next_cursor: null, total: items.length },
    summary: { total: agents.length, running, failed: agents.filter((a) => a.status === "failed").length, stopped: 0, unknown: 0, attention: agents.filter((a) => a.status === "failed").length, tokens: 36_000, share_pct: 41, counted: "in_session" },
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

export const TREE = [
  group("u1", "fix the totals and add tests", 3),
  row(agent("a1", { status: "running", ended_ms: null, task: "Audit the cart store", children: 1, tokens: 412_000, own_tokens: 390_000 })),
  row(agent("n1", { parent: "a1", depth: 2, kind: "Explore", background: false, status: "failed" }), [true], true),
  row(agent("a2", { task: "Review the diff" }), [], true),
  group("u0", "plan the totals epic", 41, false),
];

/** A subagent's page as the backend answers it: nothing recorded beyond
 * where it stands unless `extra` says. */
export const detailOf = (a: SubagentInfo, extra: Partial<SubagentDetailInfo> = {}): SubagentDetailInfo => ({
  ...blankDetail("Kit Tool"),
  state: "shown",
  agent: a,
  counted: "in_session",
  ...extra,
});

/**
 * AJ-E1520 (0.3) — the preview's (and the gallery's) Subagents: what a
 * seeded fixture (`__AIJ_FIXTURE__.subagents`, by conversation) answers,
 * in the backend's shapes and order — groups by the message that
 * started them (the newest open), each a tree with its lines, filtered,
 * sorted and paged like `features/subagents/view.rs`. Preview only: the
 * app asks the backend.
 */
import type { PageRequest } from "../../types/generated/PageRequest";
import type { SubagentsInfo } from "../../types/generated/SubagentsInfo";
import type { SubagentInfo } from "../../types/generated/SubagentInfo";
import type { SubagentRowInfo } from "../../types/generated/SubagentRowInfo";
import type { SubagentsQuery } from "../../types/generated/SubagentsQuery";
import type { SubagentDetailInfo } from "../../types/generated/SubagentDetailInfo";
import type { LastMessagesInfo } from "../../types/generated/LastMessagesInfo";

/** A seeded subagent: the row's fields, and what its page shows. */
export type SeedAgent = Partial<SubagentInfo> & {
  id: string;
  /** The message its turn began with (groups by it). */
  asked?: string | null;
  asked_ms?: number | null;
  prompt?: string | null;
  result?: string | null;
  failure?: string | null;
  files?: string[];
  detail?: Partial<SubagentDetailInfo>;
};

/** What a preview (or a spec) seeds, by conversation id. */
export type Seeded = {
  state?: string;
  reason?: string | null;
  live?: boolean;
  reading?: boolean;
  conversations_note?: string | null;
  tool?: string;
  /** The session's own tokens (for the share). */
  session_tokens?: number;
  items: SeedAgent[];
  /** Each subagent's turns, by its id. */
  turns?: Record<string, LastMessagesInfo["turns"]>;
};

type Fixture = { subagents?: Record<string, Seeded>; projectSubagents?: Record<string, string[]> };

function fixture(): Fixture {
  return (window as unknown as { __AIJ_FIXTURE__?: Fixture }).__AIJ_FIXTURE__ ?? {};
}

function info(seed: SeedAgent, live: boolean, all: SeedAgent[]): SubagentInfo {
  const children = all.filter((other) => other.parent === seed.id);
  const own = seed.own_tokens ?? seed.tokens ?? null;
  const below = (id: string): number => all.filter((o) => o.parent === id).reduce((sum, o) => sum + (o.own_tokens ?? o.tokens ?? 0) + below(o.id), 0);
  const status = seed.status ?? "done";
  return {
    kind: null,
    task: null,
    started_ms: null,
    ended_ms: null,
    duration_ms: null,
    last_activity_ms: null,
    parent: null,
    depth: 1,
    background: null,
    has_conversation: true,
    model: null,
    session: null,
    ...seed,
    status: status === "running" && !live ? "unknown" : status,
    own_tokens: own,
    tokens: own === null ? null : own + below(seed.id),
    children: children.length,
  };
}

function matches(seed: SubagentInfo & SeedAgent, query: SubagentsQuery): boolean {
  const view = query.view ?? "all";
  if (view === "running" && seed.status !== "running") return false;
  if (view === "attention" && !["failed", "stopped", "unknown"].includes(seed.status)) return false;
  if (query.kind && seed.kind !== query.kind) return false;
  const hay = [seed.task, seed.kind, seed.prompt, seed.result, seed.failure, ...(seed.files ?? [])].filter(Boolean).join("\n").toLowerCase();
  for (const term of (query.search ?? "").toLowerCase().split(/\s+/).filter(Boolean)) {
    if (term.startsWith("type:")) {
      if (!(seed.kind ?? "").toLowerCase().includes(term.slice(5))) return false;
    } else if (term.startsWith("file:")) {
      if (!(seed.files ?? []).join("\n").toLowerCase().includes(term.slice(5))) return false;
    } else if (!hay.includes(term)) return false;
  }
  return true;
}

function rank(sort: string | null, agent: SubagentInfo): number {
  if (sort === "tokens") return agent.tokens ?? -1;
  if (sort === "longest") return agent.duration_ms ?? -1;
  return agent.started_ms ?? -Infinity;
}

function build(seeded: Seeded, query: SubagentsQuery): { rows: SubagentRowInfo[]; matched: number; agents: SubagentInfo[] } {
  const live = seeded.live ?? false;
  const agents = seeded.items.map((seed) => ({ ...seed, ...info(seed, live, seeded.items) }));
  const byId = new Map(agents.map((a) => [a.id, a]));
  const active = (query.view ?? "all") !== "all" || !!query.kind || !!query.search?.trim();
  const matched = new Set(agents.filter((a) => matches(a, query)).map((a) => a.id));
  const shown = new Set(matched);
  if (active) for (const id of matched) for (let up = byId.get(id)?.parent; up; up = byId.get(up)?.parent ?? null) shown.add(up);
  const rootOf = (a: SubagentInfo): SubagentInfo => (a.parent && byId.get(a.parent) ? rootOf(byId.get(a.parent)!) : a);
  const groups = new Map<string, (typeof agents)[number][]>();
  for (const agent of agents) {
    const root = byId.get(rootOf(agent).id)!;
    const key = root.asked ? `p:${root.asked}` : "none";
    groups.set(key, [...(groups.get(key) ?? []), agent]);
  }
  const sort = query.sort ?? null;
  const order = [...groups.entries()]
    .filter(([, members]) => members.some((m) => shown.has(m.id)))
    .map(([key, members]) => {
      const tokens = members.reduce((sum, m) => sum + (m.own_tokens ?? 0), 0);
      const at = members.find((m) => !m.parent)?.asked_ms ?? Math.min(...members.map((m) => m.started_ms ?? Infinity));
      const groupRank = sort === "tokens" ? tokens : sort === "longest" ? Math.max(...members.map((m) => m.duration_ms ?? -1)) : at;
      return { key, members, tokens, at, groupRank };
    })
    .sort((a, b) => b.groupRank - a.groupRank);
  const rows: SubagentRowInfo[] = [];
  order.forEach((group, place) => {
    const open = query.closed.includes(group.key) ? false : query.open.includes(group.key) || active || place === 0;
    const root = group.members.find((m) => !m.parent);
    rows.push({
      key: `g:${group.key}`,
      row: "group",
      agent: null,
      guides: [],
      last: false,
      context: false,
      group: {
        key: group.key,
        asked: root?.asked ?? null,
        at_ms: Number.isFinite(group.at) ? group.at : null,
        count: group.members.length,
        running: group.members.filter((m) => m.status === "running").length,
        failed: group.members.filter((m) => m.status === "failed").length,
        tokens: group.tokens || null,
        open,
        session: root?.session ?? null,
      },
    });
    if (!open) return;
    const walk = (list: SubagentInfo[], guides: boolean[]) => {
      const sorted = list.filter((a) => shown.has(a.id)).sort((a, b) => rank(sort, b) - rank(sort, a));
      sorted.forEach((agent, i) => {
        const last = i === sorted.length - 1;
        const { asked: _a, asked_ms: _b, prompt: _c, result: _d, failure: _e, files: _f, detail: _g, ...row } = agent as SubagentInfo & SeedAgent;
        rows.push({ key: `a:${agent.id}`, row: "agent", group: null, agent: row, guides, last, context: !matched.has(agent.id) });
        walk(
          agents.filter((a) => a.parent === agent.id),
          [...guides, !last],
        );
      });
    };
    walk(
      group.members.filter((m) => !m.parent || !byId.has(m.parent)),
      [],
    );
  });
  return { rows, matched: matched.size, agents };
}

function listed(seeded: Seeded | undefined, query: SubagentsQuery, page: PageRequest): SubagentsInfo {
  const summaryEmpty = { total: 0, running: 0, failed: 0, attention: 0, tokens: null, share_pct: null, counted: "none" };
  const base = { kinds: [], matched: 0, running: 0, reading: false, live: false, conversations_note: null, tool: "Claude Code" };
  if (!seeded) return { state: "listed", reason: null, page: { items: [], next_cursor: null, total: 0 }, summary: summaryEmpty, ...base };
  const { rows, matched, agents } = build(seeded, query);
  const start = page.cursor ? Number(page.cursor.replace("preview:", "")) : 0;
  const limit = page.limit ?? 50;
  const tokens = agents.reduce((sum, a) => sum + (a.own_tokens ?? 0), 0);
  const count = (words: string[]) => agents.filter((a) => words.includes(a.status)).length;
  const kinds = new Map<string, number>();
  for (const a of agents) if (a.kind) kinds.set(a.kind, (kinds.get(a.kind) ?? 0) + 1);
  return {
    ...base,
    state: seeded.state ?? "listed",
    reason: seeded.reason ?? null,
    page: { items: rows.slice(start, start + limit), next_cursor: start + limit < rows.length ? `preview:${start + limit}` : null, total: rows.length },
    summary: {
      total: agents.length,
      running: count(["running"]),
      failed: count(["failed"]),
      attention: count(["failed", "stopped", "unknown"]),
      tokens: tokens || null,
      share_pct: tokens && seeded.session_tokens ? (tokens / (tokens + seeded.session_tokens)) * 100 : null,
      counted: tokens ? "in_session" : "none",
    },
    kinds: [...kinds.entries()].map(([kind, n]) => ({ kind, count: n })).sort((a, b) => b.count - a.count),
    matched,
    running: count(["running"]),
    reading: seeded.reading ?? false,
    live: seeded.live ?? false,
    conversations_note: seeded.conversations_note ?? null,
    tool: seeded.tool ?? "Claude Code",
  };
}

export function previewList(conversationId: string, query: SubagentsQuery, page: PageRequest): SubagentsInfo {
  return listed(fixture().subagents?.[conversationId], query, page);
}

export function previewProject(projectId: string, query: SubagentsQuery, page: PageRequest): SubagentsInfo {
  const ids = fixture().projectSubagents?.[projectId] ?? [];
  const all = ids.map((id) => fixture().subagents?.[id]).filter((s): s is Seeded => !!s);
  if (all.length === 0) return listed(undefined, query, page);
  return listed({ ...all[0]!, live: true, items: all.flatMap((s) => s.items.map((item) => ({ ...item, status: item.status === "running" && !s.live ? "unknown" : item.status }))) }, query, page);
}

export function previewDetail(conversationId: string, subagentId: string): SubagentDetailInfo {
  const seeded = fixture().subagents?.[conversationId];
  const seed = seeded?.items.find((item) => item.id === subagentId);
  const empty: SubagentDetailInfo = {
    state: "none", reason: null, agent: null, chain: [], prompt: null, result: null, failure: null, messages_off: false, asked: null, tools: null, lines: null,
    files: [], files_total: 0, children: [], children_total: 0, tokens: null, counted: "none", cost_usd: null, share_pct: null, processes: [], process_session: null,
    commands: 0, stop_hint: null, worktree: null, branch: null, stopped_by_user: null, tool: seeded?.tool ?? "Claude Code",
  };
  if (!seeded || !seed) return empty;
  const live = seeded.live ?? false;
  const chain: SubagentDetailInfo["chain"] = [];
  for (let up = seed.parent; up; ) {
    const parent = seeded.items.find((item) => item.id === up);
    if (!parent) break;
    chain.unshift({ id: parent.id, title: parent.task ?? parent.kind ?? null });
    up = parent.parent ?? null;
  }
  let root: SeedAgent = seed;
  while (root.parent) root = seeded.items.find((item) => item.id === root.parent) ?? { ...root, parent: null };
  const kids = seeded.items.filter((item) => item.parent === seed.id).map((item) => info(item, live, seeded.items));
  return {
    ...empty,
    state: "shown",
    agent: info(seed, live, seeded.items),
    chain,
    prompt: seed.prompt ?? null,
    result: seed.result ?? null,
    failure: seed.failure ?? null,
    asked: root.asked ?? null,
    files: seed.files ?? [],
    files_total: seed.files?.length ?? 0,
    children: kids,
    children_total: kids.length,
    counted: seed.own_tokens ? "in_session" : "none",
    stop_hint: seeded.tool && seeded.tool !== "Claude Code" ? null : "In Claude Code: /tasks, select it, press x.",
    ...seed.detail,
  };
}

export function seededTurns(conversationId: string, subagentId: string): LastMessagesInfo["turns"] | undefined {
  return fixture().subagents?.[conversationId]?.turns?.[subagentId];
}

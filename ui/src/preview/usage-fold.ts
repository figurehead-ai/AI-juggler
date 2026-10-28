/**
 * The preview's usage history (no backend): the backend's folding
 * (`features::usage_history::fold`) over the preview's synthetic hourly
 * rows, so the screens draw the same shapes they get from the app.
 */
import type { UsageHistoryInfo } from "../types/generated/UsageHistoryInfo";
import type { UsageSessionInfo } from "../types/generated/UsageSessionInfo";
import type { UsageShareInfo } from "../types/generated/UsageShareInfo";
import type { UsageTotalsInfo } from "../types/generated/UsageTotalsInfo";

/** One profile's usage of one session with one model in one hour. */
export type Row = { profile: string; hour: number; session: string; model: string; input: number; output: number; cache: number; messages: number };
export type SessionFacts = { project: string | null; tool: string; title: string | null; juggler: string | null; started: number };
export type Names = { profiles: Map<string, { label: string; tool: string }>; projects: Map<string, string>; sessions: Map<string, SessionFacts> };

const RATES: [RegExp, number, number, number][] = [
  [/opus/, 5, 25, 0.5],
  [/sonnet/, 3, 15, 0.3],
  [/haiku/, 1, 5, 0.1],
  [/gpt|codex/, 1.25, 10, 0.125],
];

export function priceOf(row: Pick<Row, "model" | "input" | "output" | "cache">): number | null {
  const rate = RATES.find(([pattern]) => pattern.test(row.model));
  return rate ? (row.input * rate[1] + row.output * rate[2] + row.cache * rate[3]) / 1e6 : null;
}

export const sessionKey = (profile: string, session: string) => `${profile}\u0000${session}`;

function bucketOf(edges: number[], at: number): number {
  if (at < edges[0]! || at >= edges[edges.length - 1]!) return -1;
  let lo = 0;
  let hi = edges.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (edges[mid]! <= at) lo = mid;
    else hi = mid;
  }
  return lo;
}

type Share = { io: number; cache: number; usd: number; priced: boolean; sessions: Set<string>; parts: Map<string, number> };
const share = (): Share => ({ io: 0, cache: 0, usd: 0, priced: false, sessions: new Set(), parts: new Map() });

function label(groupBy: string, key: string, names: Names): string {
  if (key === "__other") return "Other";
  if (key === "") return { model: "Model not recorded", project: "No project", tool: "Unknown tool", profile: "A removed profile" }[groupBy] ?? key;
  if (groupBy === "profile") return names.profiles.get(key)?.label ?? "A removed profile";
  if (groupBy === "project") return names.projects.get(key) ?? "A removed project";
  return key;
}

export type Folded = Omit<UsageHistoryInfo, "edges_ms" | "group_by" | "previous" | "started_per_bucket" | "switches" | "unmeasured_sessions" | "truncated" | "top_sessions" | "session_count" | "reading"> & {
  sessions: UsageSessionInfo[];
};

export function fold(rows: Row[], edges: number[], groupBy: string, names: Names): Folded {
  const n = edges.length - 1;
  const series = new Map<string, { io: number[]; usd: number[] }>();
  const shares = [new Map<string, Share>(), new Map<string, Share>(), new Map<string, Share>(), new Map<string, Share>()];
  const sessions = new Map<string, { profile: string; id: string; io: number; cache: number; usd: number; priced: boolean; first: number; last: number; messages: number }>();
  const perBucket = Array.from({ length: n }, () => new Set<string>());
  const hours = new Set<number>();
  const totals: UsageTotalsInfo = { tokens: { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 }, usd: null, unpriced: false, sessions: 0, active_hours: 0, messages: 0 };
  let usd = 0;
  for (const row of rows) {
    const bucket = bucketOf(edges, row.hour);
    if (bucket < 0) continue;
    const key = sessionKey(row.profile, row.session);
    const facts = names.sessions.get(key);
    const cost = priceOf(row);
    const io = row.input + row.output;
    totals.tokens.input += row.input;
    totals.tokens.output += row.output;
    totals.tokens.cache_read += row.cache;
    totals.messages += row.messages;
    if (cost === null) totals.unpriced = true;
    else usd += cost;
    hours.add(row.hour);
    const tool = facts?.tool ?? names.profiles.get(row.profile)?.tool ?? "";
    const project = facts?.project ?? "";
    const group = { tool, model: row.model, project }[groupBy as "tool"] ?? row.profile;
    const entry = series.get(group) ?? { io: new Array<number>(n).fill(0), usd: new Array<number>(n).fill(0) };
    entry.io[bucket]! += io;
    entry.usd[bucket]! += cost ?? 0;
    series.set(group, entry);
    [row.profile, tool, row.model, project].forEach((value, index) => {
      const map = shares[index]!;
      const s = map.get(value) ?? share();
      s.io += io;
      s.cache += row.cache;
      if (cost !== null) {
        s.usd += cost;
        s.priced = true;
      }
      s.sessions.add(key);
      s.parts.set(row.profile, (s.parts.get(row.profile) ?? 0) + io);
      map.set(value, s);
    });
    perBucket[bucket]!.add(key);
    const sum = sessions.get(key) ?? { profile: row.profile, id: row.session, io: 0, cache: 0, usd: 0, priced: false, first: row.hour, last: row.hour, messages: 0 };
    sum.io += io;
    sum.cache += row.cache;
    if (cost !== null) {
      sum.usd += cost;
      sum.priced = true;
    }
    sum.first = Math.min(sum.first, row.hour);
    sum.last = Math.max(sum.last, row.hour);
    sum.messages += row.messages;
    sessions.set(key, sum);
  }
  totals.usd = usd > 0 || !totals.unpriced ? usd : null;
  totals.sessions = sessions.size;
  totals.active_hours = hours.size;
  const ranked = (map: Map<string, Share>, by: string): UsageShareInfo[] =>
    [...map]
      .map(([key, s]) => ({
        key,
        label: label(by, key, names),
        io: s.io,
        cache: s.cache,
        usd: s.priced ? s.usd : null,
        sessions: s.sessions.size,
        profiles: [...s.parts].sort((a, b) => b[1] - a[1]).slice(0, 6).map(([profile, io]) => ({ profile, io })),
      }))
      .sort((a, b) => b.io - a.io || a.key.localeCompare(b.key));
  let list = [...series].sort((a, b) => b[1].io.reduce((x, y) => x + y, 0) - a[1].io.reduce((x, y) => x + y, 0) || a[0].localeCompare(b[0]));
  if (list.length > 8) {
    const rest = list.slice(7);
    list = list.slice(0, 7);
    list.push(["__other", { io: edges.slice(1).map((_, i) => rest.reduce((s, r) => s + r[1].io[i]!, 0)), usd: edges.slice(1).map((_, i) => rest.reduce((s, r) => s + r[1].usd[i]!, 0)) }]);
  }
  return {
    series: list.map(([key, value]) => ({ key, label: label(groupBy, key, names), io: value.io, usd: value.usd })),
    totals,
    by_profile: ranked(shares[0]!, "profile"),
    by_tool: ranked(shares[1]!, "tool"),
    by_model: ranked(shares[2]!, "model"),
    by_project: ranked(shares[3]!, "project").slice(0, 200),
    sessions: [...sessions.values()]
      .map((sum) => {
        const facts = names.sessions.get(sessionKey(sum.profile, sum.id));
        return {
          profile: sum.profile,
          id: sum.id,
          session_id: facts?.juggler ?? null,
          tool: facts?.tool ?? names.profiles.get(sum.profile)?.tool ?? "",
          title: facts?.title ?? null,
          project_id: facts?.project ?? null,
          io: sum.io,
          cache: sum.cache,
          usd: sum.priced ? sum.usd : null,
          first_ms: sum.first,
          last_ms: sum.last,
          messages: sum.messages,
        };
      })
      .sort((a, b) => b.io - a.io || b.last_ms - a.last_ms),
    sessions_per_bucket: perBucket.map((set) => set.size),
    active_hours: [...hours].sort((a, b) => a - b),
  };
}

export function startedPerBucket(edges: number[], starts: number[]): number[] {
  const counts = new Array<number>(edges.length - 1).fill(0);
  for (const start of starts) {
    const bucket = bucketOf(edges, start);
    if (bucket >= 0) counts[bucket]! += 1;
  }
  return counts;
}

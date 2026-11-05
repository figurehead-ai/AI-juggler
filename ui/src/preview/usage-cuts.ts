/**
 * AJ-E2902: the preview's cost cuts, from its own usage history: each
 * session's ending picked by its id (most finish; a few crash or are
 * stopped while stuck; other apps' have none), each folder its own
 * Account. Loaded only without a backend.
 */
import type { UsageQueryInput } from "../types/generated/UsageQueryInput";
import type { UsageSessionInfo } from "../types/generated/UsageSessionInfo";
import type { UsageCostCutsInfo } from "../types/generated/UsageCostCutsInfo";
import type { UsageCutInfo } from "../types/generated/UsageCutInfo";
import type { UsageWastedInfo } from "../types/generated/UsageWastedInfo";
import { pageOf, type Page, type PageRequest } from "../lib/paged";
import { previewHistory, previewSessionsPage } from "./usage-history";
import { subscriptionsList } from "../lib/ipc/subscriptions";

/** Every session in range (the preview has a few hundred at most). */
async function sessionsOf(input: UsageQueryInput): Promise<UsageSessionInfo[]> {
  const all: UsageSessionInfo[] = [];
  let cursor: string | null = null;
  do {
    const page: Page<UsageSessionInfo> = await previewSessionsPage(input, { cursor, limit: 200 });
    all.push(...page.items);
    cursor = page.next_cursor;
  } while (cursor);
  return all;
}

/** The preview's sessions stand for AI Juggler's own (most end; a few
 * are another app's, with no ending). */
function endingOf(row: UsageSessionInfo): "finished" | "crashed" | "stuck" | null {
  const n = [...row.id].reduce((sum, char) => (sum * 31 + char.charCodeAt(0)) % 1000, 7);
  return n < 620 ? "finished" : n < 680 ? "crashed" : n < 710 ? "stuck" : null;
}

const empty = (): UsageCutInfo => ({ count: 0, io: 0, usd: null, unpriced: false });
function take(cut: UsageCutInfo, row: UsageSessionInfo): void {
  cut.count += 1;
  cut.io += row.io;
  if (row.usd != null) cut.usd = (cut.usd ?? 0) + row.usd;
  else if (row.io > 0) cut.unpriced = true;
}

export async function previewCuts(input: UsageQueryInput): Promise<UsageCostCutsInfo> {
  const [history, sessions, accounts] = await Promise.all([previewHistory({ ...input, group_by: "project" }), sessionsOf(input), subscriptionsList()]);
  // As the backend does: each folder's share under the Account it's
  // signed in to, named as every screen names it.
  const accountOf = new Map(accounts.flatMap((account) => account.profiles.map((profile) => [profile.id, account] as const)));
  const finished = empty();
  const wasted = empty();
  let crashed = 0;
  let stuck = 0;
  let unknown = 0;
  for (const row of sessions) {
    const ending = endingOf(row);
    if (ending === "finished") take(finished, row);
    else if (ending) {
      take(wasted, row);
      if (ending === "crashed") crashed += 1;
      else stuck += 1;
    } else unknown += 1;
  }
  const projects = new Map(history.by_project.map((share) => [share.key, share.label]));
  const rows = new Map<string, UsageCostCutsInfo["by_account"][number]>();
  const parts = new Map<string, Map<string, { io: number; usd: number | null }>>();
  for (const share of history.by_profile) {
    const account = accountOf.get(share.key);
    const key = account?.key ?? share.key;
    const row = rows.get(key) ?? {
      key,
      account_id: account?.account_id ?? null,
      label: account?.display_name ?? share.label,
      tool: account?.tool ?? sessions.find((r) => r.profile === share.key)?.tool ?? "claude-code",
      io: 0,
      cache: 0,
      usd: null,
      unpriced: false,
      sessions: 0,
      top_project: null,
    };
    row.io += share.io;
    row.cache += share.cache;
    if (share.usd != null) row.usd = (row.usd ?? 0) + share.usd;
    row.sessions += share.sessions;
    const mine = sessions.filter((r) => r.profile === share.key && r.project_id);
    if (mine.some((r) => r.usd == null && r.io > 0)) row.unpriced = true;
    const byProject = parts.get(key) ?? new Map<string, { io: number; usd: number | null }>();
    for (const r of mine) {
      const part = byProject.get(r.project_id!) ?? { io: 0, usd: null };
      part.io += r.io;
      if (r.usd != null) part.usd = (part.usd ?? 0) + r.usd;
      byProject.set(r.project_id!, part);
    }
    parts.set(key, byProject);
    rows.set(key, row);
  }
  const by_account = [...rows.values()].map((row) => {
    const top = [...(parts.get(row.key) ?? [])].sort((a, b) => (b[1].usd ?? 0) - (a[1].usd ?? 0) || b[1].io - a[1].io)[0];
    return { ...row, top_project: top ? { id: top[0], label: projects.get(top[0]) ?? top[0], usd: top[1].usd, io: top[1].io } : null };
  });
  by_account.sort((a, b) => (b.usd ?? 0) - (a.usd ?? 0) || b.io - a.io);
  return { finished, wasted, crashed, stuck, unknown_endings: unknown, by_account, prices: { version: "v4", captured: "2026-11-03", stale: false, overridden: false } };
}

export async function previewWasted(input: UsageQueryInput, page: PageRequest): Promise<Page<UsageWastedInfo>> {
  const rows = (await sessionsOf(input)).flatMap((session): UsageWastedInfo[] => {
    const ending = endingOf(session);
    return ending === "crashed" || ending === "stuck" ? [{ session, why: ending }] : [];
  });
  rows.sort((a, b) => (b.session.usd ?? 0) - (a.session.usd ?? 0) || b.session.last_ms - a.session.last_ms);
  return pageOf(rows, page);
}

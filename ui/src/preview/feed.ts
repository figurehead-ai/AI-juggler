/**
 * D-118 — the preview's stand-in for the feed's read model: its items
 * (seeded as `window.__AIJ_FEED__`, else made from the preview's sessions
 * and earlier notices), filtered and paged by the backend's rules
 * (`features/feed/page.rs`): newest first, a page at a time, the user's
 * own acts only when asked. `__AIJ_FEED_CALLS__` counts the pages asked
 * for (the paging test: never more than one page a request), and
 * `__AIJ_FEED_FAILS__` makes a read fail (the error state).
 */
import type { FeedFiltersInfo } from "../types/generated/FeedFiltersInfo";
import type { FeedItemInfo } from "../types/generated/FeedItemInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { NoticeInfo } from "../types/generated/NoticeInfo";
import type { Page } from "../types/generated/Page";
import type { PageRequest } from "../types/generated/PageRequest";
import { MOCK_SESSIONS, seeded } from "./fixtures";
import { folderName } from "../lib/session-titles";
import { inState, inTab } from "../lib/feed-chips";

/** A seeded list, as far as the stand-in reads one (the seam's own). */
function isItems(value: unknown): value is FeedItemInfo[] {
  return Array.isArray(value);
}

function seed(): FeedItemInfo[] | null {
  const value: unknown = Reflect.get(window, "__AIJ_FEED__");
  return isItems(value) ? value : null;
}

function earlierSeed(): NoticeInfo[] {
  const notices: unknown = Reflect.get(window, "__AIJ_NOTICES__");
  const kept: unknown = typeof notices === "object" && notices !== null ? Reflect.get(notices, "earlier") : null;
  return Array.isArray(kept) ? kept.filter((one): one is NoticeInfo => typeof one === "object" && one !== null) : [];
}

/** Records a page asked for (`__AIJ_FEED_CALLS__`: the paging test). */
function recordCall(call: { cursor: string | null; limit: number }): void {
  const calls: unknown = Reflect.get(window, "__AIJ_FEED_CALLS__");
  Reflect.set(window, "__AIJ_FEED_CALLS__", [...(Array.isArray(calls) ? calls : []), call]);
}

const EMPTY_TARGET = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null, thread_id: null };
const NO_FACTS: FeedItemInfo["facts"] = {
  project: null, project_id: null, cwd: null, duration_ms: null, detail: null, attempt: null, tries: null, window: null, resets_at_ms: null, meter: null,
  sessions: null, credits: null, version: null, from_label: null, from_provider: null, to_label: null, to_provider: null, mechanism: null, fidelity: null,
  pool: null, pool_id: null, thread: null, thread_count: null, quote: null, change: null, need: null, line: null, told: null,
};

/** An item with the defaults the backend fills. */
export function feedItem(over: Omit<Partial<FeedItemInfo>, "facts" | "target"> & Pick<FeedItemInfo, "id" | "at_ms" | "kind" | "category"> & { facts?: Partial<FeedItemInfo["facts"]>; target?: Partial<FeedItemInfo["target"]> }): FeedItemInfo {
  return {
    level: 0, mine: false, coalesce: null, rollup: null, rollup_window_ms: 0, scope: null, subject: "", digest: null,
    author: { kind: "juggler", name: null, provider: null, folder: null, folder_label: null, desktop: false },
    ...over,
    facts: { ...NO_FACTS, ...over.facts },
    target: { ...EMPTY_TARGET, ...over.target },
  };
}

/** AJ-E3713 R6a: what the preview's finished sessions' last turns
 * changed, as the backend reads it from their transcripts' edit and write
 * calls (one counted, one a notebook's: files only). */
const CHANGED: Record<string, FeedItemInfo["facts"]["changes"] | undefined> = {
  "sess-done-1": { turn: "sess-done-1:0", files: 6, names: ["webhooks/stripe.ts", "webhooks/router.ts", "webhooks/verify.ts", "billing/events.ts", "tests/stripe.test.ts", "tests/router.test.ts"], added: 184, removed: 52 },
  "sess-done-2": { turn: "sess-done-2:0", files: 2, names: ["sync/retry.ts", "notebooks/backoff.ipynb"], added: null, removed: null },
};

const ENDED: Record<string, [string, number] | undefined> = { finished: ["finished", 1], failed: ["failed", 3], stopped: ["stopped", 0] };

/** A preview session's ending as the backend's item for it (tests seed
 * the feed with these). A gallery's seeded session can come without its
 * folder (the captures' fixtures): the item then names it as the window
 * would, never failing the whole page. */
export function endingItem(session: InstanceInfo): FeedItemInfo | null {
  const ended = ENDED[session.status];
  if (!ended) return null;
  const [kind, level] = ended;
  const at = session.last_activity_ms ?? session.started_at_ms;
  return feedItem({
    id: `l:${session.id}`, at_ms: at, kind, category: kind === "failed" ? "needs" : "finished", level, mine: kind === "stopped",
    coalesce: `s:${session.id}`, rollup: kind === "failed" ? null : `v:${kind}:${session.account_id}:${session.project_id ?? "-"}:L${level}`, rollup_window_ms: 7_200_000,
    scope: `f:${session.account_id}`, subject: session.label ?? session.title ?? (folderName(session.cwd) || "A session"),
    author: { kind: "app", name: null, provider: session.provider, folder: session.account_id, folder_label: session.account_label, desktop: session.kind === "desktop" },
    facts: { ...NO_FACTS, project_id: session.project_id, cwd: session.cwd ?? null, duration_ms: at - session.started_at_ms, changes: CHANGED[session.id] ?? null },
    target: { ...EMPTY_TARGET, session_id: session.id, project_id: session.project_id },
  });
}

/** A seeded earlier notice as the backend's item for it (its own line). */
function ofNotice(notice: NoticeInfo): FeedItemInfo {
  return feedItem({ id: `n:${notice.id}`, at_ms: notice.created_ms, kind: notice.kind.replace(/-/g, "_"), category: "other", level: 1, coalesce: `n:${notice.id}`, facts: { ...NO_FACTS, line: notice.title, project: notice.project }, target: notice.target });
}

/** The preview's items, newest first. */
export function feedItems(): FeedItemInfo[] {
  const all = seed() ?? [
    ...(seeded.sessions ?? MOCK_SESSIONS).map(endingItem).filter((item): item is FeedItemInfo => item !== null),
    ...earlierSeed().map(ofNotice),
  ];
  return [...all].sort((a, b) => b.at_ms - a.at_ms || (a.id < b.id ? 1 : -1));
}

/** The sessions running now, as the backend's Running chip reads them. */
function running(): Set<string> {
  return new Set((seeded.sessions ?? MOCK_SESSIONS).filter((one) => one.status === "running" || one.status === "starting").map((one) => one.id));
}

function keeps(item: FeedItemInfo, filters: FeedFiltersInfo): boolean {
  if (item.mine && !filters.include_mine) return false;
  // AJ-E3713 R5: the tabs and state chips by the backend's rules.
  if (filters.category && filters.category !== "all" && !inTab(item, filters.category)) return false;
  if (filters.state === "running" && !(item.target.session_id && running().has(item.target.session_id))) return false;
  if (filters.state && filters.state !== "running" && !inState(item.kind, filters.state)) return false;
  if (filters.project && item.facts.project_id !== filters.project && item.target.project_id !== filters.project) return false;
  if (filters.pool && item.facts.pool_id !== filters.pool) return false;
  if (filters.folders && !filters.folders.includes(item.author.folder ?? "")) return false;
  return true;
}

/** One page, as `feed_page` answers it (an offset cursor stands in for its key). */
export async function feedPagePreview(request: PageRequest, filters: FeedFiltersInfo): Promise<Page<FeedItemInfo>> {
  const limit = Math.min(Math.max(request.limit ?? 30, 1), 200);
  recordCall({ cursor: request.cursor ?? null, limit });
  if (Reflect.get(window, "__AIJ_FEED_FAILS__") === true) throw new Error("The feed couldn't be read.");
  const rows = feedItems().filter((item) => keeps(item, filters));
  const start = request.cursor ? Number(request.cursor) : 0;
  const items = rows.slice(start, start + limit);
  const end = start + items.length;
  return { items, next_cursor: end < rows.length ? String(end) : null, total: null };
}

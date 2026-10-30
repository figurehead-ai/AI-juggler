/**
 * Pools (AJ-E1702): the words and lookups the rail's Pool lens, the pool page
 * and the pool sheet share, and the one copy of the pools they all read
 * ({@link usePools}). The user owns every pool; nothing here reaches a
 * session.
 *
 * Instant and paged (I6): the pools' answer holds the pools with their
 * counts and the working set's places that aren't a project's default; a
 * session past the working set is read alone ({@link usePoolPlace}), and
 * a pool's members come a page at a time ({@link usePoolMembers}).
 */
import { useEffect, useSyncExternalStore } from "react";

import { onSessionsUpdated, type InstanceInfo } from "./bridge";
import { NO_PROJECT_POOL_ID, defaultPoolIdOf, poolMembers, poolPlace, poolsList, type PoolInfo, type PoolMemberInfo, type PoolMemberRow, type PoolRole, type PoolsInfo } from "./ipc/pools";
import { refreshPaged, usePagedList, type PagedList } from "./paged";

export { NO_PROJECT_POOL_ID, defaultPoolIdOf };
export type { PoolRole };

export const ROLES: { id: PoolRole; label: string }[] = [
  { id: "lead", label: "Lead" },
  { id: "member", label: "Member" },
  { id: "observer", label: "Observer" },
];

/** D-28: what agents may do in a pool, set by you in the app only. */
export const LEVELS: { id: string; label: string; words: string }[] = [
  { id: "read", label: "Read", words: "See who's here, what each one is doing, and the tasks." },
  {
    id: "work_together",
    label: "Work together",
    words:
      "Read, and post messages, share documents, work on tasks and get a new session ready for you to start. A message for a session that's waiting goes in when you click.",
  },
  { id: "full", label: "Full", words: "Work together, and put messages straight into each other's sessions, up to the hop limit." },
  { id: "custom", label: "Custom", words: "You pick each action." },
];

/** Each agent action, in plain words (the catalog's order). */
export const ACTION_WORDS: Record<string, string> = {
  read_roster: "See who's in the pool",
  read_cards: "See what each one is working on",
  read_tasks: "See the tasks",
  read_messages: "Read their messages and the shared docs",
  set_status: "Say what it's doing",
  post_message: "Post messages",
  share_doc: "Share documents",
  create_task: "Create tasks",
  claim_task: "Take a task",
  update_task: "Update a task",
  assign_task: "Give a task to someone else",
  deliver_to_session: "Put a message straight into another session",
  prepare_launch: "Get a new session ready for you to start",
};

export function levelLabel(level: string): string {
  return LEVELS.find((entry) => entry.id === level)?.label ?? "Read";
}

export function roleLabel(role: string): string {
  return ROLES.find((entry) => entry.id === role)?.label ?? "Member";
}

/** A session's place when the pools name it (one set apart from its
 * project's default, or read alone). */
function placed(pools: PoolsInfo | null, sessionId: string): PoolMemberInfo | undefined {
  return pools?.placements.find((row) => row.session_id === sessionId);
}

/** The pool a session is in: the place the pools name, else its
 * project's default pool. */
export function poolIdOf(pools: PoolsInfo | null, session: InstanceInfo): string {
  return placed(pools, session.id)?.pool_id ?? defaultPoolIdOf(session.project_id);
}

export function roleOf(pools: PoolsInfo | null, sessionId: string): PoolRole {
  const role = placed(pools, sessionId)?.role;
  return role === "lead" || role === "observer" ? role : "member";
}

/** The user has made a pool (pools stay out of sight until then). */
export function hasUserPools(pools: PoolsInfo | null): boolean {
  return (pools?.pools ?? []).some((pool) => pool.kind === "user");
}

export function poolById(pools: PoolsInfo | null, id: string): PoolInfo | undefined {
  return pools?.pools.find((pool) => pool.id === id);
}

/**
 * The rail's Pool lens (AJ-E1702, AJ-E2104): each item's pool, once you've
 * made one (`undefined` before: the lens waits and the rail shows places).
 * Only AI Juggler's own sessions are in pools; a session another app runs
 * is in none.
 */
export function railPoolOf(pools: PoolsInfo | null): ((item: { session: InstanceInfo | null }) => { id: string; label: string } | null) | undefined {
  if (!hasUserPools(pools)) return undefined;
  return (item) => {
    if (item.session === null) return null;
    const id = poolIdOf(pools, item.session);
    return { id, label: poolById(pools, id)?.name ?? "Its project's pool" };
  };
}

/** One line on what a pool of yours covers, when it mixes projects (D-33:
 * a project's default pool is that project alone), or null. */
export function scopeWords(pool: PoolInfo): string | null {
  if (pool.kind === "user" && pool.scopes > 1) return "This pool mixes projects: agents in it would see each other's work.";
  return null;
}

// --- The one copy of the pools ------------------------------------------------

/** The backend's last answer. */
let answered: PoolsInfo | null = null;
/** Places read alone, by session (sessions past the working set), the
 * newest last; a few at most. */
const alone = new Map<string, PoolMemberInfo>();
const ALONE_KEPT = 16;
/** The answer with the places read alone: what every screen reads. */
let current: PoolsInfo | null = null;
const listeners = new Set<() => void>();
let watching: (() => void) | null = null;

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function publish(): void {
  if (answered === null) current = null;
  else {
    const named = new Set(answered.placements.map((row) => row.session_id));
    const extra = [...alone.values()].filter((row) => !named.has(row.session_id));
    current = extra.length === 0 ? answered : { ...answered, placements: [...answered.placements, ...extra] };
  }
  for (const listener of listeners) listener();
}

/** Reads one session's place alone and keeps it (a failed read keeps
 * what was there). */
async function readAlone(sessionId: string): Promise<void> {
  try {
    const place = await poolPlace(sessionId);
    alone.delete(sessionId);
    alone.set(sessionId, place);
    while (alone.size > ALONE_KEPT) alone.delete(alone.keys().next().value as string);
    publish();
  } catch {
    // Its project's default stands in until the next read.
  }
}

/** A new answer: everyone reading the pools sees it, and the places read
 * alone are read again (a move may have changed them). */
function answer(info: PoolsInfo): void {
  answered = info;
  publish();
  for (const sessionId of [...alone.keys()]) void readAlone(sessionId);
}

/** A change went through: everyone reading the pools sees it, and every
 * pool's member list shown reads again. */
export function setPools(info: PoolsInfo): void {
  answer(info);
  refreshPaged(POOL_MEMBERS);
}

/** Reads the pools again (a failed read keeps the last answer). */
export async function refreshPools(): Promise<void> {
  try {
    answer(await poolsList());
  } catch {
    // Keep what was there: a pool list a moment old is still true enough.
  }
}

/**
 * The pools and the places of the sessions the app holds, read once for
 * every screen that shows them, and again when the sessions change (a new
 * session has a place; a switched one keeps its own). `null` until the
 * first answer.
 */
export function usePools(): PoolsInfo | null {
  const info = useSyncExternalStore(subscribe, () => current);
  useEffect(() => {
    if (watching) return;
    watching = onSessionsUpdated(() => void refreshPools());
    // Not part of the first window: read once it has painted.
    const idle = (window as { requestIdleCallback?: (fn: () => void) => void }).requestIdleCallback;
    if (idle) idle(() => void refreshPools());
    else setTimeout(() => void refreshPools(), 0);
  }, []);
  return info;
}

/**
 * One session's pool and role, any session: read alone when it's shown
 * (one opened from History is past the working set the pools' answer
 * covers), and kept with the pools, so the rail's Pool lens has it too.
 * `null` until known.
 */
export function usePoolPlace(sessionId: string | null): PoolMemberInfo | null {
  const pools = usePools();
  useEffect(() => {
    if (sessionId) void readAlone(sessionId);
  }, [sessionId]);
  return sessionId ? (placed(pools, sessionId) ?? null) : null;
}

/** The key prefix of every pool's member list (`refreshPaged`). */
export const POOL_MEMBERS = "pool-members:";

/** A pool's members, a page at a time: its lead first, then the most
 * recently active. The pool page and the pool sheet share it. */
export function usePoolMembers(poolId: string | null): PagedList<PoolMemberRow> {
  return usePagedList(poolId === null ? null : `${POOL_MEMBERS}${poolId}`, (page) => poolMembers(poolId!, page), { keyOf: (row) => row.session.id });
}

/** Tests: forget the pools read so far. */
export function resetPools(): void {
  watching?.();
  watching = null;
  answered = null;
  current = null;
  alone.clear();
  sheet = null;
}

// --- The pool sheet ----------------------------------------------------------

/** What the pool sheet is open on: making a pool (with a session to put
 * in it), or a session to move. A pool itself is changed on its canvas. */
export type PoolSheetTarget = { mode: "create"; sessionId?: string; projectId?: string | null } | { mode: "move"; sessionId: string };

let sheet: PoolSheetTarget | null = null;
const sheetListeners = new Set<() => void>();

/** Opens the pool sheet (`null` closes it), from anywhere. */
export function openPoolSheet(target: PoolSheetTarget | null): void {
  sheet = target;
  for (const listener of sheetListeners) listener();
}

export function usePoolSheet(): PoolSheetTarget | null {
  return useSyncExternalStore(
    (listener) => {
      sheetListeners.add(listener);
      return () => {
        sheetListeners.delete(listener);
      };
    },
    () => sheet,
  );
}

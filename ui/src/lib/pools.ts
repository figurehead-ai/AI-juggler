/**
 * Pools (AJ-E1702): the words and lookups the rail, Home, the pool page
 * and the pool sheet share, and the one copy of the pools they all read
 * ({@link usePools}). The user owns every pool; nothing here reaches a
 * session.
 */
import { useEffect, useSyncExternalStore } from "react";

import { onSessionsUpdated, type InstanceInfo } from "./bridge";
import { DEFAULT_POOL_ID, poolsList, type PoolInfo, type PoolRole, type PoolsInfo } from "./ipc/pools";

export { DEFAULT_POOL_ID };
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
    words: "Read, and post messages, share documents and work on tasks. A message for a session that's waiting goes in when you click.",
  },
  { id: "full", label: "Full", words: "Work together, and put messages straight into each other's sessions, up to the hop limit." },
  { id: "custom", label: "Custom", words: "You pick each action." },
];

/** Each agent action, in plain words (the catalog's order). */
export const ACTION_WORDS: Record<string, string> = {
  read_roster: "See who's in the pool",
  read_cards: "See what each one is working on",
  read_tasks: "See the tasks",
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

/** The pool a session is in (the default pool when the list is behind). */
export function poolIdOf(pools: PoolsInfo | null, sessionId: string): string {
  return pools?.members.find((row) => row.session_id === sessionId)?.pool_id ?? DEFAULT_POOL_ID;
}

export function roleOf(pools: PoolsInfo | null, sessionId: string): PoolRole {
  const role = pools?.members.find((row) => row.session_id === sessionId)?.role;
  return role === "lead" || role === "observer" ? role : "member";
}

/** The user has made a pool (pools stay out of sight until then). */
export function hasUserPools(pools: PoolsInfo | null): boolean {
  return (pools?.pools ?? []).some((pool) => pool.kind === "user");
}

export function poolById(pools: PoolsInfo | null, id: string): PoolInfo | undefined {
  return pools?.pools.find((pool) => pool.id === id);
}

/** The sessions in a pool, in the order given. */
export function sessionsIn(pools: PoolsInfo | null, poolId: string, sessions: InstanceInfo[]): InstanceInfo[] {
  return sessions.filter((session) => poolIdOf(pools, session.id) === poolId);
}

/** Only the sessions in one pool (`null`: every pool). */
export function filterByPool(rows: InstanceInfo[], pool: string | null, pools: PoolsInfo | null): InstanceInfo[] {
  return pool === null ? rows : rows.filter((row) => poolIdOf(pools, row.id) === pool);
}

/** One line on what the pool covers (D-33), or null when there's nothing to say. */
export function scopeWords(pool: PoolInfo): string | null {
  if (pool.kind === "default") return "Agents here will see only the sessions of their own project.";
  if (pool.scopes > 1) return "This pool mixes projects: agents in it would see each other's work.";
  return null;
}

// --- The one copy of the pools ------------------------------------------------

let current: PoolsInfo | null = null;
const listeners = new Set<() => void>();
let watching: (() => void) | null = null;

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** A change went through: everyone reading the pools sees it. */
export function setPools(info: PoolsInfo): void {
  current = info;
  for (const listener of listeners) listener();
}

/** Reads the pools again (a failed read keeps the last answer). */
export async function refreshPools(): Promise<void> {
  try {
    setPools(await poolsList());
  } catch {
    // Keep what was there: a pool list a moment old is still true enough.
  }
}

/**
 * The pools and every session's place, read once for every screen that
 * shows them, and again when the sessions change (a new session has a
 * place; a switched one keeps its own). `null` until the first answer.
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

/** Tests: forget the pools read so far. */
export function resetPools(): void {
  watching?.();
  watching = null;
  current = null;
  sheet = null;
}

// --- The pool sheet ----------------------------------------------------------

/** What the pool sheet is open on: making a pool (with a session to put
 * in it), one pool, or a session to move. */
export type PoolSheetTarget =
  | { mode: "create"; sessionId?: string; projectId?: string | null }
  | { mode: "pool"; poolId: string }
  | { mode: "move"; sessionId: string };

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

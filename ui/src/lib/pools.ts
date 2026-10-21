/**
 * Pools (AJ-E1702): the words and lookups the rail, Home and the pool
 * sheet share. The user owns every pool; nothing here reaches a session.
 */

import { DEFAULT_POOL_ID, type InstanceInfo, type PoolInfo, type PoolsInfo } from "./bridge";

export type PoolRole = "lead" | "member" | "observer";

export const ROLES: { id: PoolRole; label: string }[] = [
  { id: "lead", label: "Lead" },
  { id: "member", label: "Member" },
  { id: "observer", label: "Observer" },
];

/** What each capability is, in plain words (the grants table, §2.3). */
export const CAPABILITY_WORDS: Record<string, string> = {
  roster: "See who's here and what they're doing",
  own_inbox: "Read their own messages",
  message: "Message one member",
  broadcast: "Message the whole pool",
  share_doc: "Share a document",
  create_task: "Create tasks",
  create_plan: "Create stories and epics",
  assign: "Give work to someone else",
  claim: "Take an unassigned task",
  update_own: "Update their own task",
  close_others: "Close someone else's task",
};

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

/** One line on what the pool covers (D-33), or null when there's nothing to say. */
export function scopeWords(pool: PoolInfo): string | null {
  if (pool.kind === "default") {
    return "Agents here will see only sessions on their own project and account.";
  }
  if (pool.scopes > 1) {
    return "This pool mixes projects or accounts: agents in it would see each other's work.";
  }
  return null;
}

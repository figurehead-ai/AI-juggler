/**
 * AJ-E1702 — pools; AJ-E1703 — their context cards (ENGINEERING-GUIDE U3:
 * one IPC module per feature). The user owns every pool: these calls
 * only change pools, members, roles, access levels and rules, never a
 * session.
 *
 * The pools' answer never grows with the history (I6): the pools with
 * their counts, and the working set's places that aren't a project's
 * default. A pool's members come a page at a time (`poolMembers`); a
 * session past the working set is read alone (`poolPlace`).
 *
 * The preview keeps its own pools, seeded by `__AIJ_POOLS__` (pools and
 * members, levels and rules) and `__AIJ_POOL_ACTIVITY__` (what each session's transcript
 * said, by session id), and records the calls that change them in
 * `__AIJ_POOL_CALLS__`.
 */
import { invoke } from "./transport";
import { hasLiveBackend, MOCK_PROJECTS, MOCK_SESSIONS, refused, type InstanceInfo } from "../bridge";
import { pageOf, type Page, type PageRequest } from "../paged";
import { providerDisplayName } from "../providers";
import { previewHeldBy } from "./tasks";
import type { PoolsInfo } from "../../types/generated/PoolsInfo";
import type { PoolInfo } from "../../types/generated/PoolInfo";
import type { PoolMemberInfo } from "../../types/generated/PoolMemberInfo";
import type { PoolMemberRow } from "../../types/generated/PoolMemberRow";
import type { AccessInfo } from "../../types/generated/AccessInfo";
import type { AgentRuleInfo } from "../../types/generated/AgentRuleInfo";
import type { ContextCard } from "../../types/generated/ContextCard";
import type { CardField } from "../../types/generated/CardField";

export type { PoolsInfo, PoolInfo, PoolMemberInfo, PoolMemberRow, AccessInfo, AgentRuleInfo, ContextCard, CardField };
export type PoolRole = "lead" | "member" | "observer";

/** D-33: the default pool of sessions with no project. */
export const NO_PROJECT_POOL_ID = "pool-default:none";

/** D-33: a project's own default pool (the backend's `default_pool_id`). */
export function defaultPoolIdOf(projectId: string | null | undefined): string {
  return projectId ? `pool-default:${projectId}` : NO_PROJECT_POOL_ID;
}

/** A default pool's name: its project's (as it is now), and what it is. */
const defaultPoolName = (projectName: string | null) => `${projectName ?? "No project"} · all sessions`;

/** D-28: the agent actions, in the catalog's order, and what each
 * named level allows (the backend's `AccessLevel::actions`). */
export const AGENT_ACTIONS = [
  "read_roster",
  "read_cards",
  "read_tasks",
  "read_messages",
  "set_status",
  "post_message",
  "share_doc",
  "create_task",
  "claim_task",
  "update_task",
  "assign_task",
  "deliver_to_session",
  "prepare_launch",
] as const;
const READ = AGENT_ACTIONS.slice(0, 4);
// Staging a launch is Work together's: the Needs you card's click is the gate.
const WORK = [...AGENT_ACTIONS.slice(0, 10), "prepare_launch"];
const LEVEL_ACTIONS: Record<string, readonly string[]> = { read: READ, work_together: WORK, full: [...WORK, "deliver_to_session"] };

// D-29: delivering into a session is Full's or a rule's, never Custom's.
const accessAt = (level: string, custom: string[] = [], hopLimit = 5, stopped = false): AccessInfo => ({
  level,
  actions: AGENT_ACTIONS.filter(
    (action) => (LEVEL_ACTIONS[level] ?? custom.filter((word) => word !== "deliver_to_session")).includes(action),
  ),
  hop_limit: hopLimit,
  delivery_stopped: stopped,
});

type Activity = { files?: string[]; said?: string | null };
interface Seams {
  __AIJ_POOLS__?: { pools: Partial<PoolInfo>[]; members?: PoolMemberInfo[] };
  __AIJ_POOL_ACTIVITY__?: Record<string, Activity>;
  __AIJ_POOL_CALLS__?: string[];
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_POOL_CALLS__ ??= []).push(call);

/** The preview's own pools and every preview session's place. */
type Store = { pools: PoolInfo[]; places: PoolMemberInfo[] };
let store: Store | null = null;
let counter = 0;

const defaultPool = (projectId: string | null, name: string | null): PoolInfo => ({
  id: defaultPoolIdOf(projectId),
  name: defaultPoolName(name),
  kind: "default",
  project_id: projectId,
  access: accessAt("work_together"),
  rules: [],
  spans_macs: false,
  scopes: 1,
  members: 0,
});

/** The preview's store: every preview session has a place (its project's
 * default pool unless seeded otherwise). */
function previewStore(): Store {
  if (store === null) {
    const seeded = seams().__AIJ_POOLS__;
    const pools: PoolInfo[] = [
      ...(seeded?.pools ?? [])
        .filter((pool) => !pool.id?.startsWith("pool-default:"))
        .map((pool, index) => ({
          id: pool.id ?? `pool-seeded-${index}`,
          name: pool.name ?? `Pool ${index + 1}`,
          kind: "user",
          project_id: pool.project_id ?? null,
          access: pool.access ?? accessAt("work_together"),
          rules: pool.rules ?? [],
          spans_macs: pool.spans_macs ?? false,
          scopes: 1,
          members: 0,
        })),
      ...[...MOCK_PROJECTS].sort((a, b) => a.name.localeCompare(b.name)).map((project) => defaultPool(project.id, project.name)),
      defaultPool(null, null),
    ];
    store = { pools, places: [...(seeded?.members ?? [])] };
  }
  for (const session of MOCK_SESSIONS) {
    if (!store.places.some((row) => row.session_id === session.id)) {
      store.places.push({ session_id: session.id, pool_id: defaultPoolIdOf(session.project_id), role: "member" });
    }
  }
  return store;
}

const sessionOf = (id: string) => MOCK_SESSIONS.find((session) => session.id === id);

/** The preview's pools, as the backend answers: yours first, then each
 * project's default pool by name, "No project" last, each with how many
 * sessions it holds (archived ones aside); the places that aren't a
 * project's default, as a member. */
function preview(): PoolsInfo {
  const state = previewStore();
  const placed = state.places.filter((row) => sessionOf(row.session_id) !== undefined);
  // D-33: a pool's scopes are its projects, whatever the account.
  const inPool = (poolId: string) =>
    placed
      .filter((row) => row.pool_id === poolId)
      .map((row) => sessionOf(row.session_id))
      .filter((session): session is InstanceInfo => session !== undefined);
  return {
    pools: state.pools.map((pool) => ({
      ...pool,
      scopes: new Set(inPool(pool.id).map((session) => session.project_id ?? "")).size,
      members: inPool(pool.id).filter((session) => session.lifecycle !== "archived").length,
      access: { ...pool.access },
      rules: pool.rules.map((rule) => ({ ...rule })),
    })),
    placements: placed
      .filter((row) => row.role !== "member" || row.pool_id !== defaultPoolIdOf(sessionOf(row.session_id)?.project_id))
      .map((row) => ({ ...row }))
      .sort((a, b) => a.session_id.localeCompare(b.session_id)),
  };
}

function checkedName(name: string, except: string | null): string {
  const trimmed = name.trim();
  if (trimmed === "") throw refused("Give the pool a name");
  if ([...trimmed].length > 48) throw refused("A pool's name is at most 48 characters");
  const other = preview().pools.find((pool) => pool.id !== except && pool.name.toLowerCase() === trimmed.toLowerCase());
  if (other) throw refused(`There's already a pool called «${other.name}» — pick another name`);
  return trimmed;
}

function livePool(poolId: string): PoolInfo {
  const pool = previewStore().pools.find((row) => row.id === poolId);
  if (!pool) throw refused("That pool isn't there any more");
  return pool;
}

/** Every pool, and the working set's places. */
export async function poolsList(): Promise<PoolsInfo> {
  if (!hasLiveBackend()) return preview();
  return invoke<PoolsInfo>("pools_list");
}

const rank = (role: string) => (role === "lead" ? 0 : role === "member" ? 1 : 2);

/** One page of a pool's members (archived sessions aside): its lead
 * first, then the most recently active. */
export async function poolMembers(poolId: string, page: PageRequest): Promise<Page<PoolMemberRow>> {
  if (!hasLiveBackend()) {
    livePool(poolId);
    const rows = previewStore()
      .places.filter((row) => row.pool_id === poolId)
      .map((row) => ({ session: sessionOf(row.session_id), role: row.role }))
      .filter((row): row is { session: InstanceInfo; role: string } => row.session !== undefined && row.session.lifecycle !== "archived")
      .sort((a, b) => rank(a.role) - rank(b.role) || (b.session.last_activity_ms ?? 0) - (a.session.last_activity_ms ?? 0) || a.session.id.localeCompare(b.session.id))
      .map(({ session, role }) => ({ session, card: previewCard(session, role) }));
    return pageOf(rows, page);
  }
  return invoke<Page<PoolMemberRow>>("pools_members", { poolId, page });
}

/** One session's pool and role: any session, one past the working set
 * too (opened from History). */
export async function poolPlace(sessionId: string): Promise<PoolMemberInfo> {
  if (!hasLiveBackend()) {
    const place = previewStore().places.find((row) => row.session_id === sessionId);
    if (!place) throw refused("That session isn't there any more");
    return { ...place };
  }
  return invoke<PoolMemberInfo>("pools_place", { sessionId });
}

/** Makes a pool, optionally for one project. */
export async function poolCreate(name: string, projectId: string | null): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const clean = checkedName(name, null);
    record(`create:${clean}`);
    counter += 1;
    const mine = store!.pools.filter((pool) => pool.kind === "user").length;
    store!.pools.splice(mine, 0, { id: `pool-preview-${counter}`, name: clean, kind: "user", project_id: projectId, access: accessAt("work_together"), rules: [], spans_macs: false, scopes: 0, members: 0 });
    return preview();
  }
  return invoke<PoolsInfo>("pools_create", { name, projectId });
}

/** Renames a pool (a project's default pool keeps its project's name). */
export async function poolRename(poolId: string, name: string): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    if (pool.kind === "default") throw refused(`«${pool.name}» keeps its name: rename its project instead`);
    pool.name = checkedName(name, poolId);
    record(`rename:${poolId}:${pool.name}`);
    return preview();
  }
  return invoke<PoolsInfo>("pools_rename", { poolId, name });
}

/** Deletes a pool; its sessions go back to their projects' default pools. */
export async function poolDelete(poolId: string): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    if (pool.kind === "default") throw refused(`«${pool.name}» can't be deleted — it holds every session of its project that isn't in another pool`);
    record(`delete:${poolId}`);
    store!.pools = store!.pools.filter((row) => row.id !== poolId);
    for (const row of store!.places) {
      const session = MOCK_SESSIONS.find((entry) => entry.id === row.session_id);
      if (row.pool_id === poolId) Object.assign(row, { pool_id: defaultPoolIdOf(session?.project_id), role: "member" });
    }
    return preview();
  }
  return invoke<PoolsInfo>("pools_delete", { poolId });
}

/** Moves a session into a pool, as a member. */
export async function poolMove(sessionId: string, poolId: string): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    const session = MOCK_SESSIONS.find((row) => row.id === sessionId);
    if (!session) throw refused("That session isn't there any more");
    if (pool.project_id !== null && session.project_id !== pool.project_id) {
      const project = MOCK_PROJECTS.find((row) => row.id === pool.project_id);
      throw refused(`«${pool.name}» is for ${project?.name ?? "one project"} only — this session works somewhere else`);
    }
    if (pool.id === NO_PROJECT_POOL_ID && session.project_id) {
      throw refused("«No project · all sessions» is for sessions with no project — this one has one");
    }
    record(`move:${sessionId}:${poolId}`);
    Object.assign(store!.places.find((row) => row.session_id === sessionId)!, { pool_id: poolId, role: "member" });
    return preview();
  }
  return invoke<PoolsInfo>("pools_move", { sessionId, poolId });
}

/** Sets a session's role in its pool. */
export async function poolSetRole(sessionId: string, role: PoolRole): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    preview();
    const row = store!.places.find((entry) => entry.session_id === sessionId);
    if (!row) throw refused("That session isn't there any more");
    record(`role:${sessionId}:${role}`);
    row.role = role;
    return preview();
  }
  return invoke<PoolsInfo>("pools_set_role", { sessionId, role });
}

/** D-28: sets a pool's access level; `actions` is the Custom set. */
export async function poolSetAccess(poolId: string, level: string, actions: string[] = []): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    record(`access:${poolId}:${level}:${actions.join(",")}`);
    pool.access = accessAt(level, actions, pool.access.hop_limit, pool.access.delivery_stopped);
    return preview();
  }
  return invoke<PoolsInfo>("pools_set_access", { poolId, level, actions });
}

/** D-29: sets a pool's hop limit (1 to 20) and its delivery stop-all. */
export async function poolSetDelivery(poolId: string, hopLimit: number, stopped: boolean): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    if (!Number.isInteger(hopLimit) || hopLimit < 1 || hopLimit > 20) throw refused("A hop limit is 1 to 20");
    record(`delivery:${poolId}:${hopLimit}:${stopped}`);
    pool.access = { ...pool.access, hop_limit: hopLimit, delivery_stopped: stopped };
    return preview();
  }
  return invoke<PoolsInfo>("pools_set_delivery", { poolId, hopLimit, stopped });
}

/** D-32: lets a pool span your paired Macs, or not (off by default). */
export async function poolSetSpansMacs(poolId: string, spans: boolean): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = livePool(poolId);
    record(`spans:${poolId}:${spans}`);
    pool.spans_macs = spans;
    return preview();
  }
  return invoke<PoolsInfo>("pools_set_spans_macs", { poolId, spans });
}

/** D-28: takes back an "always allow" rule (it stops applying at once). */
export async function agentRuleRevoke(ruleId: number): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    preview();
    const pool = store!.pools.find((row) => row.rules.some((rule) => rule.id === ruleId));
    if (!pool) throw refused("That rule isn't there any more");
    record(`revoke:${ruleId}`);
    pool.rules = pool.rules.filter((rule) => rule.id !== ruleId);
    return preview();
  }
  return invoke<PoolsInfo>("pools_revoke_rule", { ruleId });
}

const unavailable = (why: string): CardField => ({ value: null, unavailable: `Unavailable — ${why}` });

/** A preview member's card, as the backend builds it. */
function previewCard(session: InstanceInfo, role: string): ContextCard {
  const folder = (session.cwd ?? "").replace(/\/+$/, "").split("/").pop() || "its folder";
  const project = MOCK_PROJECTS.find((row) => row.id === session.project_id)?.name ?? folder;
  const activity = seams().__AIJ_POOL_ACTIVITY__?.[session.id];
  // D-33, D-35: the profile's label, never an email.
  const profile = session.account_label && !session.account_label.includes("@") ? session.account_label : null;
  // A live session is read on the tick once its transcript is found; an
  // ended one was read only while it ran.
  const live = ["starting", "running", "waiting_user", "stuck"].includes(session.status);
  const why = live ? "its transcript hasn't been read yet" : "read only while it runs in AI Juggler";
  return {
    session_id: session.id,
    name: session.label ?? session.title ?? session.git_branch ?? folder,
    // D-36: the rows' place line, project · app · profile.
    place: [project, "AI Juggler", profile].filter(Boolean).join(" · "),
    tool: providerDisplayName(session.provider),
    account: profile,
    state: session.status,
    role,
    task: { value: previewHeldBy(session.id), unavailable: null }, // E19
    said: activity
      ? activity.said
        ? { value: activity.said, unavailable: null }
        : unavailable("it hasn't said")
      : unavailable(why),
    last_activity_ms: session.last_activity_ms,
    files: activity?.files ?? [],
    files_unavailable: activity ? null : `Unavailable — ${why}`,
  };
}

/** Tests: forget the preview's pools between cases. */
export function resetPreviewPools(): void {
  store = null;
  counter = 0;
}

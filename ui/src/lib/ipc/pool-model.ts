/**
 * D-62 (AJ-E3003–E3005) — the pools model as the user changes it: a
 * pool's scope, its member rules of every kind (a session, an App, an
 * Account, a Tool, a Mac, everyone in scope) with a role and their own
 * level, who may tell whom, sealing; and a session's own "which pools,
 * how, and why not" (its Pools tab). Only the app's window calls these;
 * nothing an agent runs reaches them (D-28).
 *
 * The preview keeps its rules beside `pools.ts`'s pools and records each
 * change in `__AIJ_POOL_CALLS__`.
 */
import { invoke } from "./transport";
import { hasLiveBackend, refused } from "../bridge";
import { previewPoolToChange, previewPoolsInfo, type PoolRole, type PoolsInfo } from "./pools";
import type { PoolRuleInfo } from "../../types/generated/PoolRuleInfo";
import type { PoolRulesInfo } from "../../types/generated/PoolRulesInfo";
import type { PoolTalkInfo } from "../../types/generated/PoolTalkInfo";
import type { SessionPoolsInfo } from "../../types/generated/SessionPoolsInfo";

export type { PoolRuleInfo, PoolRulesInfo, PoolTalkInfo, SessionPoolsInfo };

/** How a member matches sessions, most specific first (D-62 decision 11). */
export const MEMBER_KINDS = ["session", "app", "account", "tool", "mac", "everyone"] as const;
export type MemberKind = (typeof MEMBER_KINDS)[number];

/** Who may tell whom (D-62 decision 13). */
export type TalkRule = "allowed" | "ask" | "blocked";

/** A member rule's stored ref: `everyone` is `all`. */
export const memberRef = (kind: MemberKind, id = ""): string => (kind === "everyone" ? "all" : `${kind}:${id}`);

type Seams = { __AIJ_POOL_CALLS__?: string[] };
const record = (call: string) => (((globalThis as unknown as Seams).__AIJ_POOL_CALLS__ ??= []).push(call));

/** The preview's rules, by pool. */
const previewRules = new Map<string, PoolRulesInfo>();
const rulesOf = (poolId: string): PoolRulesInfo => {
  const known = previewRules.get(poolId);
  if (known) return known;
  const pool = previewPoolToChange(poolId);
  const made: PoolRulesInfo = { pool_id: poolId, members: pool.kind === "default" ? [{ member: "all", kind: "everyone", role: "member", level: null, joined_at: 0 }] : [], talk: [] };
  previewRules.set(poolId, made);
  return made;
};
const kindOf = (member: string): string => (member === "all" ? "everyone" : member.split(":")[0] === "work" ? "session" : member.split(":")[0]);

/** Adds a member rule to a pool, or changes its role and its own level. */
export async function poolJoin(poolId: string, member: string, role: PoolRole, level: string | null = null, actions: string[] = []): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const rules = rulesOf(poolId);
    record(`join:${poolId}:${member}:${role}:${level ?? ""}`);
    const row = rules.members.find((one) => one.member === member);
    if (row) Object.assign(row, { role, level });
    else rules.members.push({ member, kind: kindOf(member), role, level, joined_at: Date.now() });
    return previewPoolsInfo();
  }
  return invoke<PoolsInfo>("pools_join", { poolId, member, role, level, actions });
}

/** Ends a member rule in a pool; its other pools stay. */
export async function poolLeave(poolId: string, member: string): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const rules = rulesOf(poolId);
    if (previewPoolToChange(poolId).kind === "default" && member === "all") throw refused("A project's own pool always holds everyone in its project.");
    record(`leave:${poolId}:${member}`);
    rules.members = rules.members.filter((one) => one.member !== member);
    return previewPoolsInfo();
  }
  return invoke<PoolsInfo>("pools_leave", { poolId, member });
}

/** Sets a pool's scope: every project, or these (`null`: no project). */
export async function poolSetScope(poolId: string, every: boolean, projects: (string | null)[]): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = previewPoolToChange(poolId);
    if (pool.kind === "default") throw refused("A project's own pool keeps its project.");
    if (!every && projects.length === 0) throw refused("Pick at least one project, or every project.");
    record(`scope:${poolId}:${every ? "every" : projects.map((one) => one ?? "none").join(",")}`);
    Object.assign(pool, { scope_all: every, scope: every ? [] : [...projects], project_id: !every && projects.length === 1 ? projects[0] : null });
    return previewPoolsInfo();
  }
  return invoke<PoolsInfo>("pools_set_scope", { poolId, every, projects });
}

/** Seals a pool, or opens it. */
export async function poolSetSealed(poolId: string, sealed: boolean): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    const pool = previewPoolToChange(poolId);
    if (pool.kind === "default") throw refused("A project's own pool can't be sealed: make a pool for the work you want kept apart.");
    record(`sealed:${poolId}:${sealed}`);
    pool.sealed = sealed;
    return previewPoolsInfo();
  }
  return invoke<PoolsInfo>("pools_set_sealed", { poolId, sealed });
}

/** Whether a pool's broad members skip sessions another pool names. */
export async function poolSetSkipHeld(poolId: string, skip: boolean): Promise<PoolsInfo> {
  if (!hasLiveBackend()) {
    record(`skip_held:${poolId}:${skip}`);
    previewPoolToChange(poolId).skip_held = skip;
    return previewPoolsInfo();
  }
  return invoke<PoolsInfo>("pools_set_skip_held", { poolId, skip });
}

/** Who may tell whom: a role pair (`role:lead` → `role:member`) or one
 * connection; `null` clears it. */
export async function poolSetTalk(poolId: string, from: string, to: string, rule: TalkRule | null): Promise<PoolRulesInfo> {
  if (!hasLiveBackend()) {
    const rules = rulesOf(poolId);
    record(`talk:${poolId}:${from}:${to}:${rule ?? "clear"}`);
    rules.talk = rules.talk.filter((one) => one.from !== from || one.to !== to);
    if (rule) rules.talk.push({ from, to, rule });
    return { ...rules, members: [...rules.members], talk: [...rules.talk] };
  }
  return invoke<PoolRulesInfo>("pools_set_talk", { poolId, from, to, rule });
}

/** A pool's member rules and talk settings. */
export async function poolRules(poolId: string): Promise<PoolRulesInfo> {
  if (!hasLiveBackend()) {
    const rules = rulesOf(poolId);
    return { ...rules, members: [...rules.members], talk: [...rules.talk] };
  }
  return invoke<PoolRulesInfo>("pools_rules", { poolId });
}

/** A session's pools, how it got into each, and the ones that left it out. */
export async function poolsOfSession(sessionId: string): Promise<SessionPoolsInfo> {
  if (!hasLiveBackend()) {
    const info = previewPoolsInfo();
    const placed = info.placements.find((row) => row.session_id === sessionId);
    const pool = placed ? info.pools.find((one) => one.id === placed.pool_id) : undefined;
    return {
      session_id: sessionId,
      pools: pool && placed ? [{ pool_id: pool.id, pool_name: pool.name, role: placed.role, via: "session", member: `session:${sessionId}`, level: null, sealed: pool.sealed }] : [],
      skipped: [],
      account_unknown: false,
    };
  }
  return invoke<SessionPoolsInfo>("pools_of_session", { sessionId });
}

export function resetPreviewPoolModel(): void {
  previewRules.clear();
}

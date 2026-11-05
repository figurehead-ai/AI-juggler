/**
 * D-62 (AJ-E3007): the New pool wizard's draft, what it can add, and the
 * pool it makes.
 *
 * - Candidates come from what exists, by kind (sessions, Apps, Accounts,
 *   Tools, Macs, everyone), Open now first, then Recent, then All. One
 *   that can't join says why and the step that would let it in.
 * - Each member says what it can do in the pool: full (a session AI
 *   Juggler started with its agent tools), shell only (its tool has no
 *   per-launch tools yet), watched (another app runs it), or later (a
 *   broad member: its sessions join by themselves).
 * - Making the pool is one sequence of the user's own calls: create,
 *   scope, members, talk, seal. Nothing starts a session.
 */
import type { InstanceInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";
import { memberRef, poolJoin, poolSetScope, poolSetSealed, poolSetTalk, type MemberKind, type TalkRule } from "./ipc/pool-model";
import { poolCreate, poolSetAccess, poolSetSpansMacs, type PoolInfo, type PoolRole, type PoolsInfo } from "./ipc/pools";
import { pairKey, ruleOf, templateById, TALK_ROLES, type PairKey, type PoolTemplate } from "./pool-templates";
import { sessionState } from "./rail-items";
import type { AppModel } from "./apps";
import type { SubscriptionInfo } from "./subscriptions";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import { MACHINES_WORD } from "./machine-words";

/** What a member can do in the pool. */
export type Reach = "full" | "shell" | "watched" | "later";

export interface DraftMember {
  ref: string;
  kind: MemberKind;
  label: string;
  role: PoolRole;
  /** Its own level, `null`: the pool's. */
  level: string | null;
  reach: Reach;
}

export interface Draft {
  template: string;
  name: string;
  every: boolean;
  /** The projects it takes (`null`: sessions with no project). */
  projects: (string | null)[];
  members: DraftMember[];
  talk: Partial<Record<PairKey, TalkRule>>;
  level: "read" | "work_together" | "full";
  sealed: boolean;
  spansMacs: boolean;
}

/** A new draft from a template, in `project` when one is in view. */
export function draftOf(template: PoolTemplate, project: string | null): Draft {
  return {
    template: template.id,
    name: template.id === "blank" ? "" : template.name,
    every: template.scope === "every",
    projects: template.scope === "every" || project === null ? [] : [project],
    members: [],
    talk: { ...template.talk },
    level: template.level,
    sealed: template.sealed ?? false,
    spansMacs: template.spansMacs ?? false,
  };
}

/** The role a template gives the next member of `kind` added: its roles
 * go to the sessions that can act, in order; a Watched session (it can't
 * act until continued) joins as a Member. */
export function nextRole(draft: Draft, kind: MemberKind, reach: Reach = "full"): PoolRole {
  const template = templateById(draft.template);
  if (kind !== "session") return template.broadRole ?? "member";
  if (reach === "watched") return "member";
  const named = draft.members.filter((member) => member.kind === "session" && member.reach !== "watched").length;
  return template.roles[named] ?? "member";
}

export interface Candidate {
  ref: string;
  kind: MemberKind;
  label: string;
  /** Its project, state and where it runs. */
  detail: string;
  group: "open" | "recent" | "all";
  reach: Reach;
  /** Why it can't join now; `null` when it can. */
  blocked: string | null;
  /** The step that would let it in. */
  fix?: { label: string; action: "scope" | "continue" | "resume" | "open-pool"; arg: string };
  lastMs: number;
}

const DAY = 24 * 60 * 60 * 1000;

/** The scope holds a session of `project`. */
export function inScope(draft: Draft, project: string | null): boolean {
  return draft.every || draft.projects.includes(project);
}

/** Sessions: AI Juggler's own (full or shell only) and the ones other apps
 * run (watched), each with why it can't join when it can't. */
export function sessionCandidates(input: {
  draft: Draft;
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  pools: PoolsInfo | null;
  tools: ToolSpecInfo[];
  projectName: (id: string | null) => string;
  nameOf: (session: InstanceInfo) => string;
  now: number;
}): Candidate[] {
  const { draft, pools, tools, projectName, now } = input;
  const agentTools = new Set(tools.filter((tool) => tool.supports.agent_tools).map((tool) => tool.id));
  const sealedPool = (sessionId: string): PoolInfo | undefined => {
    const ids = new Set((pools?.placements ?? []).filter((row) => row.session_id === sessionId).map((row) => row.pool_id));
    return pools?.pools.find((pool) => pool.sealed && ids.has(pool.id));
  };
  const own: Candidate[] = input.sessions.map((session) => {
    const state = sessionState(session);
    const ended = state.state === "ended";
    const lastMs = session.last_activity_ms ?? session.started_at_ms ?? 0;
    const project = session.project_id ?? null;
    const sealed = sealedPool(session.id);
    const reach: Reach = agentTools.has(session.provider) ? "full" : "shell";
    const base = { ref: memberRef("session", session.id), kind: "session" as const, label: input.nameOf(session), reach, lastMs };
    const detail = `${projectName(project)} · ${state.word}`;
    const group = !ended ? "open" : now - lastMs < 7 * DAY ? "recent" : "all";
    if (!inScope(draft, project)) {
      return { ...base, detail, group, blocked: `Works in ${projectName(project)}, not in this pool's projects.`, fix: { label: `Add ${projectName(project)}`, action: "scope", arg: project ?? "" } };
    }
    if (sealed) return { ...base, detail, group, blocked: `It's in «${sealed.name}», which is sealed.`, fix: { label: `Open ${sealed.name}`, action: "open-pool", arg: sealed.id } };
    if (ended) return { ...base, detail, group, blocked: "Ended. Resume it to add it.", fix: { label: "Resume", action: "resume", arg: session.id } };
    return { ...base, detail, group, blocked: null };
  });
  const theirs: Candidate[] = input.conversations
    .filter((conversation) => conversation.session_id === null)
    .map((conversation) => {
      const project = conversation.project_id ?? null;
      const lastMs = conversation.last_activity_ms ?? conversation.started_ms ?? 0;
      const where = conversation.places[0]?.shown ?? "another app";
      const base = {
        ref: `conversation:${conversation.account_id}/${conversation.id}`,
        kind: "session" as const,
        label: conversation.title ?? "A conversation",
        detail: `${projectName(project)} · in ${where}`,
        group: (conversation.live ? "open" : now - lastMs < 7 * DAY ? "recent" : "all") as Candidate["group"],
        reach: "watched" as const,
        lastMs,
      };
      if (!inScope(draft, project)) return { ...base, blocked: `Works in ${projectName(project)}, not in this pool's projects.`, fix: { label: `Add ${projectName(project)}`, action: "scope" as const, arg: project ?? "" } };
      return { ...base, blocked: null, fix: { label: "Continue in AI Juggler", action: "continue" as const, arg: `${conversation.account_id}/${conversation.id}` } };
    });
  return [...own, ...theirs].sort((a, b) => b.lastMs - a.lastMs);
}

/** Apps, Accounts, Tools and Macs: broad members, whose sessions in scope
 * join by themselves, now and later. */
export function broadCandidates(input: {
  kind: Exclude<MemberKind, "session" | "everyone">;
  apps: AppModel[];
  subscriptions: SubscriptionInfo[];
  tools: ToolSpecInfo[];
  sessions: InstanceInfo[];
  thisMac: string;
  macName: (id: string) => string;
}): Candidate[] {
  const open = (match: (session: InstanceInfo) => boolean) => input.sessions.filter((session) => sessionState(session).state !== "ended" && match(session)).length;
  const row = (ref: string, kind: MemberKind, label: string, count: number, blocked: string | null = null): Candidate => ({
    ref,
    kind,
    label,
    detail: count > 0 ? `${count} open now · new ones join` : "new ones join",
    group: count > 0 ? "open" : "all",
    reach: "later",
    blocked,
    lastMs: count,
  });
  switch (input.kind) {
    case "app":
      return input.apps.map((app) => row(memberRef("app", app.id), "app", app.name, open((session) => app.folder !== null && session.account_id === app.folder.id)));
    case "account":
      // A signed-out group of folders has no Account behind it to add (the
      // P3 walk offered one named "Not signed in").
      return input.subscriptions.filter((account) => account.state !== "signed_out").map((account) => row(memberRef("account", account.key), "account", account.display_name, 0));
    case "tool":
      return input.tools.map((tool) => row(memberRef("tool", tool.id), "tool", tool.display_name, open((session) => session.provider === tool.id)));
    case "mac": {
      const macs = [...new Set(input.sessions.map((session) => session.machine_id))];
      if (!macs.includes(input.thisMac)) macs.unshift(input.thisMac);
      return macs.map((id) => row(memberRef("mac", id), "mac", input.macName(id), open((session) => session.machine_id === id), id === input.thisMac ? null : `Its sessions join once pools work across ${MACHINES_WORD} (0.5).`));
    }
  }
}

/** A talk grid's changes from allowed, to send. */
export function talkChanges(talk: Draft["talk"]): { from: string; to: string; rule: TalkRule }[] {
  const out: { from: string; to: string; rule: TalkRule }[] = [];
  for (const from of TALK_ROLES) {
    for (const to of TALK_ROLES) {
      const rule = ruleOf(talk, from, to);
      if (rule !== "allowed") out.push({ from: `role:${from}`, to: `role:${to}`, rule });
    }
  }
  return out;
}

/** Makes the pool the draft describes, or (`into`) sets an existing one
 * up as it says; its id. Every step is the user's own call; nothing
 * starts a session. */
export async function makePool(draft: Draft, into?: string): Promise<string> {
  let made: PoolInfo | undefined;
  if (into) made = { id: into } as PoolInfo;
  else {
    const before = await poolCreate(draft.name.trim(), draft.every || draft.projects.length !== 1 ? null : draft.projects[0] ?? null);
    made = before.pools.find((pool) => pool.name === draft.name.trim() && pool.kind === "user");
  }
  if (!made) throw new Error("The pool wasn't made");
  await poolSetScope(made.id, draft.every, draft.every ? [] : draft.projects);
  if (draft.level !== "work_together") await poolSetAccess(made.id, draft.level);
  for (const member of draft.members) await poolJoin(made.id, member.ref, member.role, member.level);
  for (const change of talkChanges(draft.talk)) await poolSetTalk(made.id, change.from, change.to, change.rule);
  if (draft.sealed) await poolSetSealed(made.id, true);
  if (draft.spansMacs) await poolSetSpansMacs(made.id, true);
  return made.id;
}

export { pairKey };

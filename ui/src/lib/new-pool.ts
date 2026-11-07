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
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ConversationInfo } from "./ipc/conversations";
import { memberRef, poolJoin, poolSetScope, poolSetSealed, poolSetTalk, type MemberKind, type TalkRule } from "./ipc/pool-model";
import { poolCreate, poolSetAccess, poolSetSpansMacs, type PoolInfo, type PoolRole, type PoolsInfo } from "./ipc/pools";
import { pairKey, ruleOf, templateById, TALK_ROLES, type PairKey, type PoolTemplate } from "./pool-templates";
import { sessionState } from "./rail-items";
import type { AppModel } from "./apps";
import type { SubscriptionInfo } from "./subscriptions";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import { MACHINE_WORD, MACHINES_WORD, THIS_MACHINE, THIS_MACHINE_IN_TEXT } from "./machine-words";

/** What a member can do in the pool. */
export type Reach = "full" | "shell" | "watched" | "later";

/** Each reach in a word, as the wizard's chip and the palette say it. */
export const REACH_WORDS: Record<Reach, string> = {
  full: "full tools",
  shell: "shell only",
  watched: "watched",
  later: "new ones join",
};

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

/** The template's roles no member holds yet, in its order ("lead",
 * "reviewer"): Apps and other broad members join as Members, so a
 * Writer and reviewer of two Apps holds neither until they're chosen
 * (the 7b walk: the card promised Lead and Reviewer, and both were
 * Member with nothing said). */
export function rolesToChoose(draft: { template: string; members: readonly { role: PoolRole }[] }): PoolRole[] {
  const held = new Set(draft.members.map((member) => member.role));
  return templateById(draft.template).roles.filter((role, at, all) => role !== "member" && all.indexOf(role) === at && !held.has(role));
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

/** The Mac most of the sessions here run on: this one. */
export function thisMacOf(sessions: InstanceInfo[]): string {
  const counts = new Map<string, number>();
  for (const session of sessions) counts.set(session.machine_id, (counts.get(session.machine_id) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1])[0]?.[0] ?? "this-mac";
}

/** A Mac's name here: This Mac, a paired Mac's own, or "A paired Mac". */
export function macLabel(id: string, thisMac: string, paired: { machine_id: string; label: string }[]): string {
  if (id === thisMac) return THIS_MACHINE;
  return paired.find((mac) => mac.machine_id === id)?.label ?? `A paired ${MACHINE_WORD}`;
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
  /** The Macs paired with this one (AJ-E1515), by their names here. */
  paired: { machine_id: string; label: string }[];
}): Candidate[] {
  const open = (match: (session: InstanceInfo) => boolean) => input.sessions.filter((session) => sessionState(session).state !== "ended" && match(session)).length;
  const row = (ref: string, kind: MemberKind, label: string, count: number, blocked: string | null = null): Candidate => ({
    ref,
    kind,
    label,
    // What's open now; that new ones join is the reach's own word (the
    // wizard's chip, the palette's line), said once (the P6 walk).
    detail: count > 0 ? `${count} open now` : "none open now",
    group: count > 0 ? "open" : "all",
    reach: "later",
    blocked,
    lastMs: count,
  });
  switch (input.kind) {
    case "app":
      return input.apps.map((app) => row(memberRef("app", app.id), "app", app.name, open((session) => app.folder !== null && session.account_id === app.folder.id)));
    case "account":
      // The stored Account, never the list's key (the pools L2 on the
      // replica: a member by the key matched no session). A signed-out
      // group of folders, or one whose sign-in can't be read, has no
      // Account behind it to add (the P3 walk offered "Not signed in").
      return input.subscriptions.flatMap((account) => {
        const id = account.account_id;
        if (account.state === "signed_out" || id === null || id === undefined) return [];
        const folders = new Set(account.profiles.map((profile) => profile.id));
        return [row(memberRef("account", id), "account", account.display_name, open((session) => session.account_id !== null && folders.has(session.account_id)))];
      });
    case "tool":
      return input.tools.map((tool) => row(memberRef("tool", tool.id), "tool", tool.display_name, open((session) => session.provider === tool.id)));
    case "mac": {
      // This Mac, then every paired one (the P6 walk: a paired Mac with no
      // sessions here was never offered). A paired Mac says it waits for
      // pools across Macs (AJ-E1705) and has no Add: nothing pretends.
      const macs = [input.thisMac, ...input.sessions.map((session) => session.machine_id), ...input.paired.map((mac) => mac.machine_id)].filter((id, at, all) => all.indexOf(id) === at);
      return macs.map((id) => {
        if (id === input.thisMac) return row(memberRef("mac", id), "mac", THIS_MACHINE, open((session) => session.machine_id === id));
        const name = macLabel(id, input.thisMac, input.paired);
        return { ...row(memberRef("mac", id), "mac", name, 0, `Its sessions join once pools work across ${MACHINES_WORD} (0.5).`), detail: `paired with ${THIS_MACHINE_IN_TEXT}`, group: "all" as const };
      });
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

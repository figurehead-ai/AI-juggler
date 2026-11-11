/**
 * D-62 (AJ-E3007): the New pool wizard's draft, what it can add, and the
 * pool it makes.
 *
 * - Candidates come from what exists, by kind (sessions, Apps, Accounts,
 *   Tools, Macs, everyone), Open now first, then Recent, then All. One
 *   that can't join says why and the step that would let it in.
 * - Each member says what it can do in the pool: full (a session AI
 *   Juggler started with its agent tools), shell only (its tool has no
 *   per-launch tools yet, its App can't take them, or the user switched
 *   them off for every session, its project or its App), watched (another app runs it), or later (a
 *   broad member: its sessions join by themselves).
 * - Making the pool is one sequence of the user's own calls: create,
 *   scope, members, talk, seal. Nothing starts a session.
 */
import { agentToolsWhy, shellOnlyWhy, switchedOffWhy } from "./agent-tools";
import type { AgentBridgeInfo } from "../types/generated/AgentBridgeInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ConversationInfo } from "./ipc/conversations";
import { memberRef, poolJoin, poolSetScope, poolSetSealed, poolSetTalk, type MemberKind, type TalkRule } from "./ipc/pool-model";
import { poolCreate, poolSetAccess, poolSetSpansMacs, type PoolInfo, type PoolRole, type PoolsInfo } from "./ipc/pools";
import { pairKey, ruleOf, templateById, TALK_ROLES, type PairKey, type PoolTemplate } from "./pool-templates";
import { sessionState } from "./rail-items";
import { namedByFacts, tellApart } from "./session-titles";
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
  /** A broad member whose sessions all work through their shell: why. */
  shell?: string | null;
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

/** D-92: when the template wants a Lead and no member picked can take it
 * (Apps, Accounts, Tools, Macs and everyone can't; a Watched session
 * can't act yet), how one is chosen, in plain words; `null` when a
 * member here can lead, or none is wanted (the rc26 P1 walk: Writer and
 * reviewer of two Apps said "Pick them in the list below", where no row
 * offered Lead). */
export function leadLater(draft: { template: string; members: readonly Pick<DraftMember, "kind" | "role" | "reach">[] }): string | null {
  if (draft.members.length === 0 || !rolesToChoose(draft).includes("lead")) return null;
  if (draft.members.some((member) => member.kind === "session" && member.reach !== "watched")) return null;
  return "No Lead yet: Lead is one session, and these join as a group. Once one of their sessions runs, click it on the pool's canvas and make it Lead; or go back and pick a session now.";
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
  /** What joining means for it, when that's less than full tools: a
   * desktop App's sessions are watched (D-62 decision 15). */
  note?: string;
  /** An App or Tool member whose sessions all work through their shell
   * only: why (the P8 walk: a Kimi Code App said nothing). */
  shell?: string | null;
  lastMs: number;
}

/** A desktop App member: it may join (decision 15 offers it, never
 * greyed), and says what its sessions can do here (the rc26 P1 walk: the
 * Claude app read "none open now · new ones join" beside Add, as if they
 * would act). */
function desktopAppNote(app: string): string {
  return `Runs in the ${app}: its sessions join watched (they can't reply) until continued in AI Juggler.`;
}

/** A session another app runs: it joins watched, and says it can't reply
 * here before it's added (P1 step 8). */
function watchedNote(where: string): string {
  return `Runs in ${where}: it can't reply here until it's continued in AI Juggler.`;
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
  /** The agent tools' switches (`useAgentBridge`); `null` while they load. */
  bridge?: AgentBridgeInfo | null;
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
  // D-96 D: alike names told apart, as the rail names them (the rc27 pool
  // pass: four untitled Lab sessions in one folder read the same here).
  const apart = tellApart(
    input.sessions.filter((session) => namedByFacts(session)),
    (session) => input.nameOf(session),
    (session) => [session.started_at_ms, session.id],
  );
  const own: Candidate[] = input.sessions.map((session) => {
    const state = sessionState(session);
    const ended = state.state === "ended";
    const lastMs = session.last_activity_ms ?? session.started_at_ms ?? 0;
    const project = session.project_id ?? null;
    const sealed = sealedPool(session.id);
    // AJ-E3002 R3: as its App's launches get them, not only its tool,
    // and as the switches allow for its own project (the rc18 walk).
    const full = agentTools.has(session.provider) && !agentToolsWhy(session.account_id) && !switchedOffWhy(input.bridge, session.account_id, project);
    const reach: Reach = full ? "full" : "shell";
    const base = { ref: memberRef("session", session.id), kind: "session" as const, label: apart.get(session) ?? input.nameOf(session), reach, lastMs };
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
        // P1 step 8 (the rc29 walk): its reason before it's added.
        note: watchedNote(where),
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
function macLabel(id: string, thisMac: string, paired: { machine_id: string; label: string }[]): string {
  if (id === thisMac) return THIS_MACHINE;
  return paired.find((mac) => mac.machine_id === id)?.label ?? `A paired ${MACHINE_WORD}`;
}

/** A stored member's name (`app:…`, `account:acct:…`, `tool:…`, `mac:…`,
 * `all`), from the same Apps, Accounts and Tools the other screens list.
 * An Account member is the stored Account, never the list's key (the
 * rc26 P5 walk: every Account member read "An Account"); one no App here
 * is signed in to readably now is "Account unknown" (AJ-E3004 req. 3). */
export function memberLabel(
  member: string,
  input: {
    apps: readonly Pick<AppModel, "id" | "name">[];
    subscriptions: readonly Pick<SubscriptionInfo, "account_id" | "display_name">[];
    tools: readonly Pick<ToolSpecInfo, "id" | "display_name">[];
    thisMac: string;
    paired: { machine_id: string; label: string }[];
  },
): string {
  if (member === "all") return "Everyone in this pool's projects";
  const [kind, ...rest] = member.split(":");
  const id = rest.join(":");
  if (kind === "app") return input.apps.find((app) => app.id === id)?.name ?? "An App";
  if (kind === "account") return input.subscriptions.find((account) => account.account_id === id)?.display_name ?? "Account unknown";
  if (kind === "tool") return input.tools.find((tool) => tool.id === id)?.display_name ?? "A tool";
  if (kind === "mac") return macLabel(id, input.thisMac, input.paired);
  return "A member";
}

/** What an App or Tool member's "Shell only" reads of an App. */
type AppTool = { id: string; kind: AppModel["kind"]; tool: string | null; folder: { id: string } | null };

/** The tool and App an App or Tool member's sessions all share, so its
 * "Shell only" answer is theirs (the P8 walk); `null` for a member whose
 * sessions may differ (an Account, a Mac, everyone) or a desktop App
 * (its sessions are watched, D-62 decision 15). */
export function memberTool(member: string, apps: readonly AppTool[]): { tool: string; account: string | null } | null {
  const [kind, ...rest] = member.split(":");
  const id = rest.join(":");
  if (kind === "tool") return { tool: id, account: null };
  if (kind !== "app") return null;
  const app = apps.find((one) => one.id === id);
  if (!app || app.kind !== "cli" || !app.tool) return null;
  return { tool: app.tool, account: app.folder?.id ?? null };
}

/** A broad member's "Shell only" reason, as its sessions' nodes give it;
 * `null` when they get the agent tools or it isn't one answer for all. */
export function memberShellWhy(
  member: string,
  input: { apps: readonly AppTool[]; tools: readonly { id: string; supports: { agent_tools: boolean } }[]; bridge?: AgentBridgeInfo | null },
): string | null {
  const shared = memberTool(member, input.apps);
  if (!shared) return null;
  const spec = input.tools.find((tool) => tool.id === shared.tool);
  return shellOnlyWhy(spec, agentToolsWhy(shared.account), input.bridge, shared.account, null);
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
  /** The agent tools' switches, for an App or Tool member's "Shell only". */
  bridge?: AgentBridgeInfo | null;
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
    shell: memberShellWhy(ref, input),
    lastMs: count,
  });
  switch (input.kind) {
    case "app":
      return input.apps.map((app) => {
        const one = row(memberRef("app", app.id), "app", app.name, open((session) => app.folder !== null && session.account_id === app.folder.id));
        return app.kind === "desktop" ? { ...one, note: desktopAppNote(app.app) } : one;
      });
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

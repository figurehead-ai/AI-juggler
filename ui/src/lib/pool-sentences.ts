/**
 * AJ-E3011 round 3: a pool read back in sentences, one way for both paths.
 * New pool's Review reads a draft, a made pool's inspector reads the pool;
 * each becomes a {@link PoolShape} first, so the summary on the diagram
 * and the Review can never say a pool differently (the walks found the
 * diagram path saying less than the setup).
 */
import type { PoolRole } from "./ipc/pools";
import type { TalkRule } from "./ipc/pool-model";
import { listWords } from "./account-terms";
import { effectiveRule, ruleOf, TALK_ROLES, type PairKey } from "./pool-templates";
import { TALK_WORDS, joinSentence, leadSentence, levelSentence, memberSentence, nextLeadsSentence, roleCard } from "./pool-words";

/** One member, by its name on screen. */
export interface ShapeMember {
  /** `session:…`, `app:…`, `account:…`, `tool:…`, `mac:…`, or `all`. */
  ref: string;
  label: string;
  role: PoolRole;
  /** Its own choice of what it may do; `null`: the pool's. */
  level: string | null;
  /** Its new sessions join by themselves (an App, Account, Tool, Mac). */
  later: boolean;
}

/** A pool, draft or made, as its sentences read it. */
export interface PoolShape {
  members: ShapeMember[];
  /** The App (`app:…`) whose next session leads, while no session does. */
  leadNext: string | null;
  talk: Partial<Record<PairKey, TalkRule>>;
  level: string;
  /** A Custom level's actions picked by hand, counted. */
  picked?: number;
  sealed: boolean;
}

/** At most this many names in "Who's in"; then "and N more". */
const NAMES = 3;

/** Who's in: its members by name (three, then "and N more"), and
 * whether new sessions join by themselves. */
export function whoSentence(shape: Pick<PoolShape, "members">): string {
  const named = shape.members.filter((member) => member.ref !== "all");
  const everyone = shape.members.some((member) => member.ref === "all");
  if (named.length === 0) return everyone ? "Every session in these projects, as it starts." : "No one yet: add members now, or on the diagram later.";
  const shown = named.slice(0, NAMES).map((member) => member.label);
  const names = named.length > NAMES ? `${shown.join(", ")} and ${named.length - NAMES} more` : shown.join(", ");
  if (everyone) return `${names}, and every other session in these projects as it starts.`;
  return `${names}. ${named.some((member) => member.later) ? "Their new sessions here join by themselves." : "Nothing else joins by itself."}`;
}

/** Who leads, read back: the session, the App that takes charge as soon
 * as one of its sessions starts in `where`, or none. */
export function leadLine(shape: Pick<PoolShape, "members" | "leadNext">, where: string): string {
  const lead = shape.members.find((member) => member.role === "lead" && member.ref.startsWith("session:"));
  if (lead) return leadSentence(lead.label, null);
  const next = shape.leadNext ? shape.members.find((member) => member.ref === shape.leadNext) : undefined;
  return leadSentence(null, next?.label ?? null, where);
}

/** Everyone else: how new sessions join, then each named member's role
 * (an App that leads next as one member). */
export function everyoneLines(shape: Pick<PoolShape, "members" | "leadNext" | "level" | "picked">, where: string): string[] {
  const everyone = shape.members.find((member) => member.ref === "all");
  return [
    joinSentence(where, everyone?.role ?? null, everyone?.level ?? shape.level, shape.picked),
    ...shape.members
      .filter((member) => member.ref !== "all" && member.role !== "lead")
      .map((member) => (member.ref === shape.leadNext ? nextLeadsSentence(member.label, member.role, member.level, shape.level, shape.picked) : memberSentence(member.label, member.role, member.level, shape.level, shape.picked))),
  ];
}

/** The talk grid in a sentence: "Everyone may message everyone" or its
 * exceptions, as the backend enforces them (an Observer's Allowed asks). */
export function talkSentence(shape: Pick<PoolShape, "talk">): string {
  // An Observer that may message no one "only reads", said once.
  const observerReads = TALK_ROLES.every((to) => ruleOf(shape.talk, "observer", to) === "blocked");
  const changed: string[] = [];
  for (const from of TALK_ROLES) {
    for (const to of TALK_ROLES) {
      const rule = effectiveRule(from, ruleOf(shape.talk, from, to));
      if (rule !== "allowed" && !(from === "observer" && observerReads)) changed.push(`${roleCard(from).label} to ${roleCard(to).label}: ${TALK_WORDS[rule].label.toLowerCase()}`);
    }
  }
  const reads = observerReads ? "; an Observer only reads" : "";
  return changed.length === 0 ? `Everyone may message everyone${reads}.` : `${changed.join("; ")}. The rest is allowed${reads}.`;
}

/** What they may do on their own. */
export const ownLine = (shape: Pick<PoolShape, "level" | "picked">): string => levelSentence(shape.level, shape.picked);

/** Whether it's kept apart. */
export const sealLine = (shape: Pick<PoolShape, "sealed">): string =>
  shape.sealed ? "Sealed: its sessions are in no other pool, and nothing leaves it through AI Juggler." : "Not sealed: its sessions can be in other pools too.";

/** What happens next, in one line, once the pool is ready (round 4 #3):
 * "Open Claude Code · Work in web first: it leads; Codex checks its work;
 * their messages show here". The diagram's checklist and Review say it
 * (Review as {@link nextParts}, a card each). */
export function nextLine(shape: Pick<PoolShape, "members" | "leadNext">, where: string): string {
  return nextParts(shape, where).join("; ");
}

/** With no lead: which new session joins by itself, if any (AJ-E3014
 * item 4: "Open a session in web: it joins" while every new session's
 * switch was off, and none would). */
function joinsFirst(shape: Pick<PoolShape, "members">, where: string): string {
  if (shape.members.some((member) => member.ref === "all")) return `Open a session in ${where}: it joins`;
  const laters = shape.members.filter((member) => member.later).map((member) => member.label);
  if (laters.length > 0) return `Open a session of ${listWords(laters)} in ${where}: it joins`;
  return `A new session in ${where} doesn't join by itself: add it on the diagram`;
}

/** What happens next, in parts: who to open first (and that it leads),
 * then who checks, then that their messages show here. */
export function nextParts(shape: Pick<PoolShape, "members" | "leadNext">, where: string): string[] {
  const session = shape.members.find((member) => member.role === "lead" && member.ref.startsWith("session:"));
  const next = shape.leadNext ? shape.members.find((member) => member.ref === shape.leadNext) : undefined;
  const reviewers = shape.members.filter((member) => member.role === "reviewer").map((member) => member.label);
  // Sam's round-4 walk: which one to open first, named.
  const parts = [session ? `${session.label} leads` : next ? `Open ${next.label} in ${where} first: it leads` : joinsFirst(shape, where)];
  if (reviewers.length > 0) parts.push(`${listWords(reviewers)} ${reviewers.length === 1 ? "checks" : "check"} ${session || next ? "its" : "their"} work`);
  parts.push("their messages show here");
  return parts;
}

/** The lead's first message, from the roles (round 6, Sam's walk): whom
 * to tell, and what — "Plan the work in web, hand out tasks, and ask
 * Codex to review." `null` without a lead. */
export function firstPrompt(shape: Pick<PoolShape, "members" | "leadNext">, where: string): { to: string; words: string } | null {
  const session = shape.members.find((member) => member.role === "lead" && member.ref.startsWith("session:"));
  const next = shape.leadNext ? shape.members.find((member) => member.ref === shape.leadNext) : undefined;
  const lead = session ?? next;
  if (!lead) return null;
  // A reviewer by its tool's name ("Codex"), not its whole App name;
  // round 7: by its whole name where another member shares that tool's
  // name (two profiles of the Claude app: "Claude app · Side", never
  // "Claude app" for both). Whole, not the screens' short profile name
  // ("Side"): these words go to the lead, which must tell them apart.
  const head = (label: string) => label.split(" · ")[0]!;
  const names = shape.members.map((member) => (shape.members.filter((one) => head(one.label) === head(member.label)).length > 1 ? member.label : head(member.label)));
  const reviewers = shape.members.flatMap((member, at) => (member.role === "reviewer" ? [names[at]!] : []));
  const review = reviewers.length > 0 ? `, and ask ${listWords(reviewers)} to review` : "";
  return { to: lead.label, words: `Plan the work in ${where}, hand out tasks${review}.` };
}

/** A made pool's talk rows (`role:…` pairs) as a grid. */
export function talkOf(rows: readonly { from: string; to: string; rule: string }[]): Partial<Record<PairKey, TalkRule>> {
  const out: Partial<Record<PairKey, TalkRule>> = {};
  for (const row of rows) {
    const from = TALK_ROLES.find((role) => `role:${role}` === row.from);
    const to = TALK_ROLES.find((role) => `role:${role}` === row.to);
    if (from && to && (row.rule === "ask" || row.rule === "blocked")) out[`${from}>${to}`] = row.rule;
  }
  return out;
}

/** A draft pool (New pool) as its sentences read it, each member by its
 * name on screen. */
export function shapeOfDraft(
  draft: { members: readonly { ref: string; label: string; role: PoolRole; level: string | null; reach: string }[]; leadNext: string | null; talk: Partial<Record<PairKey, TalkRule>>; level: string; sealed: boolean },
  nameOf: (ref: string, label: string) => string,
): PoolShape {
  return {
    members: draft.members.map((member) => ({ ref: member.ref, label: nameOf(member.ref, member.label), role: member.role, level: member.level, later: member.reach === "later" })),
    leadNext: draft.leadNext,
    talk: draft.talk,
    level: draft.level,
    sealed: draft.sealed,
  };
}

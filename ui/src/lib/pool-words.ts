/**
 * 0.4.6 (AJ-E3011): a pool's setup in plain words, for someone who has
 * never heard of pools, roles, leads or permission levels. One place for
 * every word the setup, the canvas and the inspector say about a role,
 * what sessions may do on their own, who may message whom and who leads,
 * so a control and its read-back sentence can never disagree.
 *
 * - "Level" is gone: the label is "What these sessions may do on their
 *   own", with Look only, Work together, and Work together and wake each
 *   other (D-28's Read, Work together and Full; waking is D-29 and D-119).
 * - Each role says what it may do in one line (role cards).
 * - Every choice reads back as a sentence ("New sessions in web join as
 *   Member and may Work together.").
 */
import type { PoolRole } from "./ipc/pools";
import type { TalkRule } from "./ipc/pool-model";

/** The guide's page on pools (opened in the browser by the app). */
export const POOLS_GUIDE = "pools";

/** What a pool is, why, and what happens once it's made (the intro). */
export const INTRO = {
  what: "A pool is a team of your sessions on the same projects.",
  why: "They share messages, notes and tasks, and you approve what matters.",
  after: "Once it's made, its sessions join as they start, and its canvas shows who does what.",
};

export interface RoleCard {
  id: PoolRole;
  label: string;
  /** What it may do, in one line. */
  line: string;
}

/** The four roles, each in one line (role cards and the role picker). */
export const ROLE_CARDS: readonly RoleCard[] = [
  { id: "lead", label: "Lead", line: "Plans the work and hands out tasks. One session." },
  { id: "member", label: "Member", line: "Works on the tasks it's given, and messages the team." },
  { id: "reviewer", label: "Reviewer", line: "Reads everything, checks work in review and gives its verdict. Doesn't build." },
  { id: "observer", label: "Observer", line: "Watches and reads. Changes nothing." },
];

export const roleCard = (role: string): RoleCard => ROLE_CARDS.find((card) => card.id === role) ?? ROLE_CARDS[1]!;

/** A stored role word as a role (a Member when it isn't one). */
export const asRole = (role: string): PoolRole => roleCard(role).id;

/** D-28's three levels, as the setup names them. */
export type OwnLevel = "read" | "work_together" | "full";

export interface OwnLevelWords {
  id: OwnLevel;
  label: string;
  /** Its consequence, one line. */
  line: string;
  /** An example of it. */
  example: string;
}

/** The label above the choice: what it sets, and for whom. */
export const OWN_LABEL = "What these sessions may do on their own";

/** "Same as the pool", a member's first choice. */
export const SAME_AS_POOL = "Same as the pool";

export const OWN_LEVELS: readonly OwnLevelWords[] = [
  {
    id: "read",
    label: "Look only",
    line: "They see the team, its cards, tasks and their own messages. Anything more waits for your OK in Needs you.",
    example: "Codex sees that Claude holds the login task, but asks you before it sends Claude a note.",
  },
  {
    id: "work_together",
    label: "Work together",
    line: "They message each other and share notes and tasks. Anything more waits for your OK in Needs you.",
    example: "Claude can see that Codex is reviewing and send it a note; Codex reads it on its next turn.",
  },
  {
    id: "full",
    label: "Work together and wake each other",
    line: "As Work together, and a message wakes an idle session so the work keeps going. Each wake starts a turn on your subscription.",
    example: "Claude finishes a change and its note starts a turn in the idle Codex session, without you.",
  },
];

export const ownLevel = (level: string | null | undefined): OwnLevelWords => OWN_LEVELS.find((one) => one.id === level) ?? OWN_LEVELS[1]!;

/** A stored level word as one of the three (Work together otherwise). */
export const asOwnLevel = (level: string | null | undefined): OwnLevel => ownLevel(level).id;

/** A level's name, Custom included (the header's picker offers it). */
export function ownLabel(level: string | null | undefined): string {
  if (level === "custom") return "Custom";
  return ownLevel(level).label;
}

/** Where, in words: "web", "web and docs", "web, docs and api", or
 * "every project". */
export function whereWords(every: boolean, names: readonly string[]): string {
  if (every) return "every project";
  if (names.length === 0) return "no project yet";
  if (names.length === 1) return names[0]!;
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** The "everyone" member's name for a scope: "Everyone in web". */
export function everyoneLabel(every: boolean, names: readonly string[]): string {
  return every ? "Everyone, in every project" : `Everyone in ${whereWords(false, names)}`;
}

/** The join line read back as a sentence: who joins by themselves, as
 * what, and what they may do on their own. `role` is the "everyone"
 * member's role; `null`: nobody joins but who's in. */
export function joinSentence(where: string, role: PoolRole | null, level: string): string {
  if (role === null) return `Only who's in joins: other sessions in ${where} stay out of this pool.`;
  return `New sessions in ${where} join as ${roleCard(role).label} and may ${ownLabel(level)}.`;
}

/** One member read back: "Codex CLI · Personal is a Reviewer and may
 * Look only." `level` is its own, `null`: the pool's. */
export function memberSentence(name: string, role: PoolRole, level: string | null, poolLevel: string): string {
  const may = level === null ? `${ownLabel(poolLevel)}, as the pool` : ownLabel(level);
  const article = role === "observer" ? "an" : "a";
  return role === "lead" ? `${name} leads and may ${may}.` : `${name} is ${article} ${roleCard(role).label} and may ${may}.`;
}

/** What a session may do on its own, read back for the pool. */
export function levelSentence(level: string): string {
  return `On their own they may ${ownLabel(level)}. ${ownLevel(level).line}`;
}

/** The lead, read back. `next`: the App whose next session leads. */
export function leadSentence(lead: string | null, next: string | null): string {
  if (lead) return `${lead} leads: it plans the work and hands out tasks.`;
  if (next) return `${next}'s next session here leads: it plans the work and hands out tasks.`;
  return NO_LEAD;
}

/** What a pool without a lead lacks, said where the lead would be. */
export const NO_LEAD = "No lead for now: nobody splits the work into tasks. Each session works on what you give it.";

/** What a lead would add (the picker's "No lead for now" line, the
 * canvas's lead placeholder). */
export const LEAD_WOULD = "A lead would split the work into tasks and hand them out. You can pick one any time.";

/** "[App]'s next session" as the picker and the canvas name it. */
export const nextSessionWords = (app: string): string => `${app}'s next session`;

/** The talk table's words for each rule, and their attention level:
 * Allowed is calm (L0), Ask first needs you (L2), Blocked stops (L3). */
export const TALK_WORDS: Record<TalkRule, { label: string; level: 0 | 2 | 3 }> = {
  allowed: { label: "Allowed", level: 0 },
  ask: { label: "Ask first", level: 2 },
  blocked: { label: "Blocked", level: 3 },
};

/** A stored talk rule as a rule (allowed otherwise). */
export const asTalkRule = (rule: string | null | undefined): TalkRule => (rule === "ask" || rule === "blocked" ? rule : "allowed");

/** The line above the talk table. */
export const TALK_ABOVE = "Who may send a message to whom. Allowed needs nothing from you; Ask first waits for your OK in Needs you; Blocked stops it.";
/** The line below it: one pair of sessions. */
export const TALK_BELOW = "To set it for one pair of sessions, click the line between them on the canvas.";

/** A row's head: "Lead can message…". */
export function talkRowLabel(role: PoolRole): string {
  return `${roleCard(role).label} can message…`;
}

/** A column's head: "the Lead", "Members"… */
export function talkColumnLabel(role: PoolRole): string {
  return role === "lead" ? "the Lead" : `${roleCard(role).label}s`;
}

/** D-119: how a member hears a message, from what it may do on its own
 * (its own, else the pool's): at Work together and wake each other an
 * idle session wakes; otherwise it reads them on its next turn. A Codex
 * app thread can't be woken (one shared app-server). */
export function wakeWords(level: string, cantWake = false, many = false): string {
  if (cantWake) return "can't be woken";
  if (many) return level === "full" ? "wake on new messages" : "read messages on their next turn";
  return level === "full" ? "wakes on new messages" : "reads messages on its next turn";
}

/** The setup's steps, in order, each with its heading and one line on
 * why (the setup's steps and the canvas's checklist say the same). */
export const SETUP_STEPS = ["where", "who", "lead", "everyone", "talk", "own"] as const;
export type SetupStep = (typeof SETUP_STEPS)[number] | "template" | "review";

export const STEP_WORDS: Record<SetupStep, { title: string; nav: string; why: string }> = {
  template: { title: "How to start", nav: "Start", why: "A template fills in roles and who may message whom; you can change any of it." },
  where: { title: "Where", nav: "Where", why: "Sessions in these projects can join." },
  who: { title: "Who's in", nav: "Who's in", why: "Only these work together." },
  lead: { title: "Who leads", nav: "Who leads", why: "The lead plans and hands out the work." },
  everyone: { title: "Everyone else", nav: "Everyone else", why: "What each member does, and how new sessions join." },
  talk: { title: "Who may message whom", nav: "Who may message whom", why: "Set an exception only where you want one; the rest is allowed." },
  own: { title: "What they may do on their own", nav: "On their own", why: "Anything more waits for your OK in Needs you." },
  review: { title: "Review", nav: "Review", why: "The whole pool in sentences, before it's made." },
};

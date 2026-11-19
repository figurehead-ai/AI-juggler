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
import { observerReadsOnly, ruleOf, TALK_ROLES, type PairKey } from "./pool-templates";

/** The guide's page on pools (opened in the browser by the app). */
export const POOLS_GUIDE = "pools";

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

/** What "on their own" covers, and what it leaves to each app (round 2,
 * #4: the walks read it as covering what they may do to the files). */
export const OWN_SCOPE = "This is how they work together through AI Juggler. What each may change in your files stays with its own app's settings.";

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
    // Its line above says each wake's cost.
    example: "Codex finishes a review; its note wakes Claude's idle session to carry on, without you.",
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

/** What they may do, in a sentence's words: a level's name, or for
 * Custom the actions picked by hand ("do the 11 things you picked"; the
 * Sam walk read "they may Custom"). */
export function ownMay(level: string | null | undefined, picked?: number): string {
  if (level !== "custom") return ownLabel(level);
  if (picked === undefined) return "do the things you picked";
  return picked === 1 ? "do the 1 thing you picked" : `do the ${picked} things you picked`;
}

/** A Custom pool's actions picked by hand, counted as its Custom list
 * shows them (messages into sessions are their own setting); `undefined`
 * at another level. */
export function pickedCount(pool: { access: { level: string; actions: readonly string[] } }): number | undefined {
  return pool.access.level === "custom" ? pool.access.actions.filter((action) => action !== "deliver_to_session").length : undefined;
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
export function joinSentence(where: string, role: PoolRole | null, level: string, picked?: number): string {
  if (role === null) return `Only who's in joins: other sessions in ${where} stay out of this pool.`;
  return `New sessions in ${where} join as ${roleCard(role).label} and may ${ownMay(level, picked)}.`;
}

/** One member read back: "Codex CLI · Personal is a Reviewer and may
 * Look only." `level` is its own, `null`: the pool's. */
export function memberSentence(name: string, role: PoolRole, level: string | null, poolLevel: string, picked?: number): string {
  const may = level === null ? `${ownMay(poolLevel, picked)}, as the pool` : ownLabel(level);
  const article = role === "observer" ? "an" : "a";
  return role === "lead" ? `${name} leads and may ${may}.` : `${name} is ${article} ${roleCard(role).label} and may ${may}.`;
}

/** What a session may do on its own, read back for the pool. */
export function levelSentence(level: string, picked?: number): string {
  if (level === "custom") return `On their own they may ${ownMay(level, picked)}, under Custom actions.`;
  return `On their own they may ${ownLabel(level)}. ${ownLevel(level).line}`;
}

/** An App whose next session will lead, in short (its card, its row, its
 * inspector's subtitle; round 4 #4: "Leads next" read as "in charge now"). */
export const LEADS_WHEN = "Lead when a session starts";

/** The lead, read back. `next`: the App that takes charge as soon as one
 * of its sessions starts in `where` (round 4 #4, the shared lead control's
 * own words in both paths). */
export function leadSentence(lead: string | null, next: string | null, where = "these projects"): string {
  if (lead) return `${lead} leads: it plans the work and hands out tasks.`;
  if (next) return `${next} takes charge as soon as one of its sessions starts in ${where}: it plans the work and hands out tasks.`;
  return NO_LEAD;
}

/** The lead read back when it has ended (the 0.4.6 RC walk X1: the panel
 * said "No lead for now" under a diagram whose card said Lead). */
export function endedLeadSentence(lead: string): string {
  return `${lead} leads, but it has ended: nobody plans the work until you pick an open session or an App's next one.`;
}

/** An App whose next session leads, as one member (round 2, #2): its
 * role line, the others' role after it. */
export function nextLeadsLine(othersRole: string): string {
  return `Takes charge as soon as one of its sessions starts · its others join as ${roleCard(othersRole).label}`;
}

/** That App read back: "Claude Code · Work takes charge as soon as one of
 * its sessions starts; its others join as Member and may Work together,
 * as the pool." */
export function nextLeadsSentence(name: string, othersRole: string, level: string | null, poolLevel: string, picked?: number): string {
  const may = level === null ? `${ownMay(poolLevel, picked)}, as the pool` : ownLabel(level);
  return `${name} takes charge as soon as one of its sessions starts; its others join as ${roleCard(othersRole).label} and may ${may}.`;
}

/** What a pool without a lead lacks, said where the lead would be. */
export const NO_LEAD = "No lead for now: nobody splits the work into tasks. Each session works on what you give it.";

/** What a lead would add (the picker's "No lead for now" line, the
 * canvas's lead placeholder). */
export const LEAD_WOULD = "A lead would split the work into tasks and hand them out. You can pick one any time.";

/** "[App]'s next session" as the picker and the canvas name it. */
export const nextSessionWords = (app: string): string => `${app}'s next session`;

/** No lead chosen, as a step's value and New pool's drawing say it. */
export const NO_LEAD_YET = "No lead yet";

/** An App whose next session leads, under its name on a card: "Lead ·
 * its next session" (New pool's preview), "… in web" (the diagram's
 * planned team). */
export const leadNextLine = (where?: string): string => `Lead · its next session${where ? ` in ${where}` : ""}`;

/** Every new session in the pool's projects, under its name (Who builds'
 * row, the planned team's card). */
export const JOINS_BY_ITSELF = "Joins by itself, now and later";

/** An App's kind in a word (Who's in's cards, Who builds' rows, the
 * planned team's cards). */
export const APP_KIND_WORDS: Record<"desktop" | "cli", string> = { desktop: "Desktop app", cli: "Terminal" };

/** The talk table's words for each rule, and their attention level:
 * Allowed is calm (L0), Ask first needs you (L2), Blocked stops (L3). */
export const TALK_WORDS: Record<TalkRule, { label: string; level: 0 | 2 | 3 }> = {
  allowed: { label: "Allowed", level: 0 },
  ask: { label: "Ask first", level: 2 },
  blocked: { label: "Blocked", level: 3 },
};

/** A stored talk rule as a rule (allowed otherwise). */
export const asTalkRule = (rule: string | null | undefined): TalkRule => (rule === "ask" || rule === "blocked" ? rule : "allowed");

/** D-126: who may message whom, three ways (New pool's cards, the value
 * under Messages on both paths). Each is a whole talk grid of the rules
 * there are (no new backend). */
export type TalkPreset = "free" | "ask" | "lead";

export interface TalkPresetWords {
  id: TalkPreset;
  label: string;
  line: string;
  /** Its value under the step's name. */
  short: string;
  grid: Partial<Record<PairKey, TalkRule>>;
}

export const TALK_PRESETS: readonly TalkPresetWords[] = [
  { id: "free", label: "Everyone talks freely", line: "Any member may message any other.", short: "Everyone, freely", grid: observerReadsOnly },
  { id: "ask", label: "Members ask you first", line: "Between members, each message waits for your OK.", short: "Members ask you first", grid: { ...observerReadsOnly, "member>member": "ask" } },
  {
    id: "lead",
    label: "Members go through the lead",
    line: "Members message only the lead and the reviewer.",
    short: "Through the lead",
    grid: { ...observerReadsOnly, "member>member": "blocked", "member>observer": "blocked" },
  },
];

/** A grid no way matches (a cell changed by hand). */
export const OWN_MIX = "Your own mix";

/** The way a talk grid is, cell for cell; `null`: none (its own mix). An
 * Observer only reads (its messages are blocked or wait for you), so its
 * row is no exception and isn't compared. */
export function presetOf(talk: Partial<Record<PairKey, TalkRule>>): TalkPreset | null {
  const senders = TALK_ROLES.filter((from) => from !== "observer");
  const same = (grid: Partial<Record<PairKey, TalkRule>>) => senders.every((from) => TALK_ROLES.every((to) => ruleOf(talk, from, to) === ruleOf(grid, from, to)));
  return TALK_PRESETS.find((preset) => same(preset.grid))?.id ?? null;
}

/** What "Needs you" is, said at its first mention on the pool screens
 * (round 3, #5: the walk didn't know it). */
export const NEEDS_YOU_WHAT = "the list of things waiting for your OK, on Home";

/** The line above the talk table (the setup's first mention of Needs you). */
export const TALK_ABOVE = `Who may send a message to whom. Click a cell to change it. Allowed needs nothing from you; Ask first waits in Needs you (${NEEDS_YOU_WHAT}); Blocked stops it.`;
/** The line below it: one pair of sessions. */
export const TALK_BELOW = "An Observer only reads: its messages are blocked, or wait for your OK if you set Ask first. To set it for one pair of sessions, click the line between them on the diagram.";

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
export type SetupStep = (typeof SETUP_STEPS)[number] | "template" | "tools" | "review";

/** Each step's heading, its name in New pool's steps (`nav`), its short
 * name (`short`: the canvas checklist's rows and New pool's preview rows,
 * D-126's mock-ups) and one line on why. */
export const STEP_WORDS: Record<SetupStep, { title: string; nav: string; short: string; why: string }> = {
  template: { title: "How to start", nav: "Start", short: "Start", why: "A template fills in roles and who may message whom; you can change any of it." },
  where: { title: "Where", nav: "Where", short: "Where", why: "Sessions in these projects can join." },
  who: { title: "Who's in", nav: "Who's in", short: "Who's in", why: "Only these work together." },
  lead: { title: "Who leads", nav: "Who leads", short: "Who leads", why: "The lead plans and hands out the work. Pick one to be in charge, or leave it for now." },
  // Round 7 (Sam's walk): the title says what's chosen here, the checker
  // too (it read "Everyone else", and he missed where the reviewer is).
  everyone: { title: "Who builds, who checks", nav: "Who builds, who checks", short: "Roles", why: "Each member's role, and how new sessions join." },
  talk: { title: "Who may message whom", nav: "Who may message whom", short: "Messages", why: "Set an exception only where you want one; the rest is allowed." },
  own: { title: "What they may do on their own", nav: "On their own", short: "On their own", why: "Anything more waits for your OK in Needs you." },
  // Round 4 (D-30): the one question, its own step while unanswered.
  tools: { title: "Team tools", nav: "Team tools", short: "Team tools", why: "Asked once, the first time you make a pool." },
  review: { title: "Review", nav: "Review", short: "Review", why: "The whole pool in sentences, before it's made." },
};

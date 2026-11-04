/**
 * D-62 decision 18 (AJ-E3007): the New pool wizard's templates, as data,
 * so one is added without a new screen. Each comes from a use case in
 * `docs/research/pools-model.md` §1.3: what it's for, the member kinds it
 * asks for, its roles and its talk between roles, its level, whether it's
 * sealed or spans Macs. Everything a template sets is shown before the
 * pool is made, and changed on the canvas later.
 */
import type { PoolRole } from "./ipc/pools";
import type { MemberKind, TalkRule } from "./ipc/pool-model";
import { MACHINE_WORD, MACHINES_WORD } from "./machine-words";

/** The four roles, in the talk grid's order. */
export const TALK_ROLES: readonly PoolRole[] = ["lead", "member", "reviewer", "observer"];

/** A role pair's key in a talk grid: `from>to`. */
export type PairKey = `${PoolRole}>${PoolRole}`;
export const pairKey = (from: PoolRole, to: PoolRole): PairKey => `${from}>${to}`;

/** A chip on a template's card: its words and how it reads. */
export interface TemplateChip {
  label: string;
  tone: "accent" | "live" | "success" | "warning" | "danger" | "plain";
}

export interface PoolTemplate {
  id: string;
  name: string;
  /** What it's for, one line. */
  description: string;
  chips: TemplateChip[];
  /** Where it works first: one project, several, or every project. */
  scope: "one" | "several" | "every";
  /** The member kinds step 2 opens on, first the one it's about. */
  kinds: MemberKind[];
  /** The roles the first members it's about take, in order (the rest are
   * Members). */
  roles: PoolRole[];
  /** A broad member's role (everyone, an Account…), when it adds one. */
  broadRole?: PoolRole;
  /** The pool's level (D-28). */
  level: "read" | "work_together" | "full";
  /** Talk between roles it sets (anything else is allowed). */
  talk: Partial<Record<PairKey, TalkRule>>;
  sealed?: boolean;
  spansMacs?: boolean;
  /** What it needs that isn't there yet, in words (shown on its card). */
  waits?: string;
}

/** What a role may do in the talk grid: an Observer only reads. */
const observerReadsOnly: Partial<Record<PairKey, TalkRule>> = {
  "observer>lead": "blocked",
  "observer>member": "blocked",
  "observer>reviewer": "blocked",
};

export const POOL_TEMPLATES: readonly PoolTemplate[] = [
  {
    id: "writer-reviewer",
    name: "Writer and reviewer",
    description: "One session writes, another checks with fresh eyes.",
    chips: [
      { label: "Lead", tone: "accent" },
      { label: "Reviewer", tone: "live" },
      { label: "talk: allowed", tone: "success" },
    ],
    scope: "one",
    kinds: ["session"],
    roles: ["lead", "reviewer"],
    level: "work_together",
    talk: observerReadsOnly,
  },
  {
    id: "feature-team",
    name: "Feature team",
    description: "A lead and 3 workers on one project, each in its own worktree.",
    chips: [
      { label: "Lead", tone: "accent" },
      { label: "3 Members", tone: "plain" },
      { label: "member↔member: ask first", tone: "warning" },
    ],
    scope: "one",
    kinds: ["session"],
    roles: ["lead"],
    level: "work_together",
    talk: { ...observerReadsOnly, "member>member": "ask" },
  },
  {
    id: "across-repos",
    name: "Across repositories",
    description: "Agents in two or more projects keep each other up to date.",
    chips: [
      { label: "2+ projects", tone: "plain" },
      { label: "talk: allowed", tone: "success" },
    ],
    scope: "several",
    kinds: ["app", "session"],
    roles: ["lead"],
    level: "work_together",
    talk: observerReadsOnly,
  },
  {
    id: "reviewer-for-everything",
    name: "A reviewer for everything",
    description: "One reviewer on all your work, in every project.",
    chips: [
      { label: "every project", tone: "plain" },
      { label: "Reviewer", tone: "live" },
      { label: "member↔member: blocked", tone: "danger" },
    ],
    scope: "every",
    kinds: ["session", "everyone"],
    roles: ["reviewer"],
    broadRole: "member",
    level: "work_together",
    talk: { ...observerReadsOnly, "member>member": "blocked" },
  },
  {
    id: "rules-by-account",
    name: "Rules by account",
    description: "Every session on one account follows the same rules.",
    chips: [
      { label: "account member", tone: "plain" },
      { label: "level: Read", tone: "warning" },
    ],
    scope: "every",
    kinds: ["account"],
    roles: [],
    broadRole: "member",
    level: "read",
    talk: observerReadsOnly,
  },
  {
    id: "build-mac",
    name: `Build ${MACHINE_WORD}`,
    description: `A paired ${MACHINE_WORD} runs builds and tests for this project.`,
    chips: [
      { label: `${MACHINE_WORD} member`, tone: "plain" },
      { label: `spans ${MACHINES_WORD}`, tone: "live" },
    ],
    scope: "one",
    kinds: ["mac", "session"],
    roles: ["lead"],
    broadRole: "member",
    level: "work_together",
    talk: observerReadsOnly,
    spansMacs: true,
    waits: `A paired ${MACHINE_WORD}'s sessions join once pools work across ${MACHINES_WORD} (0.5).`,
  },
  {
    id: "sealed-client",
    name: "Sealed client project",
    description: "Nothing goes in or out. Its sessions are in no other pool.",
    chips: [
      { label: "sealed", tone: "danger" },
      { label: "one project", tone: "plain" },
    ],
    scope: "one",
    kinds: ["everyone", "app"],
    roles: [],
    broadRole: "member",
    level: "work_together",
    talk: observerReadsOnly,
    sealed: true,
  },
  {
    id: "scouts",
    name: "Scouts",
    description: "3–5 agents chase different ideas and challenge each other.",
    chips: [
      { label: "Lead", tone: "accent" },
      { label: "shared docs", tone: "plain" },
      { label: "talk: allowed", tone: "success" },
    ],
    scope: "one",
    kinds: ["session"],
    roles: ["lead"],
    level: "work_together",
    talk: observerReadsOnly,
  },
  {
    id: "blank",
    name: "Blank",
    description: "Start empty and set everything yourself.",
    chips: [],
    scope: "one",
    kinds: ["session"],
    roles: [],
    level: "work_together",
    talk: {},
  },
];

export function templateById(id: string): PoolTemplate {
  return POOL_TEMPLATES.find((one) => one.id === id) ?? POOL_TEMPLATES[POOL_TEMPLATES.length - 1]!;
}

/** The talk rule a grid says for a pair: its own, else allowed. */
export function ruleOf(talk: Partial<Record<PairKey, TalkRule>>, from: PoolRole, to: PoolRole): TalkRule {
  return talk[pairKey(from, to)] ?? "allowed";
}

/** The next rule when a grid cell is clicked: allowed → ask → blocked. */
export function nextRule(rule: TalkRule): TalkRule {
  return rule === "allowed" ? "ask" : rule === "ask" ? "blocked" : "allowed";
}

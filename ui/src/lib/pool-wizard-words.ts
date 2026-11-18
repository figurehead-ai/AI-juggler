/**
 * 0.4.6 (D-126): New pool's own words, as its mock-ups say them. The
 * shared words (a role's line, a level's, the read-back sentences) stay
 * in pool-words and pool-sentences, so the setup and the diagram never
 * disagree; this file holds what only the wizard says:
 *
 * - each step's question and its one line (the steps' names stay
 *   STEP_WORDS', the canvas checklist's too);
 * - the three groups of steps (The basics, How they work, Finish);
 * - each step's value under its name in the steps, and the preview's;
 * - the Messages presets, which map onto the talk rules there are (no
 *   new backend): a preset is a whole grid, the grid stays editable,
 *   and a grid no preset matches reads "Your own mix".
 */
import type { PoolRole } from "./ipc/pools";
import type { TalkRule } from "./ipc/pool-model";
import { observerReadsOnly, ruleOf, TALK_ROLES, type PairKey } from "./pool-templates";
import { nextSessionWords, ownLabel, POOLS_GUIDE, type SetupStep } from "./pool-words";

export type WizardGroup = "basics" | "how" | "finish";

/** The steps' groups, in order (the wizard's side). */
export const WIZARD_GROUPS: readonly { id: WizardGroup; label: string }[] = [
  { id: "basics", label: "The basics" },
  { id: "how", label: "How they work" },
  { id: "finish", label: "Finish" },
];

export const GROUP_OF: Record<SetupStep, WizardGroup> = {
  template: "basics",
  where: "basics",
  who: "basics",
  lead: "how",
  everyone: "how",
  talk: "how",
  own: "how",
  tools: "finish",
  review: "finish",
};

/** "How they work" is prefilled: said beside its name until it's reached. */
export const PREFILLED = "prefilled";

export interface WizardHead {
  /** The step's question, its page title. */
  title: string;
  /** One line on why. */
  why: string;
  /** A guide link after the line ("What a lead does"), and its section. */
  learn?: { label: string; guide: string };
}

/** Each step's question and its one line (the mock-ups' words). */
export const WIZARD_HEADS: Record<SetupStep, WizardHead> = {
  template: {
    title: "How do you want to start?",
    why: "A pool is a team of your AI sessions on the same project. Start from scratch and each step explains itself, or pick a team that is already set up.",
  },
  where: { title: "Name it and pick its project", why: "Only sessions working in these projects can join this pool." },
  who: { title: "Who's in this team?", why: "Add the apps whose sessions should work together. Each one you add can get its own role in the next steps, such as the reviewer." },
  lead: {
    title: "Who leads the team?",
    why: "The lead turns what you ask into tasks and hands them out. You can leave this for later.",
    learn: { label: "What a lead does", guide: `${POOLS_GUIDE}#make-a-pool` },
  },
  everyone: {
    title: "Who builds, and who checks?",
    why: "Give each member a role. A reviewer looks over the others' work before it counts as done.",
    learn: { label: "About roles", guide: `${POOLS_GUIDE}#make-a-pool` },
  },
  talk: { title: "Who may message whom?", why: "Most teams let everyone talk. Pick a stricter way only if you want to approve some messages first." },
  own: { title: "What may they do on their own?", why: "How much they do together without asking you. Anything beyond this waits for your OK in Needs you." },
  tools: { title: "Let your assistants message each other?", why: "Your team is set up. One last switch lets its sessions actually talk. You answer this once, for all your pools." },
  review: { title: "Ready to make it", why: "Here is the whole team. Change anything now, or later on its diagram." },
};

/** Review's title, with the pool's name. */
export const reviewTitle = (name: string): string => `Ready to make “${name}”`;

/** "Step 3 of 9 · The basics". */
export function eyebrowOf(steps: readonly SetupStep[], step: SetupStep): string {
  const group = WIZARD_GROUPS.find((one) => one.id === GROUP_OF[step])!;
  return `Step ${steps.indexOf(step) + 1} of ${steps.length} · ${group.label}`;
}

/** The start step's side: what a pool does, in four lines. */
export const POOL_DOES: readonly { id: "you" | "lead" | "talk" | "check"; line: string }[] = [
  { id: "you", line: "You give the work to the team, or to its lead." },
  { id: "lead", line: "A lead can split it into tasks and hand them out." },
  { id: "talk", line: "They message each other and share notes and tasks." },
  { id: "check", line: "A reviewer can check the work. Anything more asks you first." },
];

/** How a lead works, in three steps (Who leads' picture). */
export const LEAD_STEPS: readonly string[] = ["You tell the lead what you want.", "It splits that into tasks for the others.", "You approve anything that needs you."];

/** What happens once the pool is made (Review), as three cards. */
export function nextSteps(lead: string | null, reviewers: readonly string[], where: string, names: string): { title: string; line: string }[] {
  const check = reviewers.length === 0 ? "" : `; ${reviewers.join(" and ")} ${reviewers.length === 1 ? "checks" : "check"}`;
  return [
    { title: "The pool is made", line: `Its diagram opens with ${names}.` },
    lead ? { title: "Start the lead", line: `Start ${lead}'s session in ${where}. It takes the lead.` } : { title: "Start a session", line: `Start a session in ${where}. It joins the team.` },
    { title: "Tell it the goal", line: lead ? `It hands out tasks${check}. You approve in Needs you.` : `Each works on what you give it${check}. You approve in Needs you.` },
  ];
}

/* ---------------- Messages: the presets ---------------- */

export type TalkPreset = "free" | "ask" | "lead";

export interface TalkPresetWords {
  id: TalkPreset;
  label: string;
  line: string;
  /** Its value under the step's name. */
  short: string;
  /** The talk grid it sets: the rules there are, nothing new. */
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

/** A grid no preset matches (a cell changed by hand). */
export const OWN_MIX = "Your own mix";

/** The preset a talk grid is, cell for cell; `null`: none (its own mix). */
export function presetOf(talk: Partial<Record<PairKey, TalkRule>>): TalkPreset | null {
  const same = (grid: Partial<Record<PairKey, TalkRule>>) => TALK_ROLES.every((from) => TALK_ROLES.every((to) => ruleOf(talk, from, to) === ruleOf(grid, from, to)));
  return TALK_PRESETS.find((preset) => same(preset.grid))?.id ?? null;
}

/* ---------------- Values: under each step's name, and the preview's ---------------- */

/** A member by the name its card has, short where nothing else shares it:
 * "Codex" for "Codex · Personal", but both "Claude app · Work" and
 * "Claude app · Side" in full (round 7's rule). */
export function shortNames(labels: readonly string[]): string[] {
  const short = (label: string) => label.split(" · ")[0]!;
  return labels.map((label) => (labels.filter((one) => short(one) === short(label)).length > 1 ? label : short(label)));
}

/** "A", "A and B", "A, B and C". */
export const listWords = (names: readonly string[]): string => (names.length <= 1 ? (names[0] ?? "") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`);

export interface DraftView {
  /** Who's named in it (everyone aside), by name on screen. */
  named: readonly { ref: string; name: string; role: PoolRole }[];
  /** Everyone in its projects joins too. */
  everyone: boolean;
  /** Its projects in words ("web"). */
  where: string;
  /** The name of who leads: a session's, or the App whose next session does. */
  lead: { name: string; next: boolean } | null;
  level: string;
  talk: Partial<Record<PairKey, TalkRule>>;
}

/** Who's in, short: "Claude, Codex, everyone in web" (three names, then
 * "+2"). */
export function whoShort(view: Pick<DraftView, "named" | "everyone" | "where">): string {
  const names = shortNames(view.named.map((member) => member.name));
  const shown = names.length > 3 ? [...names.slice(0, 3), `+${names.length - 3}`] : names;
  const all = view.everyone ? [...shown, `everyone in ${view.where}`] : shown;
  return all.length === 0 ? "No one yet" : all.join(", ");
}

/** Who's in, as a line: "Claude and Codex, and every new session in web". */
export function whoLine(view: Pick<DraftView, "named" | "everyone" | "where">): string {
  const names = listWords(shortNames(view.named.map((member) => member.name)));
  if (!view.everyone) return names || "No one yet";
  return names ? `${names}, and every new session in ${view.where}` : `Every new session in ${view.where}`;
}

/** The lead, short ("Claude's next session"), or none yet. */
export function leadShort(view: Pick<DraftView, "lead" | "named">): string {
  if (!view.lead) return "No lead yet";
  const [name] = shortNames([view.lead.name, ...view.named.map((member) => member.name).filter((one) => one !== view.lead!.name)]);
  return view.lead.next ? nextSessionWords(name!) : name!;
}

/** Who checks: "Codex checks the work", else "Everyone builds". */
export function rolesShort(view: Pick<DraftView, "named">): string {
  const all = shortNames(view.named.map((member) => member.name));
  const reviewers = view.named.flatMap((member, at) => (member.role === "reviewer" ? [all[at]!] : []));
  if (reviewers.length === 0) return "Everyone builds";
  return `${listWords(reviewers)} ${reviewers.length === 1 ? "checks" : "check"} the work`;
}

/** Roles, as the preview's line: "Codex checks the work; the rest
 * build", else "Everyone builds (Member)". */
export function rolesLine(view: Pick<DraftView, "named">): string {
  const short = rolesShort(view);
  return short === "Everyone builds" ? "Everyone builds (Member)" : `${short}; the rest build`;
}

/** Who may message whom, short: the preset's own words, or its own mix. */
export function talkShort(talk: Partial<Record<PairKey, TalkRule>>): string {
  const preset = presetOf(talk);
  return TALK_PRESETS.find((one) => one.id === preset)?.short ?? OWN_MIX;
}

/** Each step's value under its name. `null` for a step with none. */
export function stepValue(step: SetupStep, view: DraftView & { template: string | null; tools: string | null }): string | null {
  switch (step) {
    case "template":
      return view.template;
    case "where":
      return view.where;
    case "who":
      return whoShort(view);
    case "lead":
      return leadShort(view);
    case "everyone":
      return rolesShort(view);
    case "talk":
      return talkShort(view.talk);
    case "own":
      return ownLabel(view.level);
    case "tools":
      return view.tools;
    default:
      return null;
  }
}

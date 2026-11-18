/**
 * 0.4.6 (D-126): New pool's own words, as its mock-ups say them. The
 * shared words (a role's line, a lead's, a level's, the Messages ways)
 * are pool-words', each step's value pool-values', the read-back
 * sentences pool-sentences', so the setup and the diagram never
 * disagree; this file holds what only the wizard says:
 *
 * - each step's question and its one line (the steps' names stay
 *   STEP_WORDS', the canvas checklist's too);
 * - the three groups of steps (The basics, How they work, Finish);
 * - How to start's and Team tools' values under their names.
 */
import type { PoolShape } from "./pool-sentences";
import { stepValues } from "./pool-values";
import { POOLS_GUIDE, type SetupStep } from "./pool-words";

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

/* ---------------- Each step's value, under its name ---------------- */

/** Each step's value under its name in the steps: the setup's six from
 * the shared values (the canvas checklist's words), How to start's and
 * Team tools' the wizard's own. `null` for a step with none. */
export function stepValue(step: SetupStep, draft: { shape: PoolShape; where: string; template: string | null; tools: string | null }): string | null {
  if (step === "template") return draft.template;
  if (step === "tools") return draft.tools;
  if (step === "review") return null;
  return stepValues(draft.shape, draft.where)[step];
}

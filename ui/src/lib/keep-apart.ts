/**
 * R4 (the 0.4.6 persona walks; main's ruling): "Keep it apart…" on a
 * project is one confirmation, not New pool's nine steps. Three testers
 * expected one, and got the wizard (clarity 2/7). The sheet says what it
 * does, in plain words. Keep apart makes the pool the wizard's "Sealed
 * client project" start would make on that project, unchanged; Customise…
 * opens that start in the wizard. A project already kept apart opens its
 * pool instead.
 *
 * This holds what the app's first window needs (which pool keeps a project
 * apart, and the words). The sheet and the pool it makes load on first use.
 */
import type { PoolInfo } from "./ipc/pools";
import { sealLine } from "./pool-sentences";
import { POOLS_GUIDE } from "./pool-words";

/** New pool's start that keeps a project apart (Customise… opens it). */
export const KEEP_APART_START = "sealed-client";

/** The guide's section on keeping a project apart. */
export const KEEP_APART_GUIDE = `${POOLS_GUIDE}#keep-a-project-apart`;

/** The sealed pool that keeps the project apart, if one does (its
 * sessions are in no other pool; D-62 decision 14). */
export function apartPool(pools: readonly PoolInfo[] | undefined, projectId: string): PoolInfo | null {
  return pools?.find((pool) => pool.sealed && pool.project_id === projectId) ?? null;
}

/** What a sealed pool also does to Claude Code (the wizard's switch says it). */
export const SEAL_CLAUDE = "Claude Code sessions AI Juggler starts here also leave Claude's own messages between sessions, from their next start.";
const SEAL_SWITCH = "A switch into or out of a sealed pool asks first.";
const SEAL_UNDO = "You can unseal it later from its diagram; its sessions go back into the pools they were in.";

/** "What does this mean?" under New pool's seal switch. */
export const SEAL_TIPS: readonly string[] = ["A sealed pool's sessions are in no other pool, and nothing leaves it through AI Juggler.", SEAL_SWITCH, SEAL_UNDO];

/** One of the three things keeping it apart does. */
export interface ApartFact {
  id: "only" | "closed" | "team";
  title: string;
  line: string;
}

/** The sheet's words for `project` (its name on screen). A serious choice
 * (the serious-choices rule): its consequence, an example, what it
 * doesn't do, "What does this mean?" and the guide. The tips open with the
 * word its pool's header will show ("sealed"). */
export function keepApartWords(project: string) {
  return {
    title: `Keep ${project} apart`,
    lead: `${project}'s sessions get a team of their own. Nothing from their work reaches your other pools through AI Juggler.`,
    facts: [
      { id: "only", title: "In this pool only", line: `Every session in ${project}, now and later, is in this pool and in no other.` },
      { id: "closed", title: "Nothing in or out", line: "No messages, notes or tasks pass between it and your other pools through AI Juggler." },
      { id: "team", title: "Still a team inside", line: "Its agents can still message each other and share notes and tasks." },
    ] satisfies ApartFact[],
    example: `Example: a session in ${project} can't send a note to your agents in other projects, and theirs can't send one in.`,
    notTitle: "What it doesn't do",
    not: `It doesn't tie ${project} to one account: sessions there can still start on any of your apps. It starts or stops nothing, and what each agent may change in your files stays with its own app's settings.`,
    tips: [sealLine({ sealed: true }), SEAL_SWITCH, SEAL_CLAUDE, SEAL_UNDO],
    /** What Keep apart makes, before it does. */
    makes: (pool: string, joins: string) => `It makes the pool «${pool}». ${joins} Change any of it first with Customise…, or later on its diagram.`,
    keep: "Keep apart",
    customise: "Customise…",
    made: `${project} is kept apart`,
    madeDetail: (pool: string) => `Its sessions are in «${pool}» and in no other pool.`,
    open: "Open it",
    failed: "Couldn't keep it apart",
  };
}

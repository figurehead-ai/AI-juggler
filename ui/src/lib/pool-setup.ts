/**
 * 0.4.6 (AJ-E3011 §1): a pool's setup checklist on its canvas. It keeps
 * the setup's order — where, who's in, who leads, everyone else, who may
 * message whom, what they may do on their own — with a tick on each step
 * done, and stays until every step is; it never vanishes when members are
 * added. Where, who's in and who leads tick from the pool itself; the
 * steps that start filled in tick once seen (the setup walked them, or
 * the user looked and said so), kept on this Mac per pool: a viewer's
 * convenience, never the pool's state.
 */
import { SETUP_STEPS, type SetupStep } from "./pool-words";
import { teamToolsLine, type TeamTools } from "./team-tools";
import type { DesktopReach } from "./connector";

const KEY = (poolId: string) => `aij.pool-setup.${poolId}`;

const STEPS: readonly string[] = [...SETUP_STEPS, "template", "review"];
const isStep = (value: unknown): value is SetupStep => typeof value === "string" && STEPS.includes(value);

/** The steps seen for `poolId` (none when storage can't be read). */
export function seenSteps(poolId: string): Set<SetupStep> {
  try {
    const raw = localStorage.getItem(KEY(poolId));
    const list: unknown = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(list) ? list.filter(isStep) : []);
  } catch {
    return new Set();
  }
}

/** Marks `steps` seen for `poolId`. */
export function markSeen(poolId: string, steps: readonly SetupStep[]): Set<SetupStep> {
  const seen = seenSteps(poolId);
  for (const step of steps) seen.add(step);
  try {
    localStorage.setItem(KEY(poolId), JSON.stringify([...seen]));
  } catch {
    // Private storage: the checklist just asks again next time.
  }
  window.dispatchEvent(new CustomEvent(SETUP_SEEN, { detail: poolId }));
  return seen;
}

/** A pool's seen steps changed here (its id in `detail`). */
export const SETUP_SEEN = "aij:pool-setup-seen";

export interface SetupFacts {
  /** A project's own pool (its scope is its project). */
  own: boolean;
  /** It takes every project. */
  every: boolean;
  /** It has who's in it (sessions, or a member that brings them). */
  whoIn: boolean;
  /** A session leads it. */
  led: boolean;
  /** It waits for an App's next session to lead. */
  leadNext: boolean;
}

export interface SetupRow {
  step: (typeof SETUP_STEPS)[number];
  done: boolean;
  /** Done by being left at its default (round 4 #2: "No lead for now",
   * everyone as Member, everyone may message, Work together are all
   * fine as they come). */
  byDefault: boolean;
}

/** Each step, in order, and whether it's done: set or seen here, or left
 * at its default. Only who's in has no default. */
export function setupRows(facts: SetupFacts, seen: ReadonlySet<SetupStep>): SetupRow[] {
  const set: Record<(typeof SETUP_STEPS)[number], boolean> = {
    where: facts.own || !facts.every || seen.has("where"),
    who: facts.whoIn,
    lead: facts.led || facts.leadNext || seen.has("lead"),
    everyone: seen.has("everyone"),
    talk: seen.has("talk"),
    own: seen.has("own"),
  };
  return SETUP_STEPS.map((step) => ({ step, done: set[step] || step !== "who", byDefault: !set[step] && step !== "who" }));
}

/** Round 4 #2: who's in and the team tools on make a pool ready; the rest
 * have defaults. */
export function setupReady(rows: readonly SetupRow[], toolsOn: boolean): boolean {
  return toolsOn && rows.every((row) => row.done);
}

/** Round 6 (Casey's run: "6 of 6 done" while nothing could work): what
 * keeps the pool from working, each in a plain line. The team tools that
 * can't run here (no one can message), and each desktop member whose app
 * has no connector it can use. While any is said, the setup is never
 * "done" or "ready". */
/** A sentence carried on after a colon: its first letter lowered, unless
 * it starts a name in capitals ("AI Juggler"). */
export const lowerFirst = (words: string): string => (/^[A-Z]{2}/.test(words) ? words : `${words.charAt(0).toLowerCase()}${words.slice(1)}`);

export function setupBlockers(tools: TeamTools | null, desktop: readonly { name: string; reach: DesktopReach }[]): string[] {
  const lower = lowerFirst;
  const out: string[] = [];
  if (tools && (tools.kind === "missing" || tools.kind === "unsafe")) out.push(`The team tools can't run, so no one here can message: ${lower(teamToolsLine(tools))}`);
  for (const one of desktop) if (one.reach.kind === "watched") out.push(`${one.name} can't talk here yet: ${lower(one.reach.line)}.`);
  return out;
}

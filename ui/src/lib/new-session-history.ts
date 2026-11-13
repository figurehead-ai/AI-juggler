/**
 * New session's guesses from history (`lib/new-session-choice`): the App
 * used last in a project, the ones its history names, the one used last
 * anywhere, and (D-103.7) the repo's own Account: the only Account that
 * ever ran in it. Pure, so the tests pin every case.
 */
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

/** What New session reads of a session to know which App was used last. */
export type UsedSession = Pick<InstanceInfo, "account_id" | "project_id" | "started_at_ms">;

/** The App used last in `project`: AI Juggler's newest session there. */
export function lastHere(apps: readonly LaunchRecipeInfo[], project: ProjectInfo | null, sessions: readonly UsedSession[]): LaunchRecipeInfo | undefined {
  if (!project) return undefined;
  return lastAnywhere(
    apps,
    sessions.filter((row) => row.project_id === project.id),
  );
}

/** An App the project's history says was used there. Walk K4 (AJ-E3306):
 * `accounts_used` is a set (the backend sorts it by name,
 * `projects/list.rs`), so it never says which was used last; its last
 * name was said as "the last you used here". The pick stays the one it
 * was (the last by name: steady, and an out-of-room one is still said and
 * another offered, D-31); only the claim is what the history holds. */
export function usedHere(apps: readonly LaunchRecipeInfo[], project: ProjectInfo | null): LaunchRecipeInfo | undefined {
  return [...(project?.accounts_used ?? [])]
    .reverse()
    .map((label) => apps.find((recipe) => recipe.account_label === label))
    .find((recipe) => recipe !== undefined);
}

/** The App of the session started last anywhere. */
export function lastAnywhere(apps: readonly LaunchRecipeInfo[], sessions: readonly UsedSession[]): LaunchRecipeInfo | undefined {
  const newest = [...sessions].filter((row) => apps.some((recipe) => recipe.account_id === row.account_id)).sort((a, b) => (b.started_at_ms ?? 0) - (a.started_at_ms ?? 0))[0];
  return newest ? apps.find((recipe) => recipe.account_id === newest.account_id) : undefined;
}

/** A name or session whose Account isn't known: one more Account, so
 * "the only one" is never claimed on a guess. */
const UNKNOWN = "?";

/** D-103.7: the Accounts (`accountOf`'s keys) whose sessions ran in
 * `project`, ever. `counted`: the backend's count by Account (every
 * session there, AI Juggler's and other apps') and the sessions in all,
 * when it has answered; a session on no known Account makes the sum fall
 * short, and counts as one more. Until then, what the app holds: its
 * sessions there and the Apps the project's history names (by launch
 * settings' name; one it can't place is one more). */
export function accountsRanHere({
  project,
  sessions,
  recipes,
  accountOf,
  counted,
}: {
  project: ProjectInfo | null;
  sessions: readonly UsedSession[];
  recipes: readonly LaunchRecipeInfo[];
  accountOf: (folderId: string) => string;
  counted?: { byAccount: ReadonlyMap<string, number>; total: number } | null;
}): string[] {
  if (!project) return [];
  if (counted) {
    const keys = [...counted.byAccount.keys()];
    const sum = [...counted.byAccount.values()].reduce((all, n) => all + n, 0);
    return sum < counted.total ? [...keys, UNKNOWN] : keys;
  }
  const keys = new Set<string>();
  for (const row of sessions) if (row.project_id === project.id) keys.add(row.account_id ? accountOf(row.account_id) : UNKNOWN);
  for (const label of project.accounts_used) {
    const recipe = recipes.find((one) => one.account_label === label);
    keys.add(recipe ? accountOf(recipe.account_id) : UNKNOWN);
  }
  return [...keys];
}

/** D-103.7: the App of the repo's own Account by its history, the only
 * Account that ever ran in `project` (`ranHere`): of its Apps, the one
 * used last here, else one its history names, else its first; one that
 * can start first. `undefined`: no such Account, or no App of it. */
export function ownApp(
  apps: readonly LaunchRecipeInfo[],
  { project, ranHere, sessions, accountOf, canStart }: { project: ProjectInfo | null; ranHere?: readonly string[] | null; sessions?: readonly UsedSession[]; accountOf: (folderId: string) => string; canStart: (folderId: string) => boolean },
): LaunchRecipeInfo | undefined {
  const own = project && ranHere?.length === 1 ? ranHere[0] : undefined;
  if (own === undefined || own === UNKNOWN) return undefined;
  const mine = apps.filter((recipe) => accountOf(recipe.account_id) === own);
  const pick = (from: readonly LaunchRecipeInfo[]) => lastHere(from, project, sessions ?? []) ?? usedHere(from, project) ?? from[0];
  return pick(mine.filter((recipe) => canStart(recipe.account_id))) ?? pick(mine);
}

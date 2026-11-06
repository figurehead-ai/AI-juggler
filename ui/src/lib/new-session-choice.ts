/**
 * D-62 (AJ-E3207) — New session's usual choice: the folder and the App it
 * opens on, each with a short why, and the pickers' top groups. Pure, so
 * the tests pin every case (principles 2–4 and 10, `docs/design/
 * PRINCIPLES.md`).
 *
 * The folder: the one it was opened for (a project's page, a drop, a
 * handover), else the project worked in last (here or in the tool's own
 * history), else the App's own folder from its launch settings, else none.
 *
 * The App (a CLI tool on a config folder; one per folder, its first
 * launch settings unless the user picks others):
 * 1. the one the user chose (in the sheet, or the door that opened it);
 * 2. this project's default; else the user's default; else the App used
 *    last in this folder;
 * 3. with no history here: the App used last anywhere; else the one with
 *    the most room (D-45: the user's own sheet may suggest by room); else
 *    the only one; else the first that can start.
 *
 * Out of room never moves the choice (D-31): the sheet says so and offers
 * the App with the most room on another Account, which the user may take.
 * Two Apps on one Account share its room, so the offer is never the same
 * Account.
 */
import { signedIn } from "./bridge";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { recentProjects, TOP } from "./pickers";
import { rankByRoom, roomKind, type RoomKind } from "./room";

export type FolderWhy = "asked" | "latest" | "app-folder";

export interface FolderChoice {
  root: string;
  project: ProjectInfo | null;
  why: FolderWhy;
}

export type AppWhy = "chosen" | "project-default" | "your-default" | "last-here" | "last-anywhere" | "most-room" | "only" | "first";

/** An Account's room, as Home's runway reads it. */
export interface Room {
  left: number | null;
  resetsAtMs: number | null;
  /** Its credits carry it on past a used-up window. */
  credits: boolean;
}

export interface AppChoice {
  /** The App's launch settings the session starts with. */
  recipe: LaunchRecipeInfo;
  why: AppWhy;
  /** Where its Account stands. */
  kind: RoomKind;
  /** When it's out of room, low or can't start: the App with the most
   * room on another Account, to offer (never taken by itself). */
  roomier: LaunchRecipeInfo | null;
}

export interface AppInputs {
  recipes: readonly LaunchRecipeInfo[];
  project: ProjectInfo | null;
  /** The config folders (`AccountInfo`): sign-in and the user's default. */
  folders: readonly AccountInfo[];
  /** The App (its folder id) the user chose, if any. */
  chosen?: string | null;
  /** Running and recent sessions: the App used last anywhere. */
  sessions?: readonly InstanceInfo[];
  /** The room of the Account folder `folderId` is signed in to. */
  roomOf: (folderId: string) => Room;
  /** The Account folder `folderId` belongs to (its own id when unknown). */
  accountOf?: (folderId: string) => string;
}

/** When a project was last worked in, here or in the tool's history. */
export function workedAt(project: ProjectInfo): number {
  return Math.max(project.last_activity_ms ?? 0, project.history_last_used_ms ?? 0);
}

/** The projects New session offers: neither moved nor forgotten. */
export function offeredProjects(projects: readonly ProjectInfo[]): ProjectInfo[] {
  return projects.filter((project) => !project.missing && !project.forgotten);
}

/** The folder it opens on (`null`: none known; the App's own folder is
 * the caller's, once the App is chosen). */
export function chooseFolder(projects: readonly ProjectInfo[], asked: string | null): FolderChoice | null {
  if (asked) return { root: asked, project: projects.find((project) => project.root === asked) ?? null, why: "asked" };
  const latest = recentProjects(offeredProjects(projects))[0];
  const project = projects.find((row) => row.id === latest);
  return project ? { root: project.root, project, why: "latest" } : null;
}

/** One launch settings per App (its first), in the backend's order. */
export function appRecipes(recipes: readonly LaunchRecipeInfo[]): LaunchRecipeInfo[] {
  const seen = new Set<string>();
  return recipes.filter((recipe) => !seen.has(recipe.account_id) && seen.add(recipe.account_id));
}

/** Whether folder `id` can start a session now (signed in; unknown: yes). */
function canStart(folders: readonly AccountInfo[], id: string): boolean {
  const state = folders.find((folder) => folder.id === id)?.login_state;
  return state === undefined || signedIn(state);
}

/** The Apps in the one order by room (Home's), each with its kind. */
export function appsByRoom(inputs: Pick<AppInputs, "recipes" | "folders" | "roomOf">): { recipe: LaunchRecipeInfo; kind: RoomKind; suggested: boolean }[] {
  const apps = appRecipes(inputs.recipes);
  return rankByRoom(
    apps,
    (recipe) => inputs.roomOf(recipe.account_id).left,
    (recipe) => canStart(inputs.folders, recipe.account_id),
    (recipe) => inputs.roomOf(recipe.account_id).credits,
  ).map((row) => ({ recipe: row.item, kind: row.kind, suggested: row.suggested }));
}

/** The App used last in `project` (its labels, newest last). */
function lastHere(apps: readonly LaunchRecipeInfo[], project: ProjectInfo | null): LaunchRecipeInfo | undefined {
  return [...(project?.accounts_used ?? [])]
    .reverse()
    .map((label) => apps.find((recipe) => recipe.account_label === label))
    .find((recipe) => recipe !== undefined);
}

/** The App of the session started last anywhere. */
function lastAnywhere(apps: readonly LaunchRecipeInfo[], sessions: readonly InstanceInfo[]): LaunchRecipeInfo | undefined {
  const newest = [...sessions].filter((row) => apps.some((recipe) => recipe.account_id === row.account_id)).sort((a, b) => (b.started_at_ms ?? 0) - (a.started_at_ms ?? 0))[0];
  return newest ? apps.find((recipe) => recipe.account_id === newest.account_id) : undefined;
}

/** The App it opens on, and why; `null` when there's none. */
export function chooseApp(inputs: AppInputs): AppChoice | null {
  const apps = appRecipes(inputs.recipes);
  if (apps.length === 0) return null;
  const ranked = appsByRoom(inputs);
  const of = (id: string | null | undefined) => (id ? apps.find((recipe) => recipe.account_id === id) : undefined);
  const startable = apps.filter((recipe) => canStart(inputs.folders, recipe.account_id));
  const candidates: Array<[LaunchRecipeInfo | undefined, AppWhy]> = [
    [of(inputs.chosen), "chosen"],
    [of(inputs.project?.default_account_id), "project-default"],
    [of(inputs.folders.find((folder) => folder.is_default)?.id), "your-default"],
    [lastHere(apps, inputs.project), "last-here"],
    [lastAnywhere(startable, inputs.sessions ?? []), "last-anywhere"],
    [ranked.find((row) => row.suggested)?.recipe, "most-room"],
    [startable.length === 1 ? startable[0] : undefined, "only"],
    [startable[0] ?? apps[0], "first"],
  ];
  const [recipe, why] = candidates.find(([found]) => found !== undefined) as [LaunchRecipeInfo, AppWhy];
  const room = inputs.roomOf(recipe.account_id);
  const kind = roomKind(room.left, canStart(inputs.folders, recipe.account_id), room.credits);
  const accountOf = inputs.accountOf ?? ((id: string) => id);
  const mine = accountOf(recipe.account_id);
  // Another Account's App with room; else, when the usual can't go on, one
  // whose room isn't measured, offered as such (walk N5: nothing was offered
  // while the others were unmeasured). Never switched to by itself (D-31).
  const other = (want: RoomKind) => ranked.find((row) => row.kind === want && accountOf(row.recipe.account_id) !== mine)?.recipe;
  const roomier = kind === "out" || kind === "low" || kind === "blocked" ? (other("room") ?? other("unknown") ?? null) : null;
  return { recipe, why, kind, roomier };
}

/** Why it's the App, in a few words after the sentence's dash. */
export function appWhyWords(why: AppWhy): string | null {
  switch (why) {
    case "project-default":
      return "this project's default";
    case "your-default":
      return "your default";
    case "last-here":
      return "the last you used here";
    case "last-anywhere":
      return "the last you used";
    case "most-room":
      return "the most room left";
    case "only":
      return "your only app";
    case "first":
      // Walk N4: why it's a guess (none of them has history to go by).
      return "a first guess: no app has run a session yet";
    case "chosen":
      return null;
  }
}

/** The folder picker's groups before All: worked in lately, then pinned. */
export function folderGroups(projects: readonly ProjectInfo[]): { label: string; values: string[] }[] {
  const offered = offeredProjects(projects);
  const recent = recentProjects(offered);
  const pinned = offered.filter((project) => project.pinned && !recent.includes(project.id)).map((project) => project.id);
  return [
    { label: "Recent", values: recent },
    { label: "Pinned", values: pinned },
  ];
}

/** The App picker's groups before All: Suggested (the choice, and the
 * most room when that's another), then the Apps used in this project. */
export function appGroups(choice: AppChoice | null, ranked: ReturnType<typeof appsByRoom>, project: ProjectInfo | null): { label: string; values: string[] }[] {
  const suggested = [choice?.recipe.account_id, ranked.find((row) => row.suggested)?.recipe.account_id].filter((id, at, all): id is string => !!id && all.indexOf(id) === at);
  const used = [...(project?.accounts_used ?? [])]
    .reverse()
    .map((label) => ranked.find((row) => row.recipe.account_label === label)?.recipe.account_id)
    .filter((id, at, all): id is string => !!id && !suggested.includes(id) && all.indexOf(id) === at)
    .slice(0, TOP);
  return [
    { label: "Suggested", values: suggested },
    { label: project ? `Used in ${project.name}` : "Used before", values: used },
  ];
}

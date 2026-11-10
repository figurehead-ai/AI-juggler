/**
 * D-62 (AJ-E3207) — New session's usual choice: the folder and the App it
 * opens on, each with a short why, and the pickers' top groups. Pure, so
 * the tests pin every case (principles 2–4 and 10, `docs/design/
 * PRINCIPLES.md`).
 *
 * The folder: the one it was opened for (a project's page, a drop, a
 * handover), else the project worked in last (here or in the tool's own
 * history), else, with no time on any, a project the user has (walk N4,
 * rc26: one without history once opened on a temporary folder), else the
 * App's own folder from its launch settings, else none.
 *
 * The App (a CLI tool on a config folder; one per folder, its first
 * launch settings unless the user picks others):
 * 1. the one the user chose (in the sheet, or the door that opened it);
 * 2. this project's default (the user's "Always use for <project>", in
 *    Change app or on the project page); else the App the user chose for
 *    the start just before ("Start another", D-96 F); else the user's
 *    default; else the App used last in this folder;
 * 3. with no history here: the App used last anywhere; else the one with
 *    the most room (D-45: the user's own sheet may suggest by room); else
 *    the only one; else the first that can start.
 *
 * An App the user chose (step 1, and step 2 up to the user's default)
 * stands even when it can't start: it's said and another offered. One
 * only guessed from history (used here, last anywhere) that can't start
 * (not signed in) is passed over while another can, and the why says so
 * ("— Claude Code · aij-test isn't signed in", the rc27 walk: the first
 * default was an App that couldn't start).
 *
 * Room never moves the user's own choice (D-31): an App from step 1 or
 * step 2 up to the user's default, out of room or low, stands; the sheet
 * says so and offers the App with the most room on another Account, which
 * the user may take. Two Apps on one Account share its room, so the offer
 * is never the same Account. A guess (used here, last anywhere) whose
 * Account is out, or low at `GIVE_WAY_LEFT` (10%) or less (`lib/room`),
 * gives way to the App with the most room on another Account, and the why
 * names the one passed over (D-97, amending D-31; D-96 E's words: "— Work
 * is almost out (5% left)"); with none that has room, it stays, warned.
 * In step 3 room picks, with nothing else to go by; when the App it passed
 * over (the first that can start) is out or low, the why says that too.
 */
import { signedIn } from "./bridge";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { recentProjects, TOP } from "./pickers";
import { GIVE_WAY_LEFT, outOfRoomWords, rankByRoom, roomKind, type RoomKind } from "./room";
import { whenWords } from "./time-words";

/** `used-before`: no time, but the tool's history has sessions there;
 * `yours` / `one-of-yours`: no history at all, the only project or one of
 * several (never said as work in it: adding a folder isn't). */
export type FolderWhy = "asked" | "latest" | "used-before" | "yours" | "one-of-yours" | "app-folder";

export interface FolderChoice {
  root: string;
  project: ProjectInfo | null;
  why: FolderWhy;
}

export type AppWhy = "chosen" | "project-default" | "kept" | "your-default" | "last-here" | "used-here" | "last-anywhere" | "most-room" | "only" | "first";

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
  /** Picked by room (`most-room`): the App it would have opened on
   * without room (a guess from history, D-97, or the first that can
   * start), when that one's Account is out or low. Its why says so;
   * `null` when nothing was skipped. */
  skipped: LaunchRecipeInfo | null;
  /** The App history pointed to that can't start (not signed in),
   * passed over for this one; its why says so. `null`: none. */
  passedOver?: LaunchRecipeInfo | null;
}

/** The steps that are the user's own choice: they stand even when the
 * App can't start (said, and another offered). */
const USERS_OWN = new Set<AppWhy>(["chosen", "project-default", "kept", "your-default"]);

/** What New session reads of a session to know which App was used last. */
export type UsedSession = Pick<InstanceInfo, "account_id" | "project_id" | "started_at_ms">;

export interface AppInputs {
  recipes: readonly LaunchRecipeInfo[];
  project: ProjectInfo | null;
  /** The config folders (`AccountInfo`): sign-in and the user's default. */
  folders: readonly AccountInfo[];
  /** The App (its folder id) the user chose, if any. */
  chosen?: string | null;
  /** The App the user chose for the start just before (Start another,
   * D-96 F): after this project's own default, before the user's. */
  kept?: string | null;
  /** Running and recent sessions: the App used last, here and anywhere
   * (only these fields are read). */
  sessions?: readonly UsedSession[];
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
  const offered = offeredProjects(projects);
  const latest = recentProjects(offered)[0];
  const project = projects.find((row) => row.id === latest);
  if (project) return { root: project.root, project, why: "latest" };
  return firstFolder(offered);
}

/** With no project worked in at any known time (walk N4, rc26: since the
 * backfill stopped stamping every found folder "now", 1e51970b0, a
 * newcomer's project has no time and New session opened on a temporary
 * folder): the project the tool's history used most, said as "used here
 * before"; else a project the user added (pinned first, then by name),
 * said as theirs. Nothing here claims a time. */
function firstFolder(offered: readonly ProjectInfo[]): FolderChoice | null {
  const used = offered.filter((row) => row.history_sessions > 0 || row.instance_count > 0 || row.accounts_used.length > 0);
  const byName = (a: ProjectInfo, b: ProjectInfo) => a.name.localeCompare(b.name);
  const best = [...used].sort((a, b) => b.history_sessions + b.instance_count - (a.history_sessions + a.instance_count) || byName(a, b))[0];
  if (best) return { root: best.root, project: best, why: "used-before" };
  const mine = [...offered].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned) || byName(a, b))[0];
  if (!mine) return null;
  return { root: mine.root, project: mine, why: offered.length === 1 ? "yours" : "one-of-yours" };
}

/** Why it's the folder, after its path, when no time says it. */
export function folderWhyWords(why: FolderWhy): string | null {
  switch (why) {
    case "used-before":
      return "used here before";
    case "yours":
      return "your project";
    case "one-of-yours":
      return "one of your projects";
    case "app-folder":
      return "its launch settings' folder";
    default:
      return null;
  }
}

/** One launch settings per App (its first), in the backend's order. */
export function appRecipes(recipes: readonly LaunchRecipeInfo[]): LaunchRecipeInfo[] {
  const seen = new Set<string>();
  return recipes.filter((recipe) => !seen.has(recipe.account_id) && seen.add(recipe.account_id));
}

/** Whether folder `id` can start a session now (signed in; unknown: yes):
 * New session's rule, and the switch's room line's (0.4.1). */
export function canStart(folders: readonly AccountInfo[], id: string): boolean {
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

/** The App used last in `project`: AI Juggler's newest session there. */
function lastHere(apps: readonly LaunchRecipeInfo[], project: ProjectInfo | null, sessions: readonly UsedSession[]): LaunchRecipeInfo | undefined {
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
function usedHere(apps: readonly LaunchRecipeInfo[], project: ProjectInfo | null): LaunchRecipeInfo | undefined {
  return [...(project?.accounts_used ?? [])]
    .reverse()
    .map((label) => apps.find((recipe) => recipe.account_label === label))
    .find((recipe) => recipe !== undefined);
}

/** The App of the session started last anywhere. */
function lastAnywhere(apps: readonly LaunchRecipeInfo[], sessions: readonly UsedSession[]): LaunchRecipeInfo | undefined {
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
    [of(inputs.kept), "kept"],
    [of(inputs.folders.find((folder) => folder.is_default)?.id), "your-default"],
    // Above: the user's own, kept when it can't start (said, another
    // offered). Below: guesses, passed over when they can't start while
    // another can (the rc27 walk).
    [lastHere(apps, inputs.project, inputs.sessions ?? []), "last-here"],
    [usedHere(apps, inputs.project), "used-here"],
    [lastAnywhere(startable, inputs.sessions ?? []), "last-anywhere"],
    [ranked.find((row) => row.suggested)?.recipe, "most-room"],
    [startable.length === 1 ? startable[0] : undefined, "only"],
    [startable[0] ?? apps[0], "first"],
  ];
  // The first that stands; a guess that can't start, while another can, is
  // passed over (the first such one is said).
  const stands = ([found, why]: [LaunchRecipeInfo | undefined, AppWhy]) => found !== undefined && (USERS_OWN.has(why) || startable.length === 0 || canStart(inputs.folders, found.account_id));
  const [recipe, why] = candidates.find(stands) as [LaunchRecipeInfo, AppWhy];
  const passedOver = candidates.slice(0, candidates.findIndex(stands)).find(([found]) => found !== undefined)?.[0] ?? null;
  const room = inputs.roomOf(recipe.account_id);
  const kind = roomKind(room.left, canStart(inputs.folders, recipe.account_id), room.credits);
  const accountOf = inputs.accountOf ?? ((id: string) => id);
  const mine = accountOf(recipe.account_id);
  // Another Account's App with room; else, when the usual can't go on, one
  // whose room isn't measured, offered as such (walk N5: nothing was offered
  // while the others were unmeasured). Never switched to by itself (D-31).
  const other = (want: RoomKind) => ranked.find((row) => row.kind === want && accountOf(row.recipe.account_id) !== mine)?.recipe;
  // D-97 (amends D-31): a guess on an Account out of room, or low at 10%
  // or less, gives way to the App with the most room on another; the why
  // names the one it passed over. The user's own choice stands, warned.
  const givesWay = kind === "out" || (kind === "low" && room.left !== null && room.left <= GIVE_WAY_LEFT);
  const roomy = !USERS_OWN.has(why) && why !== "most-room" && givesWay ? other("room") : undefined;
  if (roomy) return { recipe: roomy, why: "most-room", kind: "room", roomier: null, skipped: recipe, passedOver };
  const roomier = kind === "out" || kind === "low" || kind === "blocked" ? (other("room") ?? other("unknown") ?? null) : null;
  return { recipe, why, kind, roomier, skipped: why === "most-room" ? skippedFor(recipe, startable, inputs) : null, passedOver };
}

/** D-96 E: the App room passed over, when room picked: the one it would
 * have opened on without room (the only or first that can start), if
 * its Account is out or low. Another App with room that simply has less
 * isn't a skip worth saying. */
function skippedFor(recipe: LaunchRecipeInfo, startable: readonly LaunchRecipeInfo[], inputs: AppInputs): LaunchRecipeInfo | null {
  const usual = startable[0];
  if (!usual || usual.account_id === recipe.account_id) return null;
  const room = inputs.roomOf(usual.account_id);
  const kind = roomKind(room.left, true, room.credits);
  return kind === "out" || kind === "low" ? usual : null;
}

/** Why the App room passed over was passed over, after the sentence's
 * dash, in the Out and Low marks' words: "Work is almost out (5% left)",
 * "Work is out of room until 3:10 PM"; `about` when the figure is an
 * estimate, as the mark says it ("(about 5% left)"); `null` once it has
 * room again. */
export function skippedWords(name: string, room: Room, now: number = Date.now(), about = false): string | null {
  const kind = roomKind(room.left, true, room.credits);
  const est = about ? "about " : "";
  if (kind === "low") return `${name} is almost out (${est}${room.left}% left)`;
  if (kind === "out") {
    if (about && room.resetsAtMs !== null && room.resetsAtMs > now) return `${name} is out of room until about ${whenWords(room.resetsAtMs, now)}`;
    const words = outOfRoomWords(room.resetsAtMs, now);
    return `${name} is ${words.charAt(0).toLowerCase()}${words.slice(1)}`;
  }
  return null;
}

/** Why the App history pointed to was passed over, after the sentence's
 * dash: "Claude Code · aij-test isn't signed in". */
export function passedOverWords(name: string): string {
  return `${name} isn't signed in`;
}

/** The project a pool's page opens New session on (the existing order's
 * "the one it was opened for"): a project's own pool's, else its scope's
 * only project; `null` for a pool over several, none or every project. */
export function poolProject(pool: { project_id: string | null; scope: readonly (string | null)[]; scope_all: boolean } | null | undefined): string | null {
  if (!pool) return null;
  if (pool.project_id) return pool.project_id;
  if (pool.scope_all || pool.scope.length !== 1) return null;
  return pool.scope[0] ?? null;
}

/** Why it's the App, in a few words after the sentence's dash; `after`:
 * the folder of the start just before, for a kept App. */
export function appWhyWords(why: AppWhy, after?: string | null): string | null {
  switch (why) {
    case "kept":
      return after ? `as you chose for ${after}` : "as you chose last time";
    case "project-default":
      // The user said so ("Always use for web", or the project page).
      return "your default here";
    case "your-default":
      return "your default";
    case "last-here":
      return "the last you used here";
    case "used-here":
      return "used here before";
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

/** D-96 F: the project "Start another" offers after a start: the next
 * the user works in, by the order New session opens on (worked in last,
 * else used by the tool's history), never one started in this run
 * (`started`, their roots) and never a guess (a project without work in
 * it); `null` when none is left. */
export function nextFolder(projects: readonly ProjectInfo[], started: readonly string[]): FolderChoice | null {
  const left = projects.filter((project) => !started.includes(project.root));
  const next = chooseFolder(left, null);
  return next && (next.why === "latest" || next.why === "used-before") ? next : null;
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

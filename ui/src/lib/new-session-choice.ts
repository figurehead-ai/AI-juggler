/**
 * D-62 (AJ-E3207) — New session's usual choice: the App it opens on,
 * with a short why, and the App picker's top groups (the folder's are
 * `lib/new-session-folder`). Pure, so the tests pin every case
 * (principles 2–4 and 10, `docs/design/PRINCIPLES.md`).
 *
 * The App (a CLI tool on a config folder; one per folder, its first
 * launch settings unless the user picks others; only a folder that is a
 * CLI App, `lib/apps`' `cliRecipes`):
 * 1. the one the user chose (in the sheet, or the door that opened it);
 * 2. this project's default (the user's "Always use for <project>", in
 *    Change app, on the project page or offered in the sheet); else its
 *    own Account, the only one that ever ran there (D-103.7, R2: a
 *    client's repo opened on the agency's); else the App the user chose
 *    for the start just before ("Start another", D-96 F); else the user's
 *    default; else the App used last in this folder;
 * 3. with no history here: the App used last anywhere; else the one with
 *    the most room (D-45: the user's own sheet may suggest by room); else
 *    the only one; else the first that can start.
 *
 * An App the user chose (step 1, and step 2 up to the user's default)
 * and the repo's own Account's stand even when they can't start: it's
 * said and another offered (never another Account by itself). One
 * only guessed from history (used here, last anywhere) that can't start
 * (not signed in) is passed over while another can, and the why says so
 * ("— Claude Code · aij-test isn't signed in", the rc27 walk: the first
 * default was an App that couldn't start).
 *
 * Room never moves the user's own choice (D-31) or the repo's own
 * Account: an App from step 1 or step 2 up to the user's default, out of
 * room or low, stands; the sheet
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
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { TOP } from "./pickers";
import { lastAnywhere, lastHere, ownApp, usedHere, type UsedSession } from "./new-session-history";
import { GIVE_WAY_LEFT, outOfRoomWords, rankByRoom, roomKind, type RoomKind } from "./room";
import { whenWords } from "./time-words";

export type AppWhy = "chosen" | "project-default" | "own-account" | "kept" | "your-default" | "last-here" | "used-here" | "last-anywhere" | "most-room" | "only" | "first";

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

/** The steps that are the user's own choice, and the repo's own Account
 * (D-103.7): they stand even when the App can't start or has no room
 * (said, and another offered). */
const USERS_OWN = new Set<AppWhy>(["chosen", "project-default", "own-account", "kept", "your-default"]);

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
  /** The Accounts (`accountOf`'s keys) that ever ran in the project
   * (`accountsRanHere`); its own when there's one (D-103.7). */
  ranHere?: readonly string[] | null;
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

/** The App it opens on, and why; `null` when there's none. */
export function chooseApp(inputs: AppInputs): AppChoice | null {
  const apps = appRecipes(inputs.recipes);
  if (apps.length === 0) return null;
  const ranked = appsByRoom(inputs);
  const startable = apps.filter((recipe) => canStart(inputs.folders, recipe.account_id));
  const accountOf = inputs.accountOf ?? ((id: string) => id);
  // The 0.4.6 RC walk A8: a door or a binding may name a folder that isn't
  // a CLI App (a Claude app profile's, `cliRecipes`: the Account page's
  // first folder, a project bound to it before): its Account's own CLI App
  // stands for it, one that can start first, never another Account's.
  const of = (id: string | null | undefined) => {
    if (!id) return undefined;
    const own = apps.find((recipe) => recipe.account_id === id);
    if (own) return own;
    const mine = apps.filter((recipe) => accountOf(recipe.account_id) === accountOf(id));
    return mine.find((recipe) => canStart(inputs.folders, recipe.account_id)) ?? mine[0];
  };
  const own = ownApp(apps, { ...inputs, accountOf, canStart: (id) => canStart(inputs.folders, id) });
  const candidates: Array<[LaunchRecipeInfo | undefined, AppWhy]> = [
    [of(inputs.chosen), "chosen"],
    [of(inputs.project?.default_account_id), "project-default"],
    [own, "own-account"],
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

/** Why it's the App, in a few words after the sentence's dash; `after`:
 * the folder of the start just before, for a kept App. */
export function appWhyWords(why: AppWhy, after?: string | null): string | null {
  switch (why) {
    case "kept":
      return after ? `as you chose for ${after}` : "as you chose last time";
    case "project-default":
      // The user said so ("Always use for web", or the project page).
      return "your default here";
    case "own-account":
      return "the only account used here";
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

/** F-rc3f-1: New session chose it ("Suggested" beside it): not the
 * person's (picked, kept from the start before) nor their default. */
function isSuggestion(why: AppWhy): boolean {
  return !USERS_OWN.has(why) || why === "own-account";
}

/** SK-6: whether an App in this room can start a session that goes on
 * now: not out of room (its note: "a session there would stop at its
 * limit at once"), and signed in. */
function canRunNow(kind: RoomKind): boolean {
  return kind !== "out" && kind !== "blocked";
}

/** SK-6 (the 0.4.6 soak, N5): the App New session marks "Suggested" (its
 * folder id), never one that can't run now: New session's own choice
 * (`isSuggestion`) when it can; when it can't, the App it offers instead
 * (`roomier`) if that one has room; else none (the person's own pick
 * is theirs, not a suggestion). */
export function suggestedMarkOn(choice: AppChoice | null, ranked: ReturnType<typeof appsByRoom>): string | null {
  if (!choice) return null;
  if (canRunNow(choice.kind)) return isSuggestion(choice.why) ? choice.recipe.account_id : null;
  return roomierWithRoom(choice, ranked);
}

/** The App offered in place of one that can't run now, when it has room. */
function roomierWithRoom(choice: AppChoice, ranked: ReturnType<typeof appsByRoom>): string | null {
  const id = choice.roomier?.account_id;
  return id !== undefined && ranked.some((row) => row.recipe.account_id === id && row.kind === "room") ? id : null;
}

/** F-rc3f-1: the whys that are a folder's own: named, never taken, when
 * the person's pick is carried into a folder whose own is another. */
export const FOLDER_OWN: ReadonlySet<AppWhy> = new Set<AppWhy>(["project-default", "own-account", "last-here", "used-here"]);

/** The folder's own App, beside the one kept ("Claude Code CLI · Lab is
 * the only account used in team"); `null`: a why not the folder's. */
export function folderOwnWords(why: AppWhy, folder: string, app: string): string | null {
  switch (why) {
    case "project-default":
      return `${folder}'s default is ${app}`;
    case "own-account":
      return `${app} is the only account used in ${folder}`;
    case "last-here":
      return `${app} is the last you used in ${folder}`;
    case "used-here":
      return `${app} was used in ${folder} before`;
    default:
      return null;
  }
}

/** The App picker's groups before All: Suggested (the folder's own
 * suggestion, or the App offered for it when it can't run now, and the
 * most room when that's another), then the Apps used in this project. */
export function appGroups(choice: AppChoice | null, ranked: ReturnType<typeof appsByRoom>, project: ProjectInfo | null): { label: string; values: string[] }[] {
  // SK-6: one that can't run now isn't Suggested: the App offered for it is.
  const first = choice === null ? null : canRunNow(choice.kind) ? choice.recipe.account_id : roomierWithRoom(choice, ranked);
  const suggested = [first, ranked.find((row) => row.suggested)?.recipe.account_id].filter((id, at, all): id is string => !!id && all.indexOf(id) === at);
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

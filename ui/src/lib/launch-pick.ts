/**
 * Which account New session and New project start on, and why (AJ-E305
 * Req.6, AJ-E1501 Req.4, the 0.2 walk's F17: the pick wasn't the one
 * marked Suggested, and nothing said why). Pure, so tests pin it:
 *
 * 1. the one you just chose in the sheet;
 * 2. what you picked yourself: this project's default, else your default
 *    (AJ-E1501), else the account used last in this folder;
 * 3. else the Suggested one: the most room, by the one order by room
 *    (`lib/room`, as Home's "Best for new work");
 * 4. else the first (the backend's order: most recent).
 *
 * A pick of yours out of room is passed over for the Suggested one, and
 * the sheet says so ("Your default, Work, is out of room until …"): a
 * new session there would stop at once.
 */
import { runwayOf } from "../components/AccountRunway";
import { signedIn, type AccountInfo, type InstanceInfo, type LaunchRecipeInfo, type ProjectInfo, type QuotaDisplayData } from "./bridge";
import { rankByRoom, type RoomKind } from "./room";

export type PickWhy = "chosen" | "project-default" | "your-default" | "last-here" | "suggested" | "first";

export interface LaunchPick {
  recipe: LaunchRecipeInfo;
  why: PickWhy;
  /** The Suggested account's first launch setting, when one has room. */
  suggested: LaunchRecipeInfo | null;
  /** A pick of yours passed over because it's out of room. */
  passedOver: { recipe: LaunchRecipeInfo; why: PickWhy } | null;
}

/** An account's room, as Home's Runway reads it. */
export function launchRoom(display: QuotaDisplayData | undefined, sessions: InstanceInfo[] = []): { left: number | null; resetsAtMs: number | null } {
  const run = runwayOf(display, sessions);
  return { left: run.left, resetsAtMs: run.resetsAtMs ?? null };
}

export function pickLaunch(
  recipes: LaunchRecipeInfo[],
  project: ProjectInfo | null,
  quotas: QuotaDisplayData[],
  accounts: AccountInfo[] = [],
  chosenAccount: string | null = null,
  sessions: InstanceInfo[] = [],
): LaunchPick | null {
  if (recipes.length === 0) return null;
  const left = (accountId: string) =>
    launchRoom(
      quotas.find((entry) => entry.account === accountId),
      sessions.filter((row) => row.account_id === accountId),
    ).left;
  // Signed out can't take work: never suggested (as New session's rows).
  const usable = (recipe: LaunchRecipeInfo) => {
    const state = accounts.find((account) => account.id === recipe.account_id)?.login_state;
    return state === undefined || signedIn(state);
  };
  const ranked = rankByRoom(recipes, (recipe) => left(recipe.account_id), usable);
  const kindOf = new Map<string, RoomKind>(ranked.map((row) => [row.item.id, row.kind]));
  const suggested = ranked.find((row) => row.suggested)?.item ?? null;
  const of = (accountId: string | null | undefined) => (accountId ? recipes.find((recipe) => recipe.account_id === accountId) : undefined);

  const chosen = of(chosenAccount);
  if (chosen) return { recipe: chosen, why: "chosen", suggested, passedOver: null };

  const lastHere = [...(project?.accounts_used ?? [])]
    .reverse()
    .map((label) => recipes.find((recipe) => recipe.account_label === label))
    .find((recipe) => recipe !== undefined);
  const yours: Array<[LaunchRecipeInfo | undefined, PickWhy]> = [
    [of(project?.default_account_id), "project-default"],
    [of(accounts.find((account) => account.is_default)?.id), "your-default"],
    [lastHere, "last-here"],
  ];
  const mine = yours.find(([recipe]) => recipe !== undefined);
  if (mine) {
    const [recipe, why] = mine as [LaunchRecipeInfo, PickWhy];
    if (kindOf.get(recipe.id) !== "out" || !suggested) return { recipe, why, suggested, passedOver: null };
    return { recipe: suggested, why: "suggested", suggested, passedOver: { recipe, why } };
  }
  if (suggested) return { recipe: suggested, why: "suggested", suggested, passedOver: null };
  return { recipe: ranked[0]!.item, why: "first", suggested: null, passedOver: null };
}

/** Why it's the pick, in a few words beside it. */
export function pickWhyWords(why: PickWhy): string | null {
  switch (why) {
    case "project-default":
      return "this project's default";
    case "your-default":
      return "your default";
    case "last-here":
      return "used last here";
    case "suggested":
      return "Suggested: the most room";
    case "chosen":
    case "first":
      return null;
  }
}

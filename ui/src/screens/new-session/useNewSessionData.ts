/**
 * AJ-E3207 — what New session reads, from what's kept first: the launch
 * settings the app already holds (read again behind them), every
 * Account's runway and the Accounts as Home last read them (lib/kept-
 * reads, lib/subscriptions). The choice shows in the first frame; the
 * fresh reads only add to it. Room is the Account's: two Apps on one
 * plan share it (`accountDisplay`).
 */
import { useEffect, useMemo, useState } from "react";
import { launchRecipesList, rankingRows, type AccountInfo, type InstanceInfo, type LaunchRecipeInfo, type QuotaDisplayData, type RankedRowData } from "../../lib/bridge";
import { useQuotaDisplays } from "../../lib/kept-reads";
import { accountDisplay, accountName, accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import { useApps } from "../../lib/apps";
import { cliAppId, cliAppName } from "../../lib/app-name";
import { launchRoom } from "../../lib/launch-pick";
import { machineAdvice, type MachineAdviceInfo } from "../../lib/machine";
import type { Room } from "../../lib/new-session-choice";

export interface NewSessionData {
  /** `null` until known (never while the app holds some). */
  recipes: LaunchRecipeInfo[] | null;
  /** Whether the runways have answered. */
  roomKnown: boolean;
  roomOf: (folderId: string) => Room;
  /** The Account's runway, for its words. */
  displayOf: (folderId: string) => QuotaDisplayData | undefined;
  /** The sessions on the Account (its runway's newest reading). */
  sessionsOf: (folderId: string) => InstanceInfo[];
  /** The Account it belongs to, as one key (its folder's id when unknown). */
  accountKeyOf: (folderId: string) => string;
  /** The Account's name, when it's signed in and known. */
  accountNameOf: (folderId: string) => string | null;
  /** The App's name ("Claude Code · Work Max", or the user's). */
  appNameOf: (recipe: LaunchRecipeInfo) => string;
  /** The backend's admission verdict for the folder, when it gave one. */
  verdictOf: (folderId: string) => "ok" | "tight" | "no" | undefined;
  /** This Mac's room for a launch settings' session. */
  adviceOf: (recipeId: string) => MachineAdviceInfo | undefined;
}

export function useNewSessionData({
  folders,
  sessions,
  kept,
  projectId,
}: {
  folders: readonly AccountInfo[];
  sessions: readonly InstanceInfo[];
  /** The launch settings the app already holds. */
  kept?: readonly LaunchRecipeInfo[];
  projectId: string | undefined;
}): NewSessionData {
  const [recipes, setRecipes] = useState<LaunchRecipeInfo[] | null>(() => (kept && kept.length > 0 ? [...kept] : null));
  useEffect(() => {
    let alive = true;
    void launchRecipesList().then(
      (rows) => alive && setRecipes(rows),
      () => alive && setRecipes((was) => was ?? []),
    );
    return () => {
      alive = false;
    };
  }, []);
  const { displays, known } = useQuotaDisplays();
  const subscriptions = useSubscriptions();
  const apps = useApps(folders);

  const [ranked, setRanked] = useState<RankedRowData[]>([]);
  useEffect(() => {
    let alive = true;
    void rankingRows(projectId).then(
      (rows) => alive && setRanked(rows),
      () => null,
    );
    return () => {
      alive = false;
    };
  }, [projectId]);

  const [advice, setAdvice] = useState<Map<string, MachineAdviceInfo>>(new Map());
  useEffect(() => {
    let alive = true;
    void machineAdvice().then(
      (rows) => alive && setAdvice(new Map(rows.map((row) => [row.recipe_id, row]))),
      () => null,
    );
    return () => {
      alive = false;
    };
  }, []);

  return useMemo(() => {
    const accountOf = (folderId: string) => (subscriptions ? accountOfProfile(subscriptions, folderId) : undefined);
    const displayOf = (folderId: string) => {
      const account = accountOf(folderId);
      return (account ? accountDisplay(account, displays) : undefined) ?? displays.find((display) => display.account === folderId);
    };
    const sessionsOf = (folderId: string) => {
      const members = new Set(accountOf(folderId)?.profiles.map((profile) => profile.id) ?? [folderId]);
      return sessions.filter((row) => row.account_id !== null && members.has(row.account_id));
    };
    return {
      recipes,
      roomKnown: known,
      roomOf: (folderId: string): Room => {
        const room = launchRoom(displayOf(folderId), sessionsOf(folderId));
        return { left: room.left, resetsAtMs: room.resetsAtMs, credits: room.credits !== null };
      },
      displayOf,
      sessionsOf,
      accountKeyOf: (folderId: string) => accountOf(folderId)?.key ?? folderId,
      accountNameOf: (folderId: string) => {
        const account = accountOf(folderId);
        return account && account.state === "signed_in" ? accountName(account, subscriptions ?? undefined) : null;
      },
      appNameOf: (recipe: LaunchRecipeInfo) => apps?.find((app) => app.id === cliAppId(recipe.account_id))?.name ?? cliAppName(recipe.provider, recipe.account_label),
      verdictOf: (folderId: string) => ranked.find((row) => row.account === folderId)?.admission ?? undefined,
      adviceOf: (recipeId: string) => advice.get(recipeId),
    };
  }, [recipes, known, displays, subscriptions, apps, sessions, ranked, advice]);
}

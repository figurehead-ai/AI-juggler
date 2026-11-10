/**
 * The Accounts screen's data (split from `Profiles.tsx`, the size budget).
 * 0.3.4, page switching: what the app already holds paints the first
 * frame; a skeleton is for a screen with nothing known yet.
 */
import { useCallback, useEffect, useState } from "react";
import { cachedSubscriptions, loadSubscriptions, useRecheckSignIns, type SubscriptionInfo } from "../../lib/subscriptions";
import { inPlaceProviders, limitProviders, type InPlaceSupport } from "../../lib/providers";
import { accountsList } from "../../lib/ipc/accounts";
import { errorText } from "../../lib/bridge";
import { launchRecipesList } from "../../lib/ipc/launch";
import { providersCapabilities } from "../../lib/ipc/providers";
import { useResource } from "../../lib/use-resource";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";

/** ADR-0018: which tools' copies can switch to their original — the
 * backend's static capabilities (no install probe). If the call fails the
 * offer stays; the backend refuses what it can't adopt. 0.0.11 walk (Q3):
 * the tools that don't report their own limits. */
export function useToolCapabilities(): { inPlace: InPlaceSupport | null; limits: InPlaceSupport } {
  const { data, error } = useResource("providers-capabilities", providersCapabilities);
  if (data) return { inPlace: inPlaceProviders(data), limits: limitProviders(data) };
  return { inPlace: error !== undefined ? "unknown" : null, limits: "unknown" };
}

/** The accounts, their profiles and the subscriptions they make up. */
export function useAccountsData({
  knownAccounts,
  knownProfiles,
  onError,
}: {
  /** The app's own lists, kept current by it (`null` before its first read). */
  knownAccounts: AccountInfo[] | null;
  knownProfiles: LaunchRecipeInfo[] | null;
  onError: (words: string) => void;
}): {
  accounts: AccountInfo[] | null;
  profiles: LaunchRecipeInfo[] | null;
  subscriptions: SubscriptionInfo[] | null;
  /** After a change made here: everything read again, now. */
  refresh: () => Promise<void>;
} {
  // What's already known paints the first frame (the page-switching
  // report: a skeleton on every visit); the placeholder is for a screen
  // with nothing known yet.
  const [accounts, setAccounts] = useState<AccountInfo[] | null>(knownAccounts);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(knownProfiles);
  const [subscriptions, setSubscriptions] = useState<SubscriptionInfo[] | null>(cachedSubscriptions);
  // After a change made here: everything read again, now.
  const refresh = useCallback(async () => {
    try {
      const [accountRows, profileRows, groups] = await Promise.all([accountsList(), launchRecipesList(), loadSubscriptions(true)]);
      setAccounts(accountRows);
      setProfiles(profileRows);
      setSubscriptions(groups);
    } catch (cause) {
      onError(errorText(cause));
    }
    // `onError` is the screen's state setter, stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The app's lists as they change (it reads them on every sessions
  // update and after every account change).
  useEffect(() => {
    if (knownAccounts) setAccounts(knownAccounts);
  }, [knownAccounts]);
  useEffect(() => {
    if (knownProfiles) setProfiles(knownProfiles);
  }, [knownProfiles]);

  // On opening: what the app doesn't hold is read; the accounts' sign-ins
  // behind what's shown, again only when older than half a minute, marked
  // old (a sign-in started, an account added), or when the profiles they
  // cover aren't the app's any more.
  const knownIds = knownAccounts ? knownAccounts.map((account) => account.id).sort().join(",") : null;
  useEffect(() => {
    let alive = true;
    const kept = cachedSubscriptions();
    const covered = kept ? kept.flatMap((sub) => sub.profiles.map((profile) => profile.id)).sort().join(",") : null;
    const force = knownIds !== null && covered !== null && covered !== knownIds;
    void Promise.all([knownIds === null ? accountsList() : null, knownProfiles === null ? launchRecipesList() : null, loadSubscriptions(force)]).then(
      ([accountRows, profileRows, groups]) => {
        if (!alive) return;
        if (accountRows) setAccounts(accountRows);
        if (profileRows) setProfiles(profileRows);
        setSubscriptions(groups);
      },
      (cause: unknown) => alive && onError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
    // Once, on opening: the lists' own changes come through the effects above.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // Walk A10: back in the window (a sign-in changed in a terminal), the
  // sign-ins read again when older than half a minute, as on opening.
  useEffect(() => {
    const back = () => {
      if (document.visibilityState === "hidden") return;
      void loadSubscriptions(false).then(setSubscriptions, () => undefined);
    };
    window.addEventListener("focus", back);
    document.addEventListener("visibilitychange", back);
    return () => {
      window.removeEventListener("focus", back);
      document.removeEventListener("visibilitychange", back);
    };
  }, []);
  // 0.4.1: a sign-in that couldn't be checked is read again until it is.
  useRecheckSignIns(subscriptions, setSubscriptions);
  return { accounts, profiles, subscriptions, refresh };
}

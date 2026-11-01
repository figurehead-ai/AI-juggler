/**
 * The Accounts screen's data (split from `Profiles.tsx`, the size budget).
 * 0.3.4, page switching: what the app already holds paints the first
 * frame; a skeleton is for a screen with nothing known yet.
 */
import { useCallback, useEffect, useState } from "react";
import { cachedSubscriptions, loadSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";
import { inPlaceProviders, limitProviders, type InPlaceSupport } from "../../lib/providers";
import { accountsList, errorText, launchRecipesList, providersCapabilities, type AccountInfo, type LaunchRecipeInfo } from "../../lib/bridge";

/** ADR-0018: which tools' copies can switch to their original — the
 * backend's static capabilities (no install probe). If the call fails the
 * offer stays; the backend refuses what it can't adopt. 0.0.11 walk (Q3):
 * the tools that don't report their own limits. */
export function useToolCapabilities(): { inPlace: InPlaceSupport | null; limits: InPlaceSupport } {
  const [inPlace, setInPlace] = useState<InPlaceSupport | null>(null);
  const [limits, setLimits] = useState<InPlaceSupport>("unknown");
  useEffect(() => {
    let alive = true;
    void providersCapabilities().then(
      (rows) => {
        if (!alive) return;
        setInPlace(inPlaceProviders(rows));
        setLimits(limitProviders(rows));
      },
      () => alive && setInPlace("unknown"),
    );
    return () => {
      alive = false;
    };
  }, []);
  return { inPlace, limits };
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
  return { accounts, profiles, subscriptions, refresh };
}

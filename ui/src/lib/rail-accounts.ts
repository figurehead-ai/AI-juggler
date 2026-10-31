/**
 * D-62 — the Account (subscription) each row's App is signed in to, for
 * the rail's Account lens and its `account:` filter. Read only while one
 * of them needs it (instant and lazy: the rail never waits for it).
 */
import { useMemo } from "react";
import { accountName, useSubscriptions } from "./subscriptions";
import type { AccountOf } from "./rail-places";

export function useRailAccountOf(wanted: boolean): AccountOf {
  const subscriptions = useSubscriptions(0, wanted);
  return useMemo(() => {
    const byProfile = new Map((subscriptions ?? []).flatMap((row) => row.profiles.map((profile) => [profile.id, { key: row.key, label: accountName(row) }] as const)));
    return (profileId: string | null) => (profileId === null ? null : (byProfile.get(profileId) ?? null));
  }, [subscriptions]);
}

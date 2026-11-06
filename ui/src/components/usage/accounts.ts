/**
 * 0.1.5 (D-35) — usage is recorded per profile; an account is the
 * subscription its profiles share. These fold profile series and shares
 * into accounts, each wearing the Account's one colour (AJ-E3206, stored
 * once), so an account is one colour on Usage, its card and its page. A
 * folder no Account lists stands for itself: its CLI App's colour.
 */
import { accountColor, appColor } from "../../lib/marks";
import { cliAppId } from "../../lib/app-name";
import { accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import type { Series } from "../charts/scale";

export type AccountGroup = { key: string; label: string; color: string; profiles: string[] };

/** The account a profile is on (signed-out or unknown profiles stand alone). */
export function accountGroups(subscriptions: SubscriptionInfo[] | null, labels: Map<string, string>): Map<string, AccountGroup> {
  const out = new Map<string, AccountGroup>();
  for (const sub of subscriptions ?? []) {
    if (sub.state === "signed_out") continue;
    const profiles = sub.profiles.map((profile) => profile.id);
    const group = { key: sub.key, label: accountName(sub), color: accountColor(sub), profiles };
    for (const id of profiles) out.set(id, group);
  }
  for (const [id, label] of labels) {
    if (!out.has(id)) out.set(id, { key: `profile:${id}`, label, color: appColor(cliAppId(id)), profiles: [id] });
  }
  return out;
}

/** A Runway card's title: its Account's name; a folder that isn't signed
 * in has none to name, so it says so (AJ-E3202 req. 3, walk A10), its App
 * named under it. */
export function runwayTitle(group: AccountGroup, subscriptions: SubscriptionInfo[] | null): string {
  const folder = group.profiles[0];
  const signedOut = group.key.startsWith("profile:") && (subscriptions ?? []).some((entry) => entry.state === "signed_out" && entry.profiles.some((profile) => profile.id === folder));
  return signedOut ? "Not signed in" : group.label;
}

/** Profile series summed into their accounts, biggest first. */
export function foldSeries(series: Series[], groups: Map<string, AccountGroup>): Series[] {
  const out = new Map<string, Series>();
  for (const entry of series) {
    const group = groups.get(entry.key);
    const key = group?.key ?? entry.key;
    const into = out.get(key) ?? { key, label: group?.label ?? entry.label, color: group?.color ?? entry.color, values: entry.values.map(() => 0) };
    entry.values.forEach((value, i) => (into.values[i]! += value));
    out.set(key, into);
  }
  const total = (entry: Series) => entry.values.reduce((sum, value) => sum + value, 0);
  return [...out.values()].sort((a, b) => total(b) - total(a));
}

/** Profile shares summed into their accounts, biggest first. */
export function foldItems<T extends { key: string; label: string; value: number; color: string }>(items: T[], groups: Map<string, AccountGroup>): T[] {
  const out = new Map<string, T>();
  for (const item of items) {
    const group = groups.get(item.key);
    const key = group?.key ?? item.key;
    const into = out.get(key) ?? { ...item, key, label: group?.label ?? item.label, color: group?.color ?? item.color, value: 0 };
    into.value += item.value;
    out.set(key, into);
  }
  return [...out.values()].sort((a, b) => b.value - a.value);
}

/**
 * 0.1.5 (D-35) — usage is recorded per profile; an account is the
 * subscription its profiles share. These fold profile series and shares
 * into accounts, each wearing its first profile's colour, so an account
 * is one colour on Usage, its card and its page.
 */
import { accountColor } from "../../lib/palette";
import { accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import type { Series } from "../charts/scale";

export type AccountGroup = { key: string; label: string; color: string; profiles: string[] };

/** The account a profile is on (signed-out or unknown profiles stand alone). */
export function accountGroups(subscriptions: SubscriptionInfo[] | null, labels: Map<string, string>): Map<string, AccountGroup> {
  const out = new Map<string, AccountGroup>();
  for (const sub of subscriptions ?? []) {
    if (sub.state === "signed_out") continue;
    const profiles = sub.profiles.map((profile) => profile.id);
    const group = { key: sub.key, label: accountName(sub), color: accountKeyColor(sub), profiles };
    for (const id of profiles) out.set(id, group);
  }
  for (const [id, label] of labels) {
    if (!out.has(id)) out.set(id, { key: `profile:${id}`, label, color: accountColor(id), profiles: [id] });
  }
  return out;
}

/** An account's colour: its first profile's swatch. */
export function accountKeyColor(sub: SubscriptionInfo): string {
  return accountColor(sub.profiles[0]?.id ?? sub.key);
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

/**
 * D-103.9 (persona R3, P7): who did the work on a project when no tokens
 * are measured there: its sessions per Account ("Client B · 14
 * sessions"), never "0 tokens" or "No usage yet" beside 14 sessions.
 * Counted by the backend's relation cache (every session there, AI
 * Juggler's and other apps', as its Sessions list holds them); until it
 * answers, and in the design preview, from the sessions the app holds.
 * Sessions on no known Account are one more part, said as such.
 */
import { useMemo } from "react";
import { hasLiveBackend } from "../../lib/bridge";
import { useRelatedCounts, useRelationCounts } from "../../lib/relation-counts";
import { accountName, accountOfProfile, useSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";
import { accountColor } from "../../lib/marks";
import type { DonutItem } from "../../components/charts/Donut";

/** The part for sessions whose Account isn't known. */
const UNKNOWN = "__unknown";
const UNKNOWN_WORDS = "Account unknown";

export interface SessionsByAccount {
  /** One part per Account, most sessions first; the unknown last. */
  items: DonutItem[];
  /** The sessions in all. */
  total: number;
}

function part(row: SubscriptionInfo | undefined, key: string, value: number): DonutItem {
  return row ? { key, label: accountName(row), value, color: accountColor(row) } : { key: UNKNOWN, label: UNKNOWN_WORDS, value, color: "var(--color-faint)" };
}

/** The parts, from the backend's count (`counted`: stored Account id →
 * sessions, and the sessions in all), else from `local` (each session's
 * folder). */
export function sessionsByAccount({
  counted,
  local,
  subscriptions,
}: {
  counted: { byAccount: ReadonlyMap<string, number>; total: number } | null;
  local: readonly { accountId: string | null }[];
  subscriptions: readonly SubscriptionInfo[];
}): SessionsByAccount {
  const parts = new Map<string, DonutItem>();
  const add = (row: SubscriptionInfo | undefined, key: string, n: number) => {
    const next = part(row, row ? key : UNKNOWN, n);
    const was = parts.get(next.key);
    parts.set(next.key, was ? { ...was, value: was.value + n } : next);
  };
  let total = 0;
  if (counted) {
    for (const [id, n] of counted.byAccount) add(subscriptions.find((row) => row.account_id === id), id, n);
    total = counted.total;
    const known = [...counted.byAccount.values()].reduce((sum, n) => sum + n, 0);
    if (known < total) add(undefined, UNKNOWN, total - known);
  } else {
    for (const one of local) {
      const row = one.accountId ? accountOfProfile(subscriptions, one.accountId) : undefined;
      add(row?.state === "signed_in" ? row : undefined, row?.account_id ?? row?.key ?? UNKNOWN, 1);
    }
    total = local.length;
  }
  const items = [...parts.values()].sort((a, b) => Number(a.key === UNKNOWN) - Number(b.key === UNKNOWN) || b.value - a.value || a.label.localeCompare(b.label));
  return { items, total };
}

/** `projectId`'s sessions per Account, kept fresh as its page's counts
 * are; `local`: the sessions the app holds there, meanwhile. */
export function useSessionsByAccount(projectId: string, local: readonly { accountId: string | null }[]): SessionsByAccount {
  const live = hasLiveBackend();
  const byAccount = useRelatedCounts(live ? "project" : null, projectId, "subscription");
  const total = useRelationCounts(live ? "project" : null, projectId)?.sessions.all;
  const subscriptions = useSubscriptions();
  return useMemo(
    () => sessionsByAccount({ counted: byAccount && total !== undefined ? { byAccount, total } : null, local, subscriptions: subscriptions ?? [] }),
    [byAccount, total, local, subscriptions],
  );
}

/** "Client B · 14 sessions", the parts in words (the donut's twin for
 * a line). */
export function sessionsWords(items: readonly DonutItem[]): string {
  return items.map((item) => `${item.label} · ${item.value === 1 ? "1 session" : `${item.value} sessions`}`).join(", ");
}

/**
 * AJ-E2104/E2105 with AJ-E2110's Settings › Desktop apps: the sessions
 * the rail and Home show are the ones in apps the user didn't turn off.
 * Nothing hidden is silent: each app turned off with something open in it
 * leaves its count ("2 in Cursor hidden · Settings").
 */
import { useMemo } from "react";
import { useConversations } from "./conversations";
import { COMMON_APPS, useHiddenApps } from "./desktop-apps";
import type { ConversationInfo } from "./ipc/conversations";

export interface HiddenApp {
  app: string;
  count: number;
}

/** Splits conversations by the apps turned off: one is hidden when the
 * app it is open in (its first place) is off. */
export function splitHidden(all: ConversationInfo[], hidden: ReadonlySet<string>): { shown: ConversationInfo[]; hidden: HiddenApp[] } {
  if (hidden.size === 0) return { shown: all, hidden: [] };
  const counts = new Map<string, number>();
  const shown = all.filter((c) => {
    const place = (c.places ?? [])[0];
    if (!place?.app_id || !hidden.has(place.app_id)) return true;
    if (c.live) counts.set(place.app || place.app_id, (counts.get(place.app || place.app_id) ?? 0) + 1);
    return false;
  });
  return { shown, hidden: [...counts.entries()].map(([app, count]) => ({ app, count })).sort((a, b) => a.app.localeCompare(b.app)) };
}

/** The conversations to show, and what the apps turned off hold. */
export function useShownConversations(): { conversations: ConversationInfo[]; hidden: HiddenApp[] } {
  const { all } = useConversations();
  const ids = useMemo(() => {
    const found = new Set<string>(COMMON_APPS.map((app) => app.id));
    for (const c of all) for (const place of c.places ?? []) if (place.app_id) found.add(place.app_id);
    return [...found].sort();
  }, [all]);
  const off = useHiddenApps(ids);
  return useMemo(() => {
    const { shown, hidden } = splitHidden(all, off);
    return { conversations: shown, hidden };
  }, [all, off]);
}

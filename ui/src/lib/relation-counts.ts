/**
 * AJ-E3102 — a page's tab counts from the backend's relation-count cache:
 * the same rows its lists page through, so a count agrees with the list
 * it heads, however many sessions there are. The page shows at once; its
 * counts follow a moment later (the first read after a start, or after
 * Clear cache, builds them). Each answer is kept per entity and read
 * again when sessions, the tools' conversations or the apps' places
 * change. `undefined` until read, and always in the design preview.
 */
import { useEffect } from "react";
import { hasLiveBackend, onSessionsUpdated } from "./bridge";
import { onConversationsChanged } from "./ipc/conversations";
import { onPlacesChanged } from "./ipc/places";
import { relationsCounts, type EntityKind, type RelationCountsInfo } from "./ipc/relations";
import { sharedReads, useSharedRead } from "./shared-read";

const KEY = ":";

const countsOf = sharedReads<RelationCountsInfo | undefined>(async (key) => {
  const at = key.indexOf(KEY);
  const [answer] = await relationsCounts([{ kind: key.slice(0, at) as EntityKind, id: key.slice(at + KEY.length) }]);
  return answer;
});

/** A change can come in bursts (a pass reading many files): one read
 * after it settles. */
const SETTLE_MS = 250;

/** `kind` `id`'s counts (`null` kind: none asked). */
export function useRelationCounts(kind: EntityKind | null, id: string | null | undefined): RelationCountsInfo | undefined {
  const live = hasLiveBackend();
  const key = live && kind && id != null ? `${kind}${KEY}${id}` : null;
  const source = key ? countsOf(key) : null;
  useEffect(() => {
    if (!source) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const again = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void source.refresh(), SETTLE_MS);
    };
    const stops = [onSessionsUpdated(again), onConversationsChanged(again), onPlacesChanged(again)];
    return () => {
      if (timer) clearTimeout(timer);
      stops.forEach((stop) => stop());
    };
  }, [source]);
  return useSharedRead(source) ?? undefined;
}

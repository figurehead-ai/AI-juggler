/**
 * AJ-E3102 — a page's tab counts from the backend's relation-count cache:
 * the same rows its lists page through, so a count agrees with the list
 * it heads, however many sessions there are. The page shows at once; its
 * counts follow a moment later (the first read after a start, or after
 * Clear cache, builds them). Each answer is kept per entity and read
 * again when sessions, the tools' conversations or the apps' places
 * change. `undefined` until read, and always in the design preview.
 */
import { useEffect, useMemo } from "react";
import { hasLiveBackend } from "./bridge";
import { onSessionsUpdated } from "./ipc/instances";
import { onConversationsChanged } from "./ipc/conversations";
import { onPlacesChanged } from "./ipc/places";
import { relationsCounts, relationsRelated, type EntityKind, type RelationCountsInfo } from "./ipc/relations";
import type { RelatedIdsInfo } from "../types/generated/RelatedIdsInfo";
import { onPoolsChanged } from "./pools";
import { sharedReads, useSharedRead } from "./shared-read";
import { coalesced } from "./shown-window";

const KEY = ":";

const countsOf = sharedReads<RelationCountsInfo | undefined>(async (key) => {
  const at = key.indexOf(KEY);
  const [answer] = await relationsCounts([{ kind: key.slice(0, at) as EntityKind, id: key.slice(at + KEY.length) }]);
  return answer;
});

/** AJ-E3104: which of one kind an entity's sessions are on, by
 * `of${KEY}kind${KEY}id`. */
const relatedOf = sharedReads<RelatedIdsInfo | undefined>(async (key) => {
  const [of, kind, ...id] = key.split(KEY);
  return relationsRelated({ kind: kind as EntityKind, id: id.join(KEY) }, of as EntityKind);
});

/** Read `source` again when a change could move a count: sessions, the
 * tools' conversations, the apps' places, the pools. A burst of them (a
 * pass reading many files, sessions at work) reads at once and then at
 * most once a second (AJ-E805). */
function useReadAgain(source: { refresh: () => Promise<unknown> } | null): void {
  useEffect(() => {
    if (!source) return;
    const reads = coalesced(() => void source.refresh());
    const stops = [onSessionsUpdated(reads.ask), onConversationsChanged(reads.ask), onPlacesChanged(reads.ask), onPoolsChanged(reads.ask)];
    return () => {
      reads.stop();
      stops.forEach((stop) => stop());
    };
  }, [source]);
}

/** P6: projects' counts in one read (a list of them: the Projects
 * screen's page), keyed by their ids, one per line. */
const projectsOf = sharedReads<ReadonlyMap<string, RelationCountsInfo>>(async (key) => {
  const answers = await relationsCounts(key.split("\n").map((id) => ({ kind: "project" as const, id })));
  return new Map(answers.map((answer) => [answer.id, answer]));
}, 4);

/** The projects `ids`' counts, by id, read in one call (at most 200) and
 * kept fresh as `useRelationCounts`; `undefined` until read, and always in
 * the design preview. */
export function useProjectsRelationCounts(ids: readonly string[]): ReadonlyMap<string, RelationCountsInfo> | undefined {
  const live = hasLiveBackend();
  const key = live && ids.length > 0 ? ids.slice(0, 200).join("\n") : null;
  const source = key ? projectsOf(key) : null;
  useReadAgain(source);
  return useSharedRead(source) ?? undefined;
}

/** `kind` `id`'s counts (`null` kind: none asked). */
export function useRelationCounts(kind: EntityKind | null, id: string | null | undefined): RelationCountsInfo | undefined {
  const live = hasLiveBackend();
  const key = live && kind && id != null ? `${kind}${KEY}${id}` : null;
  const source = key ? countsOf(key) : null;
  useReadAgain(source);
  return useSharedRead(source) ?? undefined;
}

/** The related `of`s of `kind` `id`, read and kept fresh; `undefined`
 * until read, and always in the design preview. */
function useRelated(kind: EntityKind | null, id: string | null | undefined, of: EntityKind) {
  const live = hasLiveBackend();
  const key = live && kind && id != null ? `${of}${KEY}${kind}${KEY}${id}` : null;
  const source = key ? relatedOf(key) : null;
  useReadAgain(source);
  return useSharedRead(source);
}

/** AJ-E3104: the ids of the `of`s `kind` `id`'s sessions are on (its
 * Pools tab's pools), sorted; `undefined` until read, and always in the
 * design preview (its pages work them out from what they show). */
export function useRelatedIds(kind: EntityKind | null, id: string | null | undefined, of: EntityKind): string[] | undefined {
  return useRelated(kind, id, of)?.ids;
}

/** How many of `kind` `id`'s sessions each related `of` shares (walk K1: a
 * project's Apps tab counted a sample of its sessions); `undefined` until
 * read, and in the design preview. */
export function useRelatedCounts(kind: EntityKind | null, id: string | null | undefined, of: EntityKind): ReadonlyMap<string, number> | undefined {
  const info = useRelated(kind, id, of);
  return useMemo(() => (info ? new Map(info.ids.map((one, index) => [one, info.sessions[index] ?? 0])) : undefined), [info]);
}

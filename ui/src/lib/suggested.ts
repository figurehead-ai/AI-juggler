/**
 * E31 (AJ-E3105; D-62 decision 21; QUALITY.md "Finding things") — what a
 * picker of things shows first: the ones open now, then the most used,
 * from the relation counts (each one's sessions, AI Juggler's and the
 * ones other apps started), read in one ask for the whole list. A picker
 * of a few shows them all in view (the Picker draws groups from 6); the
 * design preview, with no counts, shows All alone.
 */
import { useMemo } from "react";
import type { PickerGroup } from "../components/controls/Picker";
import { hasLiveBackend } from "./bridge";
import { relationsCounts, type EntityKind, type RelationCountsInfo } from "./ipc/relations";
import { sharedReads, useSharedRead } from "./shared-read";

/** How many the Most used group holds. */
export const MOST_USED = 5;

const countsOf = sharedReads<RelationCountsInfo[]>(async (key) => {
  const [kind, ...ids] = key.split("\n");
  return relationsCounts(ids.map((id) => ({ kind: kind as EntityKind, id })));
});

/** Open now, then the most used of the rest, of `ids` (each a `kind`). */
export function useSuggested(kind: EntityKind, ids: readonly string[]): PickerGroup<string>[] {
  const key = hasLiveBackend() && ids.length > 0 ? [kind, ...[...ids].sort()].join("\n") : null;
  const counts = useSharedRead(key ? countsOf(key) : null, { maxAgeMs: 30_000 });
  return useMemo(() => suggestedGroups(counts ?? []), [counts]);
}

/** The groups from each one's counts (pure: unit-tested). */
export function suggestedGroups(counts: readonly RelationCountsInfo[]): PickerGroup<string>[] {
  const open = counts.filter((row) => row.sessions.open > 0).sort((a, b) => b.sessions.open - a.sessions.open);
  const shown = new Set(open.map((row) => row.id));
  const used = counts
    .filter((row) => !shown.has(row.id) && row.sessions.all > 0)
    .sort((a, b) => b.sessions.all - a.sessions.all || a.id.localeCompare(b.id))
    .slice(0, MOST_USED);
  return [
    ...(open.length > 0 ? [{ label: "Open now", values: open.map((row) => row.id) }] : []),
    ...(used.length > 0 ? [{ label: "Most used", values: used.map((row) => row.id) }] : []),
  ];
}

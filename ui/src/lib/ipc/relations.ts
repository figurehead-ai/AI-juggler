/**
 * AJ-E3102 — the relation counts (the backend's `features::relations`):
 * how many sessions each page's lists hold, and how many projects, Apps,
 * Accounts and the rest they're on, from a cache kept current by stamp.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { EntityKind } from "../../types/generated/EntityKind";
import type { EntityRef } from "../../types/generated/EntityRef";
import type { RelationCountsInfo } from "../../types/generated/RelationCountsInfo";
import type { RelatedIdsInfo } from "../../types/generated/RelatedIdsInfo";

export type { EntityKind, EntityRef, RelationCountsInfo, RelatedIdsInfo };

/** Each entity's counts, in order (at most 200 at once). The design
 * preview has none: its pages count what they show. */
export async function relationsCounts(entities: EntityRef[]): Promise<RelationCountsInfo[]> {
  if (!hasLiveBackend() || entities.length === 0) return [];
  return call("relations_counts", { entities });
}

/** AJ-E3104: which things of kind `of` an entity's sessions are on (a
 * page's Pools and Apps tabs), from the same cache as its counts. The
 * design preview has none: its pages work them out from what they show. */
export async function relationsRelated(entity: EntityRef, of: EntityKind): Promise<RelatedIdsInfo | undefined> {
  if (!hasLiveBackend()) return undefined;
  return call("relations_related", { entity, of });
}

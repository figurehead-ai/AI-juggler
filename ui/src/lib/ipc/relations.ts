/**
 * AJ-E3102 — the relation counts (the backend's `features::relations`):
 * how many sessions each page's lists hold, and how many projects, Apps,
 * Accounts and the rest they're on, from a cache kept current by stamp.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { EntityKind } from "../../types/generated/EntityKind";
import type { EntityRef } from "../../types/generated/EntityRef";
import type { RelationCountsInfo } from "../../types/generated/RelationCountsInfo";

export type { EntityKind, EntityRef, RelationCountsInfo };

/** Each entity's counts, in order (at most 200 at once). The design
 * preview has none: its pages count what they show. */
export async function relationsCounts(entities: EntityRef[]): Promise<RelationCountsInfo[]> {
  if (!hasLiveBackend() || entities.length === 0) return [];
  return invoke<RelationCountsInfo[]>("relations_counts", { entities });
}

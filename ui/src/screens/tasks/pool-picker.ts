/**
 * The Tasks screen's pool filter, by the pickers rule (0.3.2; AJ-E3009
 * R3): the pools with members first, the most members first (at most five
 * suggested), then every pool, the empty ones marked and last; under a
 * project, only the pools whose scope holds it (D-62: a pool over several
 * projects, or every one, shows under each).
 */
import type { PickerOption } from "../../components/controls";
import { IconPool } from "../../components/icons-actions";

/** What the filter reads of a pool. */
export type PoolChoice = { id: string; name: string; members: number; scope_all: boolean; scope: readonly (string | null)[] };

export function poolPicker(pools: readonly PoolChoice[], projectId: string | null): { options: PickerOption[]; groups: { label: string; values: string[] }[] } {
  const shown = pools.filter((pool) => !projectId || pool.scope_all || pool.scope.includes(projectId));
  const options = shown.map((pool): PickerOption => ({ value: pool.id, label: pool.name, hint: pool.members === 0 ? "No sessions" : pool.members === 1 ? "1 session" : `${pool.members} sessions`, icon: IconPool, empty: pool.members === 0 }));
  const suggested = [...shown]
    .filter((pool) => pool.members > 0)
    .sort((a, b) => b.members - a.members)
    .slice(0, 5)
    .map((pool) => pool.id);
  return { options, groups: [{ label: "With sessions", values: suggested }] };
}

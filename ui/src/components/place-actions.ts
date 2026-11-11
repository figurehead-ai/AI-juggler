/**
 * A remembered desktop app's actions in the rail (its header's and its
 * Inactive apps row's menus): Keep alive on or off (0.3.3) and, while it
 * runs, Restart… after its confirm (D-71).
 */
import { placesKeepAlive } from "../lib/ipc/places";
import type { RailGroup } from "../lib/rail-places";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import { askRestart, restartCounts } from "./RestartApp";
import { showToast } from "./Toast";

/** Keep alive on or off for `known`, said if it fails. */
export function keepAliveOf(known: KnownPlaceInfo | null): { on: boolean; toggle: () => void } | null {
  if (!known) return null;
  return {
    on: known.keep_alive,
    toggle: () => void placesKeepAlive(known.id, !known.keep_alive).catch(() => showToast({ id: "keep-alive", message: `Couldn't change keep alive for ${known.label}` })),
  };
}

/** A rail header's app actions: its Keep alive, and Restart… while it runs
 * (what it stops: the group's live sessions). */
export function headerActions(known: KnownPlaceInfo | null, group: RailGroup): { keepAlive: ReturnType<typeof keepAliveOf>; onRestart?: () => void } {
  const name = group.place?.profile ? `${group.label} · ${group.place.profile}` : group.label;
  return {
    keepAlive: keepAliveOf(known),
    onRestart: known && known.app_pid !== null ? () => askRestart({ placeId: known.id, name, ...restartCounts(group.items.filter((item) => item.live)) }) : undefined,
  };
}

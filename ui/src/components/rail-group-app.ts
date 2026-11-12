/**
 * A rail App group's desktop app (0.4.1): found by the group's key, running
 * or not. Found by its running pid only, a closed profile's header had no
 * Start, Restart or Keep alive, and its Open only brought a running app
 * forward. Open here starts one that's closed.
 */
import { errorText } from "../lib/bridge";
import { appIdOfRailKey, openApp } from "../lib/apps";
import { openOnTab } from "../pages/registry";
import { showGoTo } from "../lib/go-to";
import { placesOpen } from "../lib/ipc/places";
import { placeFor } from "../lib/places";
import { knownKey } from "../lib/rail-apps";
import type { RailGroup } from "../lib/rail-places";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import { showToast } from "./Toast";

/** The remembered app of a desktop App group, grouped by App. */
export function groupApp(lens: string, group: RailGroup, places: readonly KnownPlaceInfo[] | null): KnownPlaceInfo | null {
  if (lens !== "place" || group.place?.kind !== "desktop") return null;
  const key = group.place.key;
  return (places ?? []).find((place) => knownKey(place) === key) ?? placeFor(places ? [...places] : null, group.place.appPid);
}

/** Its app isn't running: no process for it, nor for any of its rows. */
export function groupClosed(known: KnownPlaceInfo | null, group: RailGroup): known is KnownPlaceInfo {
  return known !== null && known.app_pid === null && (group.place?.appPid ?? null) === null && !group.items.some((item) => item.place.appPid !== null);
}

/** Open: forward when it runs, started when it doesn't. */
export function openGroupApp(known: KnownPlaceInfo): void {
  showToast({ id: "go-to", message: known.app_pid !== null ? "Bringing it forward…" : `Starting ${known.label}…` });
  placesOpen(known.id).then(showGoTo, (cause) => showToast({ id: "go-to", message: errorText(cause) }));
}

/** A rail App group's App id (its page), grouped by App. 0.4.3: by its own
 * folder (an App with nothing open has no row to ask). */
export function appPageOfGroup(lens: string, group: RailGroup, places: readonly KnownPlaceInfo[] | null): string | null {
  return lens === "place" && group.place ? appIdOfRailKey(group.place.key, group.accountId ?? group.items[0]?.accountId ?? null, places) : null;
}

/** App `app`'s page on its Sessions tab. */
export function openAppSessions(app: string): void {
  openOnTab("app", "sessions");
  openApp(app);
}

/** D-100: an App's sessions that aren't open: its page's Sessions tab
 * (grouped by App); its header menu's "Show recent". */
export function recentOfGroup(lens: string, group: RailGroup, places: readonly KnownPlaceInfo[] | null): (() => void) | undefined {
  const app = appPageOfGroup(lens, group, places);
  return app ? () => openAppSessions(app) : undefined;
}

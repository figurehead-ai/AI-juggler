/**
 * AJ-E3204 R3 — "Show in the rail", per App, set in the App's own
 * Settings. One stored list of the Apps kept off (by App id), read once
 * and shared; an App not on it shows (the default: nothing kept).
 *
 * Settings › Desktop apps hides a whole tool's sessions everywhere; this
 * only takes one App's group out of the rail's by-App view. Its sessions
 * that need you still show, and a search still finds it.
 */
import { appSettingGet, appSettingSet } from "./ipc/settings";
import { sharedRead, useSharedRead } from "./shared-read";
import { cliAppId } from "./app-name";
import { openApp } from "./apps";
import { openOnTab } from "../pages/registry";
import { knownKey } from "./rail-apps";
import type { KnownPlaceInfo } from "./ipc/places";

/** The stored setting: a JSON list of App ids kept off the rail. */
const RAIL_HIDDEN_KEY = "rail.hidden_apps";

/** The ids in a stored value (anything unreadable is nothing kept). */
export function parseHidden(raw: string | null): ReadonlySet<string> {
  if (!raw) return new Set();
  try {
    const list: unknown = JSON.parse(raw);
    return new Set(Array.isArray(list) ? list.filter((id): id is string => typeof id === "string") : []);
  } catch {
    return new Set();
  }
}

const HIDDEN = sharedRead(async () => parseHidden(await appSettingGet(RAIL_HIDDEN_KEY)));

/** The App ids kept off the rail (empty until read). */
export function useRailHidden(): ReadonlySet<string> {
  return useSharedRead(HIDDEN) ?? NONE;
}
const NONE: ReadonlySet<string> = new Set();

/** Shows or keeps one App off the rail. */
export async function setShownInRail(appId: string, shown: boolean): Promise<void> {
  const next = new Set(parseHidden(await appSettingGet(RAIL_HIDDEN_KEY)));
  if (shown) next.delete(appId);
  else next.add(appId);
  await appSettingSet(RAIL_HIDDEN_KEY, next.size === 0 ? "" : JSON.stringify([...next].sort()));
  await HIDDEN.refresh();
}

/** The App id of a rail key: a CLI App's from its folder
 * (`cli|<tool>|<folder>`), a desktop App's from the remembered places. */
export function appIdOfKey(key: string, places: readonly KnownPlaceInfo[] | null): string | null {
  if (key.startsWith("cli|")) {
    const folder = key.split("|")[2];
    return folder ? cliAppId(folder) : null;
  }
  return places?.find((place) => knownKey(place) === key)?.id ?? null;
}

/** The rail's side: whether a key's App is kept off, and the Also line's
 * way back (that App's Settings, where it's turned on again). */
export function useOffRail(places: readonly KnownPlaceInfo[] | null): { offRail: (key: string) => boolean; openOffRail: (key: string) => void } {
  const hidden = useRailHidden();
  return {
    offRail: (key) => hidden.size > 0 && hidden.has(appIdOfKey(key, places) ?? ""),
    openOffRail: (key) => {
      const id = appIdOfKey(key, places);
      if (!id) return;
      openOnTab("app", "settings");
      openApp(id);
    },
  };
}

/**
 * D-62 (AJ-E3203) — finding a desktop app's profiles that aren't running.
 * An app that keeps its profiles side by side (the backend's adapters
 * say which) has its other profiles in the folder beside one that ran:
 * AI Juggler reads the names of the folders there, never anything inside
 * them, once the user says yes on an App's page. Settings › Desktop apps
 * turns it off; off forgets the profiles found that way.
 */
import { useCallback, useEffect, useState } from "react";
import { onPlacesChanged, placesList, placesProfiles, placesProfilesSet, type FindProfilesInfo, type KnownPlaceInfo } from "./ipc/places";

export type { FindProfilesInfo };

/** Where finding stands (`null` until read), again whenever a place
 * changes; `answer` keeps the user's yes or no. */
export function useFindProfiles(): { info: FindProfilesInfo | null; answer: (on: boolean) => Promise<FindProfilesInfo> } {
  const [info, setInfo] = useState<FindProfilesInfo | null>(null);
  useEffect(() => {
    let live = true;
    const load = () =>
      void placesProfiles().then(
        (next) => live && setInfo(next),
        () => live && setInfo((was) => was ?? { answer: "off", folders: [] }),
      );
    load();
    const stop = onPlacesChanged(load);
    return () => {
      live = false;
      stop();
    };
  }, []);
  const answer = useCallback(async (on: boolean) => {
    const next = await placesProfilesSet(on);
    setInfo(next);
    return next;
  }, []);
  return { info, answer };
}

/** The folder whose names a yes reads for app `appId` (its icon's id),
 * when there's one to ask about: not answered yet, and a profile of that
 * app ran beside others. */
export function askFolder(info: FindProfilesInfo | null, appId: string | null): string | null {
  if (!info || info.answer !== "ask" || appId === null) return null;
  return info.folders.find((folder) => folder.app_id === appId)?.folder ?? null;
}

/** The profiles of app `appId` found that way, by their folder's name. */
export function foundProfiles(places: readonly KnownPlaceInfo[], appId: string): string[] {
  return places
    .filter((place) => place.found && place.app_id === appId)
    .map((place) => place.instance ?? place.label)
    .sort((a, b) => a.localeCompare(b));
}

/** What a yes found, in one line. */
export function foundWords(names: readonly string[]): string {
  if (names.length === 0) return "No other profiles there. Any you add show here once one runs.";
  const list = names.length <= 4 ? names.join(", ") : `${names.slice(0, 3).join(", ")} and ${names.length - 3} more`;
  return `Found ${names.length === 1 ? "1 more profile" : `${names.length} more profiles`}: ${list}. Each is an app of its own now, with Start.`;
}

/** Says yes, then the profiles of `appId` it found. */
export async function findNow(answer: (on: boolean) => Promise<FindProfilesInfo>, appId: string): Promise<string[]> {
  await answer(true);
  return foundProfiles(await placesList(), appId);
}

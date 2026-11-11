/**
 * D-62 (AJ-E3203) — finding a desktop app's profiles that aren't running.
 * An app that keeps its profiles side by side (the backend's adapters
 * say which) has its other profiles in the folder beside one that ran:
 * AI Juggler reads the names of the folders there, never anything inside
 * them, once the user says yes on an App's page. Settings › Desktop apps
 * turns it off; off forgets the profiles found that way.
 */
import { useCallback, useEffect, useState } from "react";
import { onPlacesChanged, placesProfiles, placesProfilesSet, type FindProfilesInfo } from "./ipc/places";

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

/** Says yes: AI Juggler reads the names beside the profile that ran,
 * and offers the ones not added (D-84: never added by themselves). */
export async function findNow(answer: (on: boolean) => Promise<FindProfilesInfo>): Promise<void> {
  await answer(true);
}

/**
 * 0.3.3 — the apps and profiles remembered, for the screens that show
 * them (Home's tiles, the rail's place, Settings › Desktop apps): read
 * once, again whenever one changes.
 */
import { useEffect, useState } from "react";
import { onPlacesChanged, placesFind, placesList, type KnownPlaceInfo } from "./ipc/places";

/** The places remembered (`null` until read). */
export function useKnownPlaces(): KnownPlaceInfo[] | null {
  const [places, setPlaces] = useState<KnownPlaceInfo[] | null>(null);
  useEffect(() => {
    let live = true;
    const load = () =>
      void placesList().then(
        (rows) => live && setPlaces(rows),
        () => live && setPlaces((was) => was ?? []),
      );
    load();
    const stop = onPlacesChanged(load);
    return () => {
      live = false;
      stop();
    };
  }, []);
  return places;
}

/** The app an ended session was last open in, when one is remembered
 * (`null` while it's open, or none is). */
export function useLastPlace(tool: string, conversationId: string, ended: boolean): KnownPlaceInfo | null {
  const [place, setPlace] = useState<KnownPlaceInfo | null>(null);
  useEffect(() => {
    if (!ended) {
      setPlace(null);
      return;
    }
    let live = true;
    const load = () => void placesFind(tool, conversationId).then((found) => live && setPlace(found), () => null);
    load();
    const stop = onPlacesChanged(load);
    return () => {
      live = false;
      stop();
    };
  }, [tool, conversationId, ended]);
  return place;
}

/** The remembered place a running app is: the same process (a profile's
 * name on the rail is its account's, so names don't tell). */
export function placeFor(places: KnownPlaceInfo[] | null, appPid: number | null): KnownPlaceInfo | null {
  if (!places || appPid === null) return null;
  return places.find((place) => place.app_pid === appPid) ?? null;
}

/** A place's state in words: "Running", "Not running · seen 2 h ago",
 * "Keep alive paused: it stopped 3 times in 10 minutes". */
export function placeWords(place: KnownPlaceInfo, now: number = Date.now()): string {
  if (place.paused) return `Keep alive paused: ${place.paused}`;
  if (place.app_pid !== null) return place.keep_alive ? "Running · kept alive" : "Running";
  if (place.keep_alive) return "Not running · keep alive starts it";
  return `Not running · seen ${ago(place.seen_ms, now)}`;
}

function ago(ms: number, now: number): string {
  const minutes = Math.max(0, Math.round((now - ms) / 60_000));
  if (minutes < 60) return "this hour";
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  return days === 1 ? "yesterday" : `${days} days ago`;
}

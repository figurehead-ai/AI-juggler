/**
 * 0.3.3 — the apps and profiles remembered, for the screens that show
 * them (Home's tiles, the rail's place, Settings › Desktop apps): read
 * once, again whenever one changes.
 */
import { useEffect, useState } from "react";
import { sharedRead, useSharedRead, whenForgotten, type SharedRead } from "./shared-read";
import { onPlacesChanged, placesFind, placesList, type KnownPlaceInfo } from "./ipc/places";
import { learnPlaces } from "./marks";
import { agoWords } from "./time-words";

/** The places remembered, one read for every screen that shows them
 * (walk A4: seven hooks each read them, ~20 reads a window's first
 * seconds). Kept, and read again when one changes. */
const PLACES: SharedRead<KnownPlaceInfo[]> = sharedRead(async () => {
  try {
    const rows = await placesList();
    learnPlaces(rows); // AJ-E3206: each desktop App's stored colour
    return rows;
  } catch {
    return PLACES.peek() ?? [];
  }
});
let listening = false;
function listen(): void {
  if (listening) return;
  listening = true;
  onPlacesChanged(() => void PLACES.refresh());
  whenForgotten(() => (listening = false));
}

/** Reads the remembered places again now (a start just came up). */
export function refreshKnownPlaces(): Promise<KnownPlaceInfo[] | undefined> {
  return PLACES.refresh();
}

/** The places remembered (`null` until read). */
export function useKnownPlaces(): KnownPlaceInfo[] | null {
  useEffect(listen, []);
  return useSharedRead(PLACES, { maxAgeMs: FRESH_MS }) ?? null;
}

/** A read this young is shown as it is (screens mounting together). */
const FRESH_MS = 5_000;

/** The app an ended session was last open in, when one is remembered
 * (`null` while it's open, or none is). */
/** Where a session's files say it started: a desktop app, on a folder. */
export interface StartedIn {
  account: string;
  app: string;
}

export function useLastPlace(tool: string, conversationId: string, ended: boolean, startedIn: StartedIn | null = null): KnownPlaceInfo | null {
  const account = startedIn?.account ?? null;
  const app = startedIn?.app ?? null;
  const [place, setPlace] = useState<KnownPlaceInfo | null>(null);
  useEffect(() => {
    if (!ended) {
      setPlace(null);
      return;
    }
    let live = true;
    // 0.4.1: a session started in a desktop app that AI Juggler never saw
    // open is that app's profile on its folder, when the user has one.
    const load = () => void placesFind(tool, conversationId, account, app).then((found) => live && setPlace(found), () => null);
    load();
    const stop = onPlacesChanged(load);
    return () => {
      live = false;
      stop();
    };
  }, [tool, conversationId, ended, account, app]);
  return place;
}

/** The remembered place a running app is: the same process (a profile's
 * name on the rail is its account's, so names don't tell). */
export function placeFor(places: KnownPlaceInfo[] | null, appPid: number | null): KnownPlaceInfo | null {
  if (!places || appPid === null) return null;
  return places.find((place) => place.app_pid === appPid) ?? null;
}

/** A place's state in words: "Running", "Closed · last open 12 min ago",
 * "Keep alive paused: it stopped 3 times in 10 minutes". D-103.5: one that
 * isn't running is "Closed" (was "Not running · seen this hour"), with
 * when it was last seen open in the app's ago-words. */
export function placeWords(place: KnownPlaceInfo, now: number = Date.now()): string {
  if (place.lost) return "Lost its window when the system restarted the display";
  if (place.paused) return `Keep alive paused: ${place.paused}`;
  if (place.app_pid !== null) return place.keep_alive ? "Running · kept alive" : "Running";
  if (place.keep_alive) return "Closed · keep alive starts it";
  // AJ-E3203: found by its folder's name, not seen running yet.
  if (place.found) return "Found with your other profiles";
  return `Closed · last open ${agoWords(place.seen_ms, now)}`;
}

/** The desktop app (its name, "Claude") another app's session was last
 * seen open in, from the remembered places; `undefined` when none was
 * (0.4.1: a closed one's row names only its tool). */
export function lastDesktopApp(places: readonly KnownPlaceInfo[] | null, accountId: string, tool: string, id: string): string | undefined {
  const key = `${tool}/${id}`;
  return places?.find((place) => (place.account_id === null || place.account_id === accountId) && place.recent.includes(key))?.app;
}

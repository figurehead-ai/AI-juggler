/**
 * 0.3.3 — the apps and profiles AI Juggler remembers (the backend's
 * `features::places`): Open starts one that isn't running, Keep alive
 * starts it again when it stops. The backend says when they change
 * (`places-changed`). The preview and tests set `window.__AIJ_PLACES__`.
 */
import { call, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import { previewFolders } from "./accounts";
import type { GoToOutcomeInfo } from "../../types/generated/GoToOutcomeInfo";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import type { FindProfilesInfo } from "../../types/generated/FindProfilesInfo";
import type { PlaceOfferInfo } from "../../types/generated/PlaceOfferInfo";
import { previewHooks } from "../preview-hooks";

export type { FindProfilesInfo, KnownPlaceInfo };

/** The event the backend sends when a place changed. */
export const PLACES_CHANGED = "places-changed";

/** What one page asks for: every place (the backend keeps at most 32). */
const ALL = { cursor: null, limit: 50 };

interface PreviewPlaces {
  places: KnownPlaceInfo[];
  /** `tool/id` → place id: where an ended session was last open. */
  sessions?: Record<string, string>;
  /** AJ-E3203: finding the profiles that aren't running, and the ones a
   * yes finds. */
  profiles?: FindProfilesInfo & { beside?: KnownPlaceInfo[] };
  /** D-84: found running, not added. */
  offers?: PlaceOfferInfo[];
}

/** The preview's places: a spec's fixture, else the places desktop's
 * (`sessionStorage["aij.preview.places"]`), else none. */
function preview(): PreviewPlaces {
  const host = window as unknown as { __AIJ_PLACES__?: PreviewPlaces; __AIJ_FIXTURE__?: unknown };
  if (!host.__AIJ_PLACES__) {
    let desktop = false;
    try {
      desktop = sessionStorage.getItem("aij.preview.places") === "1";
    } catch {
      desktop = false;
    }
    host.__AIJ_PLACES__ = desktop && !host.__AIJ_FIXTURE__ ? previewHooks().places.knownPlacesPreview(Date.now()) : { places: [] };
  }
  return host.__AIJ_PLACES__;
}

/** Tells this window's listeners (the preview has no backend to). */
function changedHere(): void {
  window.dispatchEvent(new CustomEvent(PLACES_CHANGED));
}

/** Every place remembered, as the backend orders them (`place_order`):
 * running first, then the rest by when they were last seen, then the ones
 * found and never run; then by name. */
export async function placesList(): Promise<KnownPlaceInfo[]> {
  if (!hasLiveBackend()) {
    // AJ-E3206: each with the colour the backend would give, after the
    // CLI Apps' (filled once, kept).
    previewHooks().slots.previewPlaceSlots(preview().places, previewFolders());
    const group = (place: KnownPlaceInfo) => (place.app_pid !== null ? 0 : place.found ? 2 : 1);
    return [...preview().places].sort((a, b) => group(a) - group(b) || b.seen_ms - a.seen_ms || a.label.localeCompare(b.label));
  }
  const page = await call("places_list", { page: ALL });
  return page.items;
}

/** The place an ended session of `tool` was last open in. */
export async function placesFind(tool: string, conversationId: string, account: string | null = null, app: string | null = null): Promise<KnownPlaceInfo | null> {
  if (!hasLiveBackend()) {
    const id = preview().sessions?.[`${tool}/${conversationId}`];
    return preview().places.find((place) => place.id === id) ?? null;
  }
  return call("places_find", { tool, conversationId, account, app });
}

/** Open: the app forward when it runs, else started; at the session when
 * its app has a link to it. */
export async function placesOpen(id: string, conversationId: string | null = null): Promise<GoToOutcomeInfo> {
  if (!hasLiveBackend()) {
    const place = preview().places.find((row) => row.id === id);
    if (!place) return { outcome: "AI Juggler doesn't remember that app", reached: "none" };
    place.paused = null;
    if (place.lost) return { outcome: `${place.label} lost its window when the system restarted the display. Restart it?`, reached: "lost", place_id: place.id };
    const running = place.app_pid !== null;
    if (!running) place.app_pid = 90_000 + preview().places.indexOf(place);
    changedHere();
    return running ? { outcome: `Brought ${place.label} forward`, reached: "app" } : { outcome: `Starting ${place.label}`, reached: "started" };
  }
  return call("places_open", { id, conversationId });
}

/** 0.3.8: quits an app or profile that lost its window when macOS
 * restarted its display, and starts it again as it ran. D-71: with
 * `confirmed`, a running one with its window too (the user confirmed what
 * it stops). */
export async function placesRestart(id: string, confirmed = false): Promise<GoToOutcomeInfo> {
  if (!hasLiveBackend()) {
    const place = preview().places.find((row) => row.id === id);
    if (!place) return { outcome: "AI Juggler doesn't remember that app any more", reached: "none" };
    place.lost = false;
    place.paused = null;
    place.app_pid = 91_000 + preview().places.indexOf(place);
    changedHere();
    return { outcome: `Restarting ${place.label}`, reached: "started" };
  }
  return call("places_restart", { id, confirmed });
}

/** Keep alive on or off. */
export async function placesKeepAlive(id: string, on: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const place = preview().places.find((row) => row.id === id);
    if (place) {
      place.keep_alive = on;
      place.paused = null;
    }
    changedHere();
    return;
  }
  return call("places_keep_alive", { id, on });
}

/** Forgets a place (remembered again the next time it runs). */
export async function placesForget(id: string): Promise<void> {
  if (!hasLiveBackend()) {
    preview().places = preview().places.filter((row) => row.id !== id);
    changedHere();
    return;
  }
  return call("places_forget", { id });
}

/** AJ-E3203: whether AI Juggler finds the profiles that aren't running
 * (asked once), and the folders whose names it reads. */
export async function placesProfiles(): Promise<FindProfilesInfo> {
  if (!hasLiveBackend()) {
    const profiles = preview().profiles;
    return { answer: profiles?.answer ?? "ask", folders: profiles?.folders ?? [] };
  }
  return call("places_profiles");
}

/** AJ-E3203: the user's answer. Yes reads the folder names now; no
 * forgets the profiles found that way. */
export async function placesProfilesSet(on: boolean): Promise<FindProfilesInfo> {
  if (!hasLiveBackend()) {
    const host = preview();
    const profiles = (host.profiles ??= { answer: "ask", folders: [] });
    profiles.answer = on ? "on" : "off";
    host.places = host.places.filter((place) => !place.found);
    // D-84, as the backend: a yes offers the profiles beside the one that
    // ran (Add makes one an App); a no forgets the offers too.
    const beside = new Set((profiles.beside ?? []).map((place) => place.id));
    host.offers = (host.offers ?? []).filter((offer) => !beside.has(offer.id));
    if (on) host.offers.push(...(profiles.beside ?? []).map((place) => ({ id: place.id, label: `Claude app · ${place.instance ?? place.label}` })));
    changedHere();
    return { answer: profiles.answer, folders: profiles.folders };
  }
  const info = await call("places_profiles_set", { on });
  changedHere();
  return info;
}

/** Calls `on` whenever a place changes; returns the unsubscribe. */
export function onPlacesChanged(on: () => void): () => void {
  window.addEventListener(PLACES_CHANGED, on);
  let stop: (() => void) | null = null;
  let gone = false;
  if (hasLiveBackend()) {
    void listen<null>(PLACES_CHANGED, () => on()).then((unlisten) => {
      if (gone) unlisten();
      else stop = unlisten;
    });
  }
  return () => {
    gone = true;
    window.removeEventListener(PLACES_CHANGED, on);
    stop?.();
  };
}

/** D-84: the apps found running that aren't the user's yet ("Found on
 * this Mac"); none without a backend. */
export async function placesOffers(): Promise<PlaceOfferInfo[]> {
  if (!hasLiveBackend()) return [...(preview().offers ?? [])];
  return call("places_offers");
}

/** D-84: Add an offer as an App, or Not now. */
export async function placesOfferAnswer(id: string, add: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    const now = preview();
    now.offers = (now.offers ?? []).filter((offer) => offer.id !== id);
    const found = now.profiles?.beside?.find((place) => place.id === id);
    if (add && found) now.places.push({ ...found });
  } else {
    await call("places_offer_answer", { id, add });
  }
  changedHere();
}

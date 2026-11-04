/**
 * 0.3.3 — the apps and profiles AI Juggler remembers (the backend's
 * `features::places`): Open starts one that isn't running, Keep alive
 * starts it again when it stops. The backend says when they change
 * (`places-changed`). The preview and tests set `window.__AIJ_PLACES__`.
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend, previewFolders } from "../bridge";
import { previewPlaceSlots } from "./preview-slots";
import type { GoToOutcomeInfo } from "../../types/generated/GoToOutcomeInfo";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import type { FindProfilesInfo } from "../../types/generated/FindProfilesInfo";
import { knownPlacesPreview } from "../../preview/places";

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
    host.__AIJ_PLACES__ = desktop && !host.__AIJ_FIXTURE__ ? knownPlacesPreview(Date.now()) : { places: [] };
  }
  return host.__AIJ_PLACES__;
}

/** Tells this window's listeners (the preview has no backend to). */
function changedHere(): void {
  window.dispatchEvent(new CustomEvent(PLACES_CHANGED));
}

/** Every place remembered: kept alive first, then by name. */
export async function placesList(): Promise<KnownPlaceInfo[]> {
  if (!hasLiveBackend()) {
    // AJ-E3206: each with the colour the backend would give, after the
    // CLI Apps' (filled once, kept).
    previewPlaceSlots(preview().places, previewFolders());
    return [...preview().places].sort((a, b) => Number(b.keep_alive) - Number(a.keep_alive) || a.label.localeCompare(b.label));
  }
  const page = await invoke<{ items: KnownPlaceInfo[] }>("places_list", { page: ALL });
  return page.items;
}

/** The place an ended session of `tool` was last open in. */
export async function placesFind(tool: string, conversationId: string): Promise<KnownPlaceInfo | null> {
  if (!hasLiveBackend()) {
    const id = preview().sessions?.[`${tool}/${conversationId}`];
    return preview().places.find((place) => place.id === id) ?? null;
  }
  return invoke<KnownPlaceInfo | null>("places_find", { tool, conversationId });
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
  return invoke<GoToOutcomeInfo>("places_open", { id, conversationId });
}

/** 0.3.8: quits an app or profile that lost its window when macOS
 * restarted its display, and starts it again as it ran. */
export async function placesRestart(id: string): Promise<GoToOutcomeInfo> {
  if (!hasLiveBackend()) {
    const place = preview().places.find((row) => row.id === id);
    if (!place) return { outcome: "AI Juggler doesn't remember that app any more", reached: "none" };
    place.lost = false;
    place.paused = null;
    place.app_pid = 91_000 + preview().places.indexOf(place);
    changedHere();
    return { outcome: `Restarting ${place.label}`, reached: "started" };
  }
  return invoke<GoToOutcomeInfo>("places_restart", { id });
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
  return invoke<void>("places_keep_alive", { id, on });
}

/** Forgets a place (remembered again the next time it runs). */
export async function placesForget(id: string): Promise<void> {
  if (!hasLiveBackend()) {
    preview().places = preview().places.filter((row) => row.id !== id);
    changedHere();
    return;
  }
  return invoke<void>("places_forget", { id });
}

/** AJ-E3203: whether AI Juggler finds the profiles that aren't running
 * (asked once), and the folders whose names it reads. */
export async function placesProfiles(): Promise<FindProfilesInfo> {
  if (!hasLiveBackend()) {
    const profiles = preview().profiles;
    return { answer: profiles?.answer ?? "ask", folders: profiles?.folders ?? [] };
  }
  return invoke<FindProfilesInfo>("places_profiles");
}

/** AJ-E3203: the user's answer. Yes reads the folder names now; no
 * forgets the profiles found that way. */
export async function placesProfilesSet(on: boolean): Promise<FindProfilesInfo> {
  if (!hasLiveBackend()) {
    const host = preview();
    const profiles = (host.profiles ??= { answer: "ask", folders: [] });
    profiles.answer = on ? "on" : "off";
    host.places = host.places.filter((place) => !place.found);
    if (on) host.places.push(...(profiles.beside ?? []).map((place) => ({ ...place })));
    changedHere();
    return { answer: profiles.answer, folders: profiles.folders };
  }
  const info = await invoke<FindProfilesInfo>("places_profiles_set", { on });
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

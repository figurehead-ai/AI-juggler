/**
 * 0.3.3 — the apps and profiles AI Juggler remembers (the backend's
 * `features::places`): Open starts one that isn't running, Keep alive
 * starts it again when it stops. The backend says when they change
 * (`places-changed`). The preview and tests set `window.__AIJ_PLACES__`.
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { GoToOutcomeInfo } from "../../types/generated/GoToOutcomeInfo";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import { knownPlacesPreview } from "../../preview/places";

export type { KnownPlaceInfo };

/** The event the backend sends when a place changed. */
export const PLACES_CHANGED = "places-changed";

/** What one page asks for: every place (the backend keeps at most 32). */
const ALL = { cursor: null, limit: 50 };

interface PreviewPlaces {
  places: KnownPlaceInfo[];
  /** `tool/id` → place id: where an ended session was last open. */
  sessions?: Record<string, string>;
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
    const running = place.app_pid !== null;
    if (!running) place.app_pid = 90_000 + preview().places.indexOf(place);
    changedHere();
    return running ? { outcome: `Brought ${place.label} forward`, reached: "app" } : { outcome: `Starting ${place.label}`, reached: "started" };
  }
  return invoke<GoToOutcomeInfo>("places_open", { id, conversationId });
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

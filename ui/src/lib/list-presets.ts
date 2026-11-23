/**
 * AJ-E1554 R4: a list opened on a state filter from elsewhere (a rail
 * group's "+12 more" opens its App's Sessions on Open now): the count's
 * way to exactly what it counts. Kept apart from the list itself, so the
 * rail doesn't load the page's list to set it.
 */
import type { StateFilter } from "../components/related/order";

const waiting = new Map<string, StateFilter>();
const listeners = new Set<(listKey: string) => void>();

/** Opens the list keyed `listKey` (when it mounts, or now if it's open) on `state`. */
export function presetListState(listKey: string, state: StateFilter): void {
  waiting.set(listKey, state);
  for (const listener of listeners) listener(listKey);
}

/** An open list hears a preset meant for it (`takeListPreset` reads it). */
export function onListPreset(listener: (listKey: string) => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

/** The state a list was asked to open on, once (`null`: none). */
export function takeListPreset(listKey: string): StateFilter | null {
  const state = waiting.get(listKey) ?? null;
  waiting.delete(listKey);
  return state;
}

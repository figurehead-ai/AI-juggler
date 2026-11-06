/**
 * D-62 (AJ-E3204) — one App's sessions and projects, from the rail's own
 * items and its own rule (`appKeyOf`): the App page lists exactly what
 * the rail puts under the App. Pure, so it's unit-tested.
 */
import { appKeyOf } from "./rail-apps";
import type { RailItem } from "./rail-items";

/** Running in its app now, in the background too (a pin doesn't make it
 * so here). */
export function openHere(item: RailItem): boolean {
  return item.live && item.state !== "ended";
}

/** App `railKey`'s sessions (archived ones left out): open first, then
 * the most recent. */
export function appItems(items: readonly RailItem[], railKey: string, byConversation: ReadonlyMap<string, string>, byFolder: ReadonlyMap<string, string> = new Map()): RailItem[] {
  return items
    .filter((item) => !item.archived && appKeyOf(item, byConversation, byFolder) === railKey)
    .sort((a, b) => Number(openHere(b)) - Number(openHere(a)) || b.lastMs - a.lastMs);
}

export interface AppProject {
  id: string;
  name: string;
  /** Its sessions in this App. */
  sessions: number;
  /** Its newest session's activity here, epoch ms. */
  lastMs: number;
}

/** The projects an App worked in, the most recently used first. */
export function appProjects(items: readonly RailItem[]): AppProject[] {
  const byId = new Map<string, AppProject>();
  for (const item of items) {
    if (!item.projectId) continue;
    const known = byId.get(item.projectId);
    if (known) {
      known.sessions += 1;
      known.lastMs = Math.max(known.lastMs, item.lastMs);
    } else {
      byId.set(item.projectId, { id: item.projectId, name: item.project ?? "A project", sessions: 1, lastMs: item.lastMs });
    }
  }
  return [...byId.values()].sort((a, b) => b.lastMs - a.lastMs || a.name.localeCompare(b.name));
}

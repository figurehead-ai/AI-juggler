/**
 * AJ-E2110 — Settings › Desktop apps: which apps' open sessions AI
 * Juggler shows, as data. The apps listed before anything is found in
 * them are the host registry's common ones (ADR-0019: an app is an
 * entry, not a branch); any other app a session is found open in joins
 * the list by itself. Each app's choice is one stored setting, on until
 * turned off; the rail's views read the same choice (`useHiddenApps`).
 */
import { useEffect, useState } from "react";
import { appSettingGet, appSettingSet } from "./bridge";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";

export type DesktopAppKind = "desktop" | "terminal" | "editor";

export interface DesktopApp {
  /** The host registry's id (`PlaceInfo.app_id`). */
  id: string;
  name: string;
  kind: DesktopAppKind;
  /** What else runs in it, said while nothing is open there. */
  hosts?: string;
}

/** The apps Settings lists from the start, in this order. */
export const COMMON_APPS: readonly DesktopApp[] = [
  { id: "claude-app", name: "Claude app", kind: "desktop" },
  { id: "terminal", name: "Terminal", kind: "terminal" },
  // The host registry's name for it, as the rail and Home say it.
  { id: "codex-app", name: "ChatGPT", kind: "desktop", hosts: "Codex" },
  { id: "cursor", name: "Cursor", kind: "editor" },
];

/** What was found open in one app: its sessions and their profiles. */
export interface AppFound {
  app: DesktopApp;
  open: number;
  profiles: string[];
}

/** An app found in a place but not in the common list (its kind isn't
 * said there, so it shows as an app window). */
function appOfPlace(place: PlaceInfo): DesktopApp | null {
  if (!place.app_id) return null;
  return { id: place.app_id, name: place.app || place.app_id, kind: "desktop" };
}

/** Each app with what is open in it now: the common ones first, then
 * any other app a live session is in. */
export function appsFound(conversations: readonly ConversationInfo[]): AppFound[] {
  const found = new Map<string, AppFound>(COMMON_APPS.map((app) => [app.id, { app, open: 0, profiles: [] }]));
  for (const conversation of conversations) {
    if (!conversation.live) continue;
    const seen = new Set<string>();
    // A reader older than places (AJ-E2101) sends none.
    for (const place of conversation.places ?? []) {
      const app = appOfPlace(place);
      if (!app || seen.has(app.id)) continue;
      seen.add(app.id);
      const entry = found.get(app.id) ?? { app, open: 0, profiles: [] };
      entry.open += 1;
      const profile = place.profile ?? place.instance;
      if (profile && !entry.profiles.includes(profile)) entry.profiles.push(profile);
      found.set(app.id, entry);
    }
  }
  return [...found.values()];
}

/** "2 open · Work, Personal", or that nothing is open there now. */
export function foundWords(found: AppFound): string {
  if (found.open === 0) return found.app.hosts ? `Nothing open now. ${found.app.hosts} sessions run in it.` : "Nothing open now.";
  const count = `${found.open} open`;
  return found.profiles.length > 0 ? `${count} · ${found.profiles.join(", ")}` : count;
}

/** The stored setting for one app ("0" = its sessions aren't shown). */
export function showSessionsKey(appId: string): string {
  return `show_sessions_from.${appId}`;
}

/** Fired when an app is turned on or off. */
export const DESKTOP_APPS_EVENT = "aij:desktop-apps-changed";

export async function appShown(appId: string): Promise<boolean> {
  return (await appSettingGet(showSessionsKey(appId))) !== "0";
}

/** Turns one app's sessions on or off (on is the default: nothing kept). */
export async function setAppShown(appId: string, on: boolean): Promise<void> {
  await appSettingSet(showSessionsKey(appId), on ? "" : "0");
  window.dispatchEvent(new CustomEvent(DESKTOP_APPS_EVENT));
}

/** The apps turned off, among `ids`; re-read when one changes. */
export function useHiddenApps(ids: readonly string[]): ReadonlySet<string> {
  const [hidden, setHidden] = useState<ReadonlySet<string>>(new Set());
  const key = ids.join(",");
  useEffect(() => {
    let alive = true;
    const load = () =>
      void Promise.all(key.split(",").filter(Boolean).map(async (id) => [id, await appShown(id)] as const)).then(
        (entries) => alive && setHidden(new Set(entries.filter(([, on]) => !on).map(([id]) => id))),
        () => undefined,
      );
    load();
    window.addEventListener(DESKTOP_APPS_EVENT, load);
    return () => {
      alive = false;
      window.removeEventListener(DESKTOP_APPS_EVENT, load);
    };
  }, [key]);
  return hidden;
}

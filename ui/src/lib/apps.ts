/**
 * D-62 (AJ-E3204) — the Apps, as their page, Home and the rail's doors
 * see them: one per folder in use (a CLI tool on it: "Claude Code · Work
 * Max") and one per desktop app or profile seen running, or found beside
 * one (AJ-E3203): "Claude app · Work Max" once its sessions name its
 * config folder, else "Claude app · b-work". The stored rows give the user's name and the Account; the
 * places and folders give the rest, so an App the backend hasn't stored
 * yet (and every App in the preview) is still one.
 *
 * Each App knows its rail key, so its page lists the very sessions the
 * rail puts under it (lib/rail-apps.ts decides that, once).
 */
import { useEffect, useMemo } from "react";
import { sharedRead, useSharedRead, whenForgotten, type SharedRead } from "./shared-read";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { appName, cliAppId, cliAppKey, cliAppWord, desktopProfile } from "./app-name";
import { appOfTool } from "./app-of";
import { APPS_CHANGED, appsList, type StoredAppInfo } from "./ipc/apps";
import type { KnownPlaceInfo } from "./ipc/places";
import { useKnownPlaces } from "./places";
import { knownAppWord, knownKey } from "./rail-apps";

import { hasLiveBackend } from "./bridge";
import { appSlot } from "./marks";

export interface AppModel {
  /** `cli:<folder>` or the place's id (`app:<host>:<instance folder>`). */
  id: string;
  kind: "cli" | "desktop";
  /** Its tool's id; `null` for a desktop app no tool claims. */
  tool: string | null;
  /** Its icon's id. */
  appId: string | null;
  /** "Claude Code", "Claude app". */
  app: string;
  /** Its folder's label, or the desktop profile's name. */
  profile: string | null;
  /** The user's name for it, else `defaultName`. */
  name: string;
  /** "Claude Code · Work Max". */
  defaultName: string;
  /** Whether the user named it. */
  named: boolean;
  /** The rail's key for it. */
  railKey: string;
  /** Its config folder (a CLI App's own; a desktop App's once its
   * sessions named one). */
  folder: AccountInfo | null;
  /** The stored Account its folder was last read as. */
  accountId: string | null;
  /** A desktop App's place (Open, Keep alive, whether it runs). */
  place: KnownPlaceInfo | null;
  /** AJ-E3206: its one colour's slot, as stored (lib/marks). */
  slot: number;
}

/** The folders that are CLI Apps. D-84 (rc22, A3): a desktop profile's
 * folder is that profile's; with the backend's list (`listed`), its CLI
 * App is one only when that lists it (`apps::users_apps`: a terminal
 * session ran on it, or the user named it). The preview has no list:
 * every folder is one there. */
function cliFolders(folders: readonly AccountInfo[], places: readonly KnownPlaceInfo[], stored: ReadonlyMap<string, StoredAppInfo>, listed: boolean | undefined): readonly AccountInfo[] {
  if (listed !== true) return folders;
  const profiles = new Set(places.flatMap((place) => (place.account_id ? [place.account_id] : [])));
  return folders.filter((folder) => !profiles.has(folder.id) || stored.has(cliAppId(folder.id)));
}

/** The Apps: CLI ones in the folders' order, then the desktop ones as
 * the places list them. */
export function appsOf({
  stored,
  places,
  folders,
  listed,
}: {
  stored: readonly StoredAppInfo[];
  places: readonly KnownPlaceInfo[];
  folders: readonly AccountInfo[];
  /** `stored` is the backend's list of the user's Apps (not the preview). */
  listed?: boolean;
}): AppModel[] {
  const byId = new Map(stored.map((row) => [row.id, row]));
  const folderOf = new Map(folders.map((folder) => [folder.id, folder]));
  const out: AppModel[] = [];
  for (const folder of cliFolders(folders, places, byId, listed)) {
    const id = cliAppId(folder.id);
    const row = byId.get(id);
    // ADR-0032: a CLI App says so ("Claude Code CLI · Work").
    const app = cliAppWord(folder.provider);
    const defaultName = appName(app, folder.label);
    out.push({
      id,
      kind: "cli",
      tool: folder.provider,
      appId: appOfTool(folder.provider).appId,
      app,
      profile: folder.label,
      name: row?.label ?? defaultName,
      defaultName,
      named: Boolean(row?.label),
      railKey: cliAppKey(folder.provider, folder.id, folder.label),
      folder,
      accountId: row?.account_id ?? null,
      place: null,
      slot: row?.colour_slot ?? folder.app_colour_slot ?? appSlot(id),
    });
  }
  for (const place of places) {
    const row = byId.get(place.id);
    const app = knownAppWord(place);
    const profile = desktopProfile(place.instance, place.profile);
    const defaultName = appName(app, profile);
    const folderId = row?.folder_id ?? place.account_id;
    out.push({
      id: place.id,
      kind: "desktop",
      tool: row?.tool || null,
      appId: place.app_id,
      app,
      profile,
      name: row?.label ?? defaultName,
      defaultName,
      named: Boolean(row?.label),
      railKey: knownKey(place),
      folder: folderId ? (folderOf.get(folderId) ?? null) : null,
      accountId: row?.account_id ?? null,
      place,
      slot: row?.colour_slot ?? place.colour_slot ?? appSlot(place.id),
    });
  }
  return out;
}

/** The App the rail keys `railKey`, when it's one. */
export function appByRailKey(apps: readonly AppModel[], railKey: string): AppModel | null {
  return apps.find((app) => app.railKey === railKey) ?? null;
}

/** The App id of the rail's App group `railKey`: a CLI App's folder
 * (`cliFolder`, the group's sessions' folder), else the remembered
 * desktop app or profile keyed so. `null` when it isn't one. */
export function appIdOfRailKey(railKey: string, cliFolder: string | null, places: readonly KnownPlaceInfo[] | null): string | null {
  if (railKey.startsWith("cli|")) return cliFolder ? cliAppId(cliFolder) : null;
  return places?.find((place) => knownKey(place) === railKey)?.id ?? null;
}

/** The stored Apps, one read for every screen (walk A4), again whenever
 * one is renamed. */
const STORED_APPS: SharedRead<StoredAppInfo[]> = sharedRead(async () => {
  try {
    return await appsList();
  } catch {
    return STORED_APPS.peek() ?? [];
  }
});
let appsListening = false;
function listenApps(): void {
  if (appsListening) return;
  appsListening = true;
  window.addEventListener(APPS_CHANGED, () => void STORED_APPS.refresh());
  whenForgotten(() => (appsListening = false));
}

/** The stored Apps (`null` until read). */
function useStoredApps(): StoredAppInfo[] | null {
  useEffect(listenApps, []);
  return useSharedRead(STORED_APPS, { maxAgeMs: 5_000 }) ?? null;
}

const NO_FOLDERS: readonly AccountInfo[] = [];

/** Every App, joined (`null` until the places are read). */
export function useApps(folders: readonly AccountInfo[] = NO_FOLDERS): AppModel[] | null {
  const stored = useStoredApps();
  const places = useKnownPlaces();
  return useMemo(
    () => (places === null ? null : appsOf({ stored: stored ?? [], places, folders, listed: hasLiveBackend() })),
    [stored, places, folders],
  );
}

/** Opens App `id`'s page. */
export const OPEN_APP = "aij:open-app";

export function openApp(id: string): void {
  window.dispatchEvent(new CustomEvent<{ id: string }>(OPEN_APP, { detail: { id } }));
}

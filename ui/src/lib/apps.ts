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
import { useCallback, useEffect, useMemo } from "react";
import { sharedRead, useSharedRead, whenForgotten, type SharedRead } from "./shared-read";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { appName, cliAppId, cliAppKey, cliAppWord, desktopProfile } from "./app-name";
import { appOfTool } from "./app-of";
import { APPS_CHANGED, appsList, type StoredAppInfo } from "./ipc/apps";
import type { KnownPlaceInfo } from "./ipc/places";
import { useKnownPlaces } from "./places";
import { knownAppWord, knownKey, knownPlaceMark } from "./rail-apps";
import type { RailPlace } from "./rail-items";

import { hasLiveBackend } from "./bridge";
import { appSlot } from "./marks";
import { knownFolderTool } from "./palette";

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
  const profiles = heldFolders(places);
  return folders.filter((folder) => isCliFolder(folder.id, profiles, stored));
}

/** The folders a desktop profile's place holds (its sessions' config folder). */
function heldFolders(places: readonly KnownPlaceInfo[]): ReadonlySet<string> {
  return new Set(places.flatMap((place) => (place.account_id ? [place.account_id] : [])));
}

/** `cliFolders`' rule for one folder, with the backend's list. */
function isCliFolder(id: string, held: ReadonlySet<string>, stored: ReadonlyMap<string, StoredAppInfo>): boolean {
  return !held.has(id) || stored.has(cliAppId(id));
}

/** A CLI App's words: its tool's CLI word and folder, the user's name first. */
function cliWords(tool: string, label: string | null, row: StoredAppInfo | undefined): { app: string; defaultName: string; name: string } {
  // ADR-0032: a CLI App says so ("Claude Code CLI · Work").
  const app = cliAppWord(tool);
  const defaultName = appName(app, label);
  return { app, defaultName, name: row?.label ?? defaultName };
}

/** A desktop App's words: its app and profile, the user's name first. */
function desktopWords(place: KnownPlaceInfo, row: StoredAppInfo | undefined): { app: string; profile: string | null; defaultName: string; name: string } {
  const app = knownAppWord(place);
  const profile = desktopProfile(place.instance, place.profile);
  const defaultName = appName(app, profile);
  return { app, profile, defaultName, name: row?.label ?? defaultName };
}

/** A launch or switch target: a folder, its tool and its label. */
export interface FolderTarget {
  folder: string;
  tool: string | null | undefined;
  label: string;
}

/** D-62 (the 0.4.6 RC walks A8, K3): the App a launch or switch target is
 * reached through, named as the Apps list names it, for a surface that
 * holds the target's folder but not the folders list: the folder's CLI
 * App ("Claude Code CLI · work", or your name for it) while it is one
 * (`cliFolders`' rule); a folder a Claude app profile holds with no CLI App
 * on it is that profile's ("Claude app · Work Max": a switch continues in
 * it), never "Claude Code CLI · Work Max", an App that isn't one. Without
 * the backend's list (the preview, or not read yet) every folder is its
 * CLI App, as before. A target with no tool is its label. */
export function targetApp(target: FolderTarget, { stored, places, listed }: { stored: readonly StoredAppInfo[]; places: readonly KnownPlaceInfo[]; listed: boolean }): TargetApp {
  const byId = new Map(stored.map((row) => [row.id, row]));
  const place = listed ? places.find((one) => one.account_id === target.folder) : undefined;
  if (place && !isCliFolder(target.folder, heldFolders(places), byId)) return { kind: "desktop", name: desktopWords(place, byId.get(place.id)).name, mark: knownPlaceMark(place) };
  if (!target.tool) return { kind: "cli", name: target.label, mark: null };
  return { kind: "cli", name: cliWords(target.tool, target.label, byId.get(cliAppId(target.folder))).name, mark: null };
}

/** D-62, D-84 (the 0.4.6 RC walks A8, K4): the launch settings a CLI
 * start may be offered with, a CLI App's (`cliFolders`' rule). Every
 * folder has launch settings, a Claude app profile's too, but that folder
 * is the profile's until a terminal session ran there or the user named
 * its CLI App: offered anyway, a start read "Claude Code CLI · Work Max",
 * an App that isn't one, and ran the CLI on the profile's folder. Without
 * the backend's list, all of them, as before. */
export function cliRecipes<T extends { account_id: string }>(recipes: readonly T[], { stored, places, listed }: { stored: readonly StoredAppInfo[]; places: readonly KnownPlaceInfo[]; listed: boolean }): readonly T[] {
  if (!listed) return recipes;
  const byId = new Map(stored.map((row) => [row.id, row]));
  const held = heldFolders(places);
  const offered = recipes.filter((recipe) => isCliFolder(recipe.account_id, held, byId));
  return offered.length === recipes.length ? recipes : offered;
}

/** A target's App: its kind, its name, and a desktop App's mark (a CLI
 * App's is its tool's on the folder, `CliAppMark`). */
export interface TargetApp {
  kind: "cli" | "desktop";
  name: string;
  mark: RailPlace | null;
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
    const { app, defaultName, name } = cliWords(folder.provider, folder.label, row);
    out.push({
      id,
      kind: "cli",
      tool: folder.provider,
      appId: appOfTool(folder.provider).appId,
      app,
      profile: folder.label,
      name,
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
    const { app, profile, defaultName, name } = desktopWords(place, row);
    const folderId = row?.folder_id ?? place.account_id;
    out.push({
      id: place.id,
      kind: "desktop",
      tool: row?.tool || null,
      appId: place.app_id,
      app,
      profile,
      name,
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

/** The Apps as read now, for `targetApp` and `cliRecipes`: the backend's
 * list only once both the stored Apps and the places are read. */
function useAppsRead(): { stored: readonly StoredAppInfo[]; places: readonly KnownPlaceInfo[]; listed: boolean } {
  const stored = useStoredApps();
  const places = useKnownPlaces();
  return useMemo(() => ({ stored: stored ?? [], places: places ?? [], listed: stored !== null && places !== null && hasLiveBackend() }), [stored, places]);
}

/** `targetApp` over the Apps as read now (every folder is its CLI App
 * until they are). */
export function useTargetApp(): (target: FolderTarget) => TargetApp {
  const read = useAppsRead();
  return useCallback((target: FolderTarget) => targetApp(target, read), [read]);
}

/** A sign-in folder's App by name, for the surfaces that list folders
 * (Settings › Sessions' retention, the cleanup rules, Usage by App): the
 * name `targetApp` gives, so a folder only a Claude app profile holds
 * reads "Claude app · Work Max", never "Claude Code CLI · Work Max", an
 * App that isn't one (the 0.5 walk's F-05). A folder whose tool isn't
 * known is its label. */
export function useFolderAppName(): (folderId: string | null | undefined, label: string) => string {
  const appOf = useTargetApp();
  return useCallback((folderId: string | null | undefined, label: string) => (folderId ? appOf({ folder: folderId, tool: knownFolderTool(folderId), label }).name : label), [appOf]);
}

/** `cliRecipes` over the Apps as read now (`null` stays `null`). */
export function useCliRecipes<T extends { account_id: string }>(recipes: readonly T[] | null): readonly T[] | null {
  const read = useAppsRead();
  return useMemo(() => (recipes === null ? null : cliRecipes(recipes, read)), [recipes, read]);
}

/** D-100: the Apps list's CLI Apps (the rail's Inactive apps lists each
 * not open, as Accounts lists them). */
export function useCliApps(folders?: readonly AccountInfo[]): AppModel[] {
  const apps = useApps(folders);
  return useMemo(() => (apps ?? []).filter((app) => app.kind === "cli"), [apps]);
}

/** Opens App `id`'s page. */
export const OPEN_APP = "aij:open-app";

export function openApp(id: string): void {
  window.dispatchEvent(new CustomEvent<{ id: string }>(OPEN_APP, { detail: { id } }));
}

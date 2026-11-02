/**
 * D-62 (AJ-E3204) — the Apps, as their page, Home and the rail's doors
 * see them: one per folder in use (a CLI tool on it: "Claude Code · Work
 * Max") and one per desktop app or profile seen running ("Claude app ·
 * b-work"). The stored rows give the user's name and the Account; the
 * places and folders give the rest, so an App the backend hasn't stored
 * yet (and every App in the preview) is still one.
 *
 * Each App knows its rail key, so its page lists the very sessions the
 * rail puts under it (lib/rail-apps.ts decides that, once).
 */
import { useEffect, useMemo, useState } from "react";
import type { AccountInfo } from "./bridge";
import { appName, cliAppKey } from "./app-name";
import { appOfTool } from "./app-of";
import { APPS_CHANGED, appsList, type StoredAppInfo } from "./ipc/apps";
import type { KnownPlaceInfo } from "./ipc/places";
import { useKnownPlaces } from "./places";
import { knownAppWord, knownKey } from "./rail-apps";
import { toolSpec } from "./tools";

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
}

/** The Apps: CLI ones in the folders' order, then the desktop ones as
 * the places list them. */
export function appsOf({
  stored,
  places,
  folders,
}: {
  stored: readonly StoredAppInfo[];
  places: readonly KnownPlaceInfo[];
  folders: readonly AccountInfo[];
}): AppModel[] {
  const byId = new Map(stored.map((row) => [row.id, row]));
  const folderOf = new Map(folders.map((folder) => [folder.id, folder]));
  const out: AppModel[] = [];
  for (const folder of folders) {
    const id = `cli:${folder.id}`;
    const row = byId.get(id);
    const app = toolSpec(folder.provider).display_name;
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
    });
  }
  for (const place of places) {
    const row = byId.get(place.id);
    const app = knownAppWord(place);
    const defaultName = appName(app, place.instance);
    const folderId = row?.folder_id ?? place.account_id;
    out.push({
      id: place.id,
      kind: "desktop",
      tool: row?.tool || null,
      appId: place.app_id,
      app,
      profile: place.instance,
      name: row?.label ?? defaultName,
      defaultName,
      named: Boolean(row?.label),
      railKey: knownKey(place),
      folder: folderId ? (folderOf.get(folderId) ?? null) : null,
      accountId: row?.account_id ?? null,
      place,
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
  if (railKey.startsWith("cli|")) return cliFolder ? `cli:${cliFolder}` : null;
  return places?.find((place) => knownKey(place) === railKey)?.id ?? null;
}

/** The stored Apps (`null` until read), again whenever one is renamed. */
function useStoredApps(): StoredAppInfo[] | null {
  const [rows, setRows] = useState<StoredAppInfo[] | null>(null);
  useEffect(() => {
    let live = true;
    const load = () =>
      void appsList().then(
        (next) => live && setRows(next),
        () => live && setRows((was) => was ?? []),
      );
    load();
    window.addEventListener(APPS_CHANGED, load);
    return () => {
      live = false;
      window.removeEventListener(APPS_CHANGED, load);
    };
  }, []);
  return rows;
}

/** Every App, joined (`null` until the places are read). */
export function useApps(folders: readonly AccountInfo[]): AppModel[] | null {
  const stored = useStoredApps();
  const places = useKnownPlaces();
  return useMemo(
    () => (places === null ? null : appsOf({ stored: stored ?? [], places, folders })),
    [stored, places, folders],
  );
}

/** Opens App `id`'s page. */
export const OPEN_APP = "aij:open-app";

export function openApp(id: string): void {
  window.dispatchEvent(new CustomEvent<{ id: string }>(OPEN_APP, { detail: { id } }));
}

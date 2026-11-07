/**
 * D-62 (AJ-E3205) — one Account's sessions and Apps. Its sessions are
 * those on any of its folders (one plan, however many apps use it); its
 * Apps are those whose folder signed in to it. Pure, so it's unit-tested.
 */
import { appByFolder } from "./rail-apps";
import type { AppModel } from "./apps";
import { appItems, openHere } from "./app-items";
import type { RailItem } from "./rail-items";

/** The Account's sessions (archived ones left out): open first, then the
 * most recent. */
export function accountItems(items: readonly RailItem[], folderIds: ReadonlySet<string>): RailItem[] {
  return items
    .filter((item) => !item.archived && item.accountId !== null && folderIds.has(item.accountId))
    .sort((a, b) => Number(openHere(b)) - Number(openHere(a)) || b.lastMs - a.lastMs);
}

export interface AccountApp {
  app: AppModel;
  /** Its sessions on this Account, and how many are open now. */
  sessions: number;
  open: number;
  /** Of the open ones, the ones working now. */
  working: number;
  /** Its newest session's activity, else when it was last seen (0: never). */
  lastMs: number;
}

/** The Apps on Account `accountId` (its folders `folderIds`), the busy
 * ones first, then the most recently used; an App with no session is
 * quiet and goes last. */
export function accountApps(
  apps: readonly AppModel[],
  accountId: string,
  folderIds: ReadonlySet<string>,
  items: readonly RailItem[],
  byConversation: ReadonlyMap<string, string>,
): AccountApp[] {
  return appRows(
    apps.filter((app) => app.accountId === accountId || (app.folder !== null && folderIds.has(app.folder.id))),
    items,
    byConversation,
  );
}

/** AJ-E3104: a tool's Apps, as an Account's are listed. */
export function toolApps(apps: readonly AppModel[], tool: string, items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): AccountApp[] {
  return appRows(
    apps.filter((app) => app.tool === tool),
    items,
    byConversation,
  );
}

/** AJ-E3104: every App on this Mac (an App is this Mac's: a CLI folder or
 * a desktop app's profile here), busy ones first. */
export function macApps(apps: readonly AppModel[], items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): AccountApp[] {
  return appRows(apps, items, byConversation);
}

/** AJ-E3104: the Apps `ids` names (a project's, from the relation
 * counts), else those with a session in `items`, as an Account's are
 * listed. */
export function appsAmong(apps: readonly AppModel[], ids: readonly string[] | undefined, items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): AccountApp[] {
  if (ids) {
    const named = new Set(ids);
    return appRows(
      apps.filter((app) => named.has(app.id)),
      items,
      byConversation,
    );
  }
  return appRows(apps, items, byConversation).filter((row) => row.sessions > 0);
}

/** Each App with its sessions in `items`: busy ones first. */
function appRows(apps: readonly AppModel[], items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): AccountApp[] {
  // The same attribution as the rail's (a closed profile's sessions are its own).
  const byFolder = appByFolder(apps.flatMap((app) => (app.place ? [app.place] : [])));
  return apps
    .map((app) => {
      const mine = appItems(items, app.railKey, byConversation, byFolder);
      return {
        app,
        sessions: mine.length,
        open: mine.filter(openHere).length,
        working: mine.filter((item) => openHere(item) && item.state === "working" && item.need === null).length,
        lastMs: Math.max(mine[0]?.lastMs ?? 0, app.place?.seen_ms ?? 0),
      };
    })
    .sort((a, b) => Number(b.sessions > 0) - Number(a.sessions > 0) || b.open - a.open || b.lastMs - a.lastMs || a.app.name.localeCompare(b.app.name));
}

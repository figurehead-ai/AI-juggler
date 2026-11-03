/**
 * D-62 (AJ-E3205) — one Account's sessions and Apps. Its sessions are
 * those on any of its folders (one plan, however many apps use it); its
 * Apps are those whose folder signed in to it. Pure, so it's unit-tested.
 */
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

/** Each App with its sessions in `items`: busy ones first. */
function appRows(apps: readonly AppModel[], items: readonly RailItem[], byConversation: ReadonlyMap<string, string>): AccountApp[] {
  return apps
    .map((app) => {
      const mine = appItems(items, app.railKey, byConversation);
      return {
        app,
        sessions: mine.length,
        open: mine.filter(openHere).length,
        lastMs: Math.max(mine[0]?.lastMs ?? 0, app.place?.seen_ms ?? 0),
      };
    })
    .sort((a, b) => Number(b.sessions > 0) - Number(a.sessions > 0) || b.open - a.open || b.lastMs - a.lastMs || a.app.name.localeCompare(b.app.name));
}

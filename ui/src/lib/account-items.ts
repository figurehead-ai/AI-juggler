/**
 * D-62 (AJ-E3205) — one Account's sessions and Apps. Its sessions are
 * those on any of its folders (one plan, however many apps use it); its
 * Apps are those whose folder signed in to it. Pure, so it's unit-tested.
 */
import { appByFolder } from "./rail-apps";
import { hasLiveBackend } from "./bridge";
import { cliAppId } from "./app-name";
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

/** What [`appsOfAccount`] needs of an Account (`SubscriptionInfo`). */
export interface AccountOfApps {
  account_id?: string | null;
  /** Its Apps' ids, as the backend lists them (`SubscriptionInfo.apps`). */
  apps?: readonly string[];
}

/** rc22 (A3): an Account's Apps, the one rule every screen lists and
 * counts them by (its card on Accounts, "N apps share this account", its
 * page, Usage's and Home's runway): of the Apps (`appsOf`, against the
 * backend's list), those the backend names for it (`SubscriptionInfo.apps`,
 * by `apps::users_apps`); where that wasn't read (the preview, or no
 * Account yet), those on one of its folders (`folderIds`) or read as it. */
function appsOfAccount(apps: readonly AppModel[], account: AccountOfApps | null, folderIds: ReadonlySet<string>): AppModel[] {
  if (account?.apps) {
    const ids = new Set(account.apps);
    return apps.filter((app) => ids.has(app.id));
  }
  const id = account?.account_id ?? null;
  return apps.filter((app) => (id !== null && app.accountId === id) || (app.folder !== null && folderIds.has(app.folder.id)) || (app.place?.account_id != null && folderIds.has(app.place.account_id)));
}

/** An Account's Apps as known now ("instant and paged"): `ids`, the
 * backend's list for it (`SubscriptionInfo.apps`), there with the
 * Accounts; `apps`, their models once the Apps are read (`null` till
 * then). `null` while the backend's list isn't read: a placeholder, never
 * its folders (the rc23 walk's flash). The preview has no list: its
 * folders' rule. */
export interface AccountAppsNow {
  ids: readonly string[];
  apps: AppModel[] | null;
}

export function accountAppsNow(apps: readonly AppModel[] | null | undefined, account: AccountOfApps | null, folderIds: ReadonlySet<string>): AccountAppsNow | null {
  if (hasLiveBackend()) {
    if (!account?.apps) return null;
    const ids = new Set(account.apps);
    return { ids: account.apps, apps: apps ? apps.filter((app) => ids.has(app.id)) : null };
  }
  if (!apps) return { ids: [...folderIds].map(cliAppId), apps: null };
  const mine = appsOfAccount(apps, account, folderIds);
  return { ids: mine.map((app) => app.id), apps: mine };
}

/** Whether some of `now`'s Apps wait for their names (a desktop App's
 * model isn't read yet; a CLI App is named by its folder). */
export function appsAwaited(now: AccountAppsNow | null): boolean {
  if (now === null) return true;
  return now.ids.some((id) => !id.startsWith("cli:") && !(now.apps ?? []).some((app) => app.id === id));
}

/** An Account's Apps by name, as a line or chips say them: each one's id,
 * whole name and mark (a CLI App's from its folder, `cliName`, so it's
 * named before the Apps are read); `null` while any is still read. */
export function namedAccountApps(
  apps: readonly AppModel[] | null | undefined,
  account: AccountOfApps | null,
  folderIds: ReadonlySet<string>,
  cliName: (folderId: string) => string,
): { id: string; name: string; mark: string | number }[] | null {
  const known = accountAppsNow(apps, account, folderIds);
  if (known === null || appsAwaited(known)) return null;
  return known.ids.map((id) => {
    const app = known.apps?.find((one) => one.id === id);
    return app ? { id, name: app.name, mark: app.slot } : { id, name: cliName(id.slice("cli:".length)), mark: id };
  });
}

/** The Apps on an Account ([`appsOfAccount`]), the busy ones first, then
 * the most recently used; an App with no session is quiet and goes last. */
export function accountApps(
  apps: readonly AppModel[],
  account: AccountOfApps | null,
  folderIds: ReadonlySet<string>,
  items: readonly RailItem[],
  byConversation: ReadonlyMap<string, string>,
): AccountApp[] {
  return appRows(appsOfAccount(apps, account, folderIds), items, byConversation);
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

/**
 * D-118 (AJ-E3702 R11) — what a feed item opens: its session, its
 * conversation in another app, its pool (a task, a thread), its Account,
 * This Mac or its project; every item has a door. Its App forward (↗) is
 * the rail row's go-to, where the session runs (`HomeFeedCard`).
 */
import type { ActionHost } from "./action-host";
import type { FeedItemInfo } from "./ipc/feed";
import type { RailItem } from "./rail-items";

export interface FeedDoors {
  host: ActionHost;
  onOpenSession: (id: string) => void;
  onOpenThisMac?: () => void;
}

const ACCOUNT_KINDS = new Set(["quota", "credits", "reset", "renewal"]);

/** Opens `item`'s thing; false when it has none. */
export function openFeedItem(item: FeedItemInfo, doors: FeedDoors): boolean {
  const { target } = item;
  const { host } = doors;
  if (target.session_id) {
    doors.onOpenSession(target.session_id);
    return true;
  }
  if (target.conversation_id && target.account_id && host.openConversationById) {
    host.openConversationById(target.account_id, target.conversation_id, target.project_id);
    return true;
  }
  if (target.pool_id && host.openPool) {
    host.openPool(target.pool_id);
    return true;
  }
  if (ACCOUNT_KINDS.has(item.kind) && target.account_id && host.openAccountById) {
    host.openAccountById(target.account_id);
    return true;
  }
  if (target.page === "this_mac" && doors.onOpenThisMac) {
    doors.onOpenThisMac();
    return true;
  }
  if (target.project_id && host.openProjectById) {
    host.openProjectById(target.project_id);
    return true;
  }
  return false;
}

/** The rail's row for an item's session or conversation, while it is
 * open somewhere (its ↗ brings that App forward). */
export function railRowOf(item: FeedItemInfo, rows: readonly RailItem[]): RailItem | null {
  const { target } = item;
  return (
    rows.find((row) => row.live && ((target.session_id !== null && row.session?.id === target.session_id) || (target.conversation_id !== null && row.conversation?.id === target.conversation_id))) ?? null
  );
}

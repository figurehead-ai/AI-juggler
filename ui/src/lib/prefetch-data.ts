/**
 * The main screens' first pages, read in idle time once the first window
 * has painted (lib/lazy's queue), so opening History or Usage shows its
 * rows at once (0.1.5 review: they opened blank for seconds). Each key is
 * the one its screen reads with `usePagedList`, so the screen finds it
 * remembered and reads it again behind it.
 */
import { hasLiveBackend } from "./bridge";
import { prefetchData } from "./lazy";
import { prefetchPaged, refreshPaged } from "./paged";
import { costRowsKey, costRowsPage } from "./ipc/costs";
import { sessionsPage, sessionsQuery } from "./ipc/sessions";

/** History's key for a tab and a search (screens/History). */
export function historyKey(list: "ended" | "archived", searched: string, version = ""): string {
  return `history:${list}:${searched}:${version}`;
}

/** The paged session lists' keys, by their start: History's and each
 * page's (a project's, an App's, an account's, a tool's, this Mac's). */
const SESSION_LISTS = ["history:", "project-sessions:", "project-open:", "app-sessions:", "account-sessions:", "tool-sessions:", "mac-sessions:"] as const;

/** Reads every session list again (0.4.1, the rc12 walk: after an
 * archive History's lists and counts stayed as they were until another
 * tab was opened). */
export function refreshSessionLists(): void {
  for (const prefix of SESSION_LISTS) refreshPaged(prefix);
}

/** Queues the main screens' first pages (live backend only). */
export function prefetchMainScreens(): void {
  if (!hasLiveBackend()) return;
  prefetchData(() => prefetchPaged(historyKey("ended", ""), (page) => sessionsPage(sessionsQuery({ which: "ended" }), page)));
  prefetchData(() => prefetchPaged(costRowsKey(null), (page) => costRowsPage(null, page)));
  // AJ-E2404: Processes' Needs attention (its own module, loaded with it).
  prefetchData(() => import("../screens/processes/useProcessRows").then((rows) => rows.prefetchProcessRows()));
}

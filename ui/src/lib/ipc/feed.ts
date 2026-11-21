/**
 * D-118 (AJ-E3702) — Home's activity feed over IPC: one page at a time,
 * newest first, narrowed by the window's filters (`feed_page`, the read
 * model in `features/feed`). In the preview, a stand-in keeps the same
 * rules over the preview's items (`preview/feed.ts`; the seam
 * `__AIJ_FEED__` seeds them), so galleries and the paging test page it
 * exactly as the backend does: never all at once.
 */
import { call, liveBackend as hasBackend } from "./transport";
import { previewHooks } from "../preview-hooks";
import type { Page, PageRequest } from "../paged";
import type { FeedFiltersInfo } from "../../types/generated/FeedFiltersInfo";
import type { FeedItemInfo } from "../../types/generated/FeedItemInfo";

export type { FeedFiltersInfo, FeedItemInfo };

/** Items in a page (D-118: 30). */
export const FEED_PAGE = 30;

/** One page of the feed past `request`'s cursor. */
export async function feedPage(request: PageRequest, filters: FeedFiltersInfo): Promise<Page<FeedItemInfo>> {
  if (!hasBackend) return previewHooks().feed.feedPagePreview(request, filters);
  return call("feed_page", { request, filters });
}

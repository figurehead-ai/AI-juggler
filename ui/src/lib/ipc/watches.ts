/**
 * AJ-E3814 — watches, as Bot's confirm card and the Bot page call them
 * (AJ-E3815): what a target can be watched for (only what its Apps
 * report, D-127), a draft in plain words (nothing written), the watch
 * created on the user's click, and the newest few for "Watches Bot set".
 * One copy: each is the Watches view's own call (`watch-view.ts`), with
 * its one stand-in (`window.__AIJ_WATCH_VIEW__`) for the preview and tests.
 */
import { watchCatalogue, watchCreate, watchDraftWords, watchesPage, type CatalogueEntry, type WatchDraftInfo } from "./watch-view";
import type { WatchPreviewInfo } from "../../types/generated/WatchPreviewInfo";

export type { CatalogueEntry, WatchDraftInfo, WatchPreviewInfo };
export type { WatchViewSeam as WatchesSeam } from "./watch-view";

/** What `target` can be watched for. */
export const watchesCatalogue = watchCatalogue;

/** A draft in plain words; nothing is written. */
export async function watchesPreview(draft: WatchDraftInfo): Promise<Pick<WatchPreviewInfo, "words" | "problem">> {
  const words = await watchDraftWords(draft);
  return { words: words.words, problem: words.problem };
}

/** Creates the watch the user confirmed (their click, never Bot's). */
export async function watchesCreate(draft: WatchDraftInfo): Promise<{ words: string }> {
  const made = await watchCreate(draft);
  return { words: made.words };
}

/** A watch as the Bot page's side lists it: its id and its words. */
export interface WatchLine {
  id: string;
  words: string;
}

/** The newest watches, in plain words (one page). */
export async function watchesNewest(limit = 5): Promise<WatchLine[]> {
  const page = await watchesPage("all", { cursor: null, limit });
  return page.items.map((info) => ({ id: info.watch.id, words: info.words }));
}

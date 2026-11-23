/**
 * AJ-E3824 (D-144): Search's records — the sessions, tasks, their pools
 * and usage AI Juggler's records hold for what's typed, through Bot's one
 * look-up (`bot::search::search_records`). Nothing is written.
 *
 * The preview and tests set `window.__AIJ_SEARCH__`: what each text
 * finds. With none, nothing is found.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { SearchRecordsInfo } from "../../types/generated/SearchRecordsInfo";
import type { SearchPoolInfo } from "../../types/generated/SearchPoolInfo";

export type { SearchPoolInfo, SearchRecordsInfo };

declare global {
  interface Window {
    __AIJ_SEARCH__?: Record<string, SearchRecordsInfo>;
  }
}

export const NOTHING_FOUND: SearchRecordsInfo = { sessions: [], tasks: [], pools: [], usage: [] };

/** What the records hold for `text`. */
export async function searchRecords(text: string): Promise<SearchRecordsInfo> {
  if (!hasLiveBackend()) return window.__AIJ_SEARCH__?.[text.trim().toLowerCase()] ?? NOTHING_FOUND;
  return call("search_records", { text });
}

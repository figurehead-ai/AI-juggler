/**
 * AJ-E1552: a paged list's first read as a `LoadState` for `LoadingState`:
 * loading until its first page, "Still loading…" after 3 s (the list's
 * read has its own deadline, so none here), then its rows, its own empty
 * state, or an error with Retry (`list.refresh`). Rows read before a
 * failed re-read stay shown.
 */
import { useLoadDeadline, type LoadState } from "./loading";
import type { PagedList } from "./paged";

export function usePagedState(list: Pick<PagedList<unknown>, "loading" | "items" | "error">, what: string): { state: LoadState; slow: boolean } {
  const { slow } = useLoadDeadline(list.loading, what, { timeoutMs: Number.POSITIVE_INFINITY });
  const failed = list.error !== null && list.error !== undefined;
  const state: LoadState = list.loading ? "loading" : list.items.length > 0 ? "ready" : failed ? "error" : "empty";
  return { state, slow };
}

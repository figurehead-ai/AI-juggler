/**
 * The cost table, a page at a time (0.1.5 review: Usage drew every
 * session × model × day row at once). Rows are `cost_rollups`' rows
 * (the same rollup); the totals stay with `costRollups`.
 */
import { invoke } from "./transport";
import { hasLiveBackend, type CostRollupRow } from "../bridge";
import { pageOf, type Page, type PageRequest } from "../paged";

/** One page of the cost table, newest day first; `account` narrows it to
 * one account. Read it with `usePagedList(costRowsKey(account), …)`. */
export async function costRowsPage(account: string | null, page: PageRequest): Promise<Page<CostRollupRow>> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_COST_ROLLUPS__?: { rows: CostRollupRow[] } }).__AIJ_COST_ROLLUPS__;
    return pageOf(seeded?.rows ?? [], page);
  }
  return invoke<Page<CostRollupRow>>("cost_rows_page", { account, page });
}

/** The paged list's key for one account's (or every account's) table. */
export function costRowsKey(account: string | null): string {
  return `costs:${account ?? "ALL"}`;
}

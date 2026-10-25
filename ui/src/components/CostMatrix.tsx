/**
 * AJ-E609 — Usage's "Prices and details": what each session used of each
 * model, by day, at list prices, with the permanent USD label
 * ("Estimated — list pricing, not your subscription fee"), the unpriced
 * `—` law, the stale-pricing warning, credits alongside (never
 * converted), and the dual subscription view.
 *
 * 0.1.5: the rows are read a page at a time (`costRowsPage`, drawn by
 * `PagedList` only as they scroll into view); the totals and the
 * subscription view stay with `costRollups`. A header row names the
 * columns; each row repeats its column names for screen readers.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { PagedList } from "./PagedList";
import { costCsvExport, costRollups, type CostRollupRow, type CostRollupsData } from "../lib/bridge";
import { costRowsKey, costRowsPage } from "../lib/ipc/costs";
import { usePagedList } from "../lib/paged";

const usdText = (value: number): string =>
  `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

/** Its tier in plain words (never "MEASURED" in capitals). */
function tierWords(confidence: string): string {
  const tier = confidence.toLowerCase();
  if (tier === "measured") return "measured";
  if (tier === "computed") return "worked out";
  return "estimate";
}

/** Session · model · day · tokens · credits · USD. */
const GRID = "grid grid-cols-[minmax(9rem,1.4fr)_minmax(8rem,1fr)_6.5rem_minmax(9rem,1fr)_5rem_minmax(8rem,1fr)] items-center gap-x-4 px-4";

const keyOf = (row: CostRollupRow): string => `${row.key}-${row.model ?? "unknown"}-${row.day ?? ""}`;

function Row({ row }: { row: CostRollupRow }): JSX.Element {
  return (
    <div className={`${GRID} min-h-11 py-2 text-body`} data-testid="cost-row">
      <span className="truncate font-mono text-meta text-ink" title={row.key}>
        <span className="sr-only">Session </span>
        {row.key}
      </span>
      <span className="truncate text-ink">
        <span className="sr-only">Model </span>
        {row.model ?? "Model not recorded"}
      </span>
      <span className="tabular-nums text-muted">
        <span className="sr-only">Day </span>
        {row.day ?? ""}
      </span>
      <span className="text-right tabular-nums text-ink">
        <span className="sr-only">Tokens in and out </span>
        {row.tokens.input.toLocaleString()} / {row.tokens.output.toLocaleString()}
      </span>
      <span className="text-right tabular-nums text-ink">
        <span className="sr-only">Credits </span>
        {row.credits != null ? row.credits.toFixed(2) : ""}
      </span>
      <span className="text-right tabular-nums">
        <span className="sr-only">USD, estimate </span>
        {/* Unpriced renders words, never $0.00 (deep/10 line 74). */}
        {row.usd.usd != null ? (
          <span className="text-ink">{usdText(row.usd.usd)}</span>
        ) : (
          <span className="text-muted" data-testid="cost-unpriced">
            {row.model ? `No price for ${row.model} yet` : "—"}
          </span>
        )}
        <span className="ml-1.5 text-label text-muted">{tierWords(row.confidence)}</span>
      </span>
    </div>
  );
}

export function CostMatrix(): JSX.Element {
  const [data, setData] = useState<CostRollupsData | null>(null);
  const [csvPath, setCsvPath] = useState("");
  const list = usePagedList(costRowsKey(null), (page) => costRowsPage(null, page), { pageSize: 50, keyOf });

  useEffect(() => {
    let alive = true;
    costRollups().then((payload) => {
      if (alive) setData(payload);
    });
    return () => {
      alive = false;
    };
  }, []);

  const fee = data?.subscription.flat_fee_usd;
  const apiValue = data?.subscription.api_equivalent_usd;
  const none = !list.loading && list.items.length === 0;

  return (
    <Card className="flex flex-col gap-4 p-card" data-testid="cost-matrix">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-section font-semibold text-ink">Costs by session, model and day</h3>
        <span className="flex-1" />
        <Button size="sm" variant="secondary" disabled={none} onClick={() => void costCsvExport().then(setCsvPath)}>
          Export CSV
        </Button>
      </div>
      {/* The permanent disclaimer — a label, never a tooltip (deep/10 §6). */}
      <p className="text-meta text-muted">USD is Estimated — list pricing, not your subscription fee.</p>
      {data?.pricing_stale && (
        <p role="status" className="text-meta text-warning">
          Pricing table is over 30 days old — numbers may not match current list rates.
        </p>
      )}
      {/* Every optional field arrives as null (never undefined): `!= null`
          (the 0.0.12 report — Usage went blank on null.toFixed). */}
      {fee != null && apiValue != null && (
        <p className="text-body text-ink">
          ≈{usdText(apiValue)} at API prices · you pay {usdText(fee)} flat
          {data?.subscription.value_ratio != null && ` · ${data.subscription.value_ratio.toFixed(2)}× value`}
        </p>
      )}
      <div className="overflow-x-auto" tabIndex={0} role="region" aria-label="Cost table">
        <div className="min-w-[52rem]">
          <div aria-hidden="true" className={`${GRID} pb-2 text-meta font-medium text-muted`} data-testid="cost-columns">
            <span>Session</span>
            <span>Model</span>
            <span>Day</span>
            <span className="text-right">Tokens (in / out)</span>
            <span className="text-right">Credits</span>
            <span className="text-right">USD (estimate)</span>
          </div>
          <PagedList
            list={list}
            keyOf={keyOf}
            renderRow={(row) => <Row row={row} />}
            rowHeight={44}
            label="Cost per session, model and day"
            empty={
              <p className="rounded-xl border border-hairline px-4 py-5 text-center text-body text-muted" data-testid="cost-empty">
                No usage recorded yet. Costs appear here as your sessions run.
              </p>
            }
            maxHeight={440}
            testId="cost-rows"
          />
          {data?.total_usd != null && !none && (
            <p className={`${GRID} mt-2 border-t border-hairline pt-3 text-body font-semibold text-ink`} data-testid="cost-total">
              <span className="col-span-5">Total (all sessions)</span>
              <span className="text-right tabular-nums">{usdText(data.total_usd)}</span>
            </p>
          )}
        </div>
      </div>
      {csvPath && (
        <p role="status" className="text-meta text-muted">
          CSV written: <span className="font-mono">{csvPath}</span>
        </p>
      )}
    </Card>
  );
}

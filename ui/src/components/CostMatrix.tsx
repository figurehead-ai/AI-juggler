/**
 * AJ-E609 — the Usage cost matrix: per-session model attribution with
 * the permanent USD label ("Estimated — list pricing, not your
 * subscription fee"), the unpriced `—` law, the stale-pricing inline
 * warning, `ZCode` credits alongside (never converted), and the dual
 * subscription view. A11y: real table semantics (`th scope`), a
 * totals row the reader announces, badge text visible without hover.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { costCsvExport, costRollups, type CostRollupsData } from "../lib/bridge";

const usdText = (value: number): string =>
  `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

export function CostMatrix(): JSX.Element {
  const [data, setData] = useState<CostRollupsData | null>(null);
  const [csvPath, setCsvPath] = useState("");

  useEffect(() => {
    let alive = true;
    costRollups().then((payload) => {
      if (alive) setData(payload);
    });
    return () => {
      alive = false;
    };
  }, []);

  if (!data) {
    return (
      <Card className="p-5 text-xs text-muted" data-testid="cost-matrix">
        Loading costs…
      </Card>
    );
  }

  const fee = data.subscription.flat_fee_usd;
  const apiValue = data.subscription.api_equivalent_usd;
  const cell = "border-t border-hairline px-2 py-1.5";
  const numeric = `${cell} text-right tabular-nums`;

  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="cost-matrix">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold">Costs by session, model and day</h3>
        <span className="flex-1" />
        <Button
          size="sm"
          variant="secondary"
          disabled={data.rows.length === 0}
          onClick={() => {
            costCsvExport().then(setCsvPath);
          }}
        >
          Export CSV
        </Button>
      </div>
      {/* The permanent disclaimer — a label, never a tooltip (deep/10 §6). */}
      <p className="text-xs text-muted">
        USD is Estimated — list pricing, not your subscription fee.
      </p>
      {data.pricing_stale && (
        <p role="status" className="text-xs text-warning">
          Pricing table is over 30 days old — numbers may not match current list rates.
        </p>
      )}
      {fee !== undefined && apiValue !== undefined && (
        <p className="text-xs">
          ≈{usdText(apiValue)} at API prices · you pay {usdText(fee)} flat
          {data.subscription.value_ratio !== undefined &&
            ` · ${data.subscription.value_ratio.toFixed(2)}× value`}
        </p>
      )}
      {data.rows.length === 0 ? (
        <p className="rounded-lg border border-hairline px-3 py-4 text-center text-xs text-muted" data-testid="cost-empty">
          No usage recorded yet. Costs appear here as your sessions run.
        </p>
      ) : (
        <div className="overflow-x-auto" tabIndex={0} role="region" aria-label="Cost table">
          <table className="w-full border-collapse text-xs">
            <caption className="sr-only">Cost per session, model, and day</caption>
            <thead>
              <tr className="text-left text-[11px] text-muted">
                <th scope="col" className="px-2 py-1.5 font-medium">Session</th>
                <th scope="col" className="px-2 py-1.5 font-medium">Model</th>
                <th scope="col" className="px-2 py-1.5 font-medium">Day</th>
                <th scope="col" className="px-2 py-1.5 text-right font-medium">Tokens (in/out)</th>
                <th scope="col" className="px-2 py-1.5 text-right font-medium">Credits</th>
                <th scope="col" className="px-2 py-1.5 text-right font-medium">USD (Estimate)</th>
              </tr>
            </thead>
            <tbody>
              {data.rows.map((row) => (
                <tr key={`${row.key}-${row.model ?? "unknown"}-${row.day ?? ""}`}>
                  <td className={`${cell} font-mono text-[11px]`}>{row.key}</td>
                  <td className={cell}>{row.model ?? "unknown model"}</td>
                  <td className={`${cell} tabular-nums`}>{row.day ?? ""}</td>
                  <td className={numeric}>
                    {row.tokens.input.toLocaleString()} / {row.tokens.output.toLocaleString()}
                  </td>
                  <td className={numeric}>
                    {row.credits !== undefined ? row.credits.toFixed(2) : ""}
                  </td>
                  <td className={numeric}>
                    {/* Unpriced renders —, never $0.00 (deep/10 line 74). */}
                    {row.usd.usd !== undefined ? usdText(row.usd.usd) : "—"}
                    <span className="ml-1 text-[10px] uppercase text-muted">{row.confidence}</span>
                  </td>
                </tr>
              ))}
            </tbody>
            {data.total_usd !== undefined && (
              <tfoot>
                <tr className="font-medium">
                  <th scope="row" colSpan={5} className={`${cell} text-left`}>
                    Total (all sessions)
                  </th>
                  <td className={numeric}>{usdText(data.total_usd)}</td>
                </tr>
              </tfoot>
            )}
          </table>
        </div>
      )}
      {csvPath && (
        <p role="status" className="text-xs text-muted">
          CSV written: <span className="font-mono">{csvPath}</span>
        </p>
      )}
    </Card>
  );
}

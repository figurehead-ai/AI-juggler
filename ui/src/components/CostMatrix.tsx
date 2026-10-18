/**
 * AJ-E609 — the Usage cost matrix: per-session model attribution with
 * the permanent USD label ("Estimated — list pricing, not your
 * subscription fee"), the unpriced `—` law, the stale-pricing inline
 * warning, `ZCode` credits alongside (never converted), and the dual
 * subscription view. A11y: real table semantics (`th scope`), a
 * totals row the reader announces, badge text visible without hover.
 */
import { useEffect, useState } from "react";
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

  if (!data) return <Card title="Usage costs">loading…</Card>;

  const fee = data.subscription.flat_fee_usd;
  const apiValue = data.subscription.api_equivalent_usd;

  return (
    <Card title="Usage costs">
      <div className="mb-2 flex justify-end">
        <Button
          onClick={() => {
            costCsvExport().then(setCsvPath);
          }}
        >
          Export CSV
        </Button>
      </div>
      {/* The permanent disclaimer — a label, never a tooltip (deep/10 §6). */}
      <p className="text-xs opacity-80">
        USD is Estimated — list pricing, not your subscription fee.
      </p>
      {data.pricing_stale && (
        <p role="status" className="text-xs text-amber-500">
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
      <table>
        <caption className="sr-only">Cost per session, model, and day</caption>
        <thead>
          <tr>
            <th scope="col">Session</th>
            <th scope="col">Model</th>
            <th scope="col">Day</th>
            <th scope="col">Tokens (in/out)</th>
            <th scope="col">Credits</th>
            <th scope="col">USD (Estimate)</th>
          </tr>
        </thead>
        <tbody>
          {data.rows.map((row) => (
            <tr key={`${row.key}-${row.model ?? "unknown"}-${row.day ?? ""}`}>
              <td>{row.key}</td>
              <td>{row.model ?? "unknown model"}</td>
              <td>{row.day ?? ""}</td>
              <td>
                {row.tokens.input.toLocaleString()} / {row.tokens.output.toLocaleString()}
              </td>
              <td>
                {row.credits !== undefined ? row.credits.toFixed(2) : ""}
              </td>
              <td>
                {/* Unpriced renders —, never $0.00 (deep/10 line 74). */}
                {row.usd.usd !== undefined ? usdText(row.usd.usd) : "—"}
                <span className="ml-1 text-[10px] uppercase opacity-70">
                  {row.confidence}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
        {data.total_usd !== undefined && (
          <tfoot>
            <tr>
              <th scope="row" colSpan={5}>
                Total (all sessions)
              </th>
              <td>{usdText(data.total_usd)}</td>
            </tr>
          </tfoot>
        )}
      </table>
      {csvPath && (
        <p role="status" className="text-xs">
          CSV written: {csvPath}
        </p>
      )}
    </Card>
  );
}

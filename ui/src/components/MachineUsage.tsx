/**
 * AJ-E1515 — what each machine used in the last day: this Mac, then each
 * paired Mac as it last reported (with when). Totals only — a paired
 * Mac's sessions, models and costs in detail stay on its own Usage. With
 * no paired Mac this card isn't shown: Usage below is this Mac's already.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { asOfWords, tokenWords } from "../lib/quotaWindow";
import { fleetUsageByMachine, type MachineUsageInfo } from "../lib/ipc/fleet-launch";

/** "1.2M tokens · ≈ $4.10 · 3 sessions". */
export function machineUsageWords(row: MachineUsageInfo): string {
  if (!row.reported) return "Hasn't reported yet";
  if (row.used_units === 0) return "Nothing used";
  const usd = row.usd_est != null ? ` · ≈ $${row.usd_est.toFixed(2)}` : "";
  return `${tokenWords(row.used_units)} tokens${usd} · ${row.sessions} session${row.sessions === 1 ? "" : "s"}`;
}

export function MachineUsageCard(): JSX.Element | null {
  const [rows, setRows] = useState<MachineUsageInfo[] | null>(null);
  useEffect(() => {
    let alive = true;
    const load = () => void fleetUsageByMachine().then((all) => alive && setRows(all), () => null);
    load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  if (!rows || rows.length < 2) return null;
  const all = rows.reduce((sum, row) => sum + row.used_units, 0);
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="machine-usage">
      <div className="flex items-baseline justify-between gap-2">
        <h4 className="text-sm font-medium">Last 24 hours, by machine</h4>
        <span className="text-label text-muted" data-testid="machine-usage-total">
          {tokenWords(all)} tokens in all
        </span>
      </div>
      <ul className="flex flex-col divide-y divide-hairline/60">
        {rows.map((row) => (
          <li key={row.machine_id ?? "here"} className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5 py-1.5 text-xs" data-testid="machine-usage-row">
            <span className="font-medium text-ink">{row.label}</span>
            <span className="text-ink">
              {machineUsageWords(row)}
              {row.as_of_ms != null && <span className="text-muted"> · {asOfWords(row.as_of_ms)}</span>}
            </span>
          </li>
        ))}
      </ul>
      <p className="text-label text-muted">Each Mac's sessions and costs in detail are on its own Usage.</p>
    </Card>
  );
}

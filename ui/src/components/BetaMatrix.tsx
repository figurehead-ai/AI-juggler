/**
 * AJ-E210 — the Beta validation matrix (Settings → Beta, disclosure
 * only): the same ordered-pair table the switch badges derive from
 * (D-04 — one source, no drift). A REAL table (row headers, scoped
 * columns, status in TEXT) so a user can tell exactly which pairs
 * are proven: validated ✓ only when the real-adapter CI row is green
 * AND a live dogfood cell passed; everything else reads "Beta — not
 * yet validated" with its reason. Never "lossless" (D-11). No
 * interaction: cells are recorded by the maintainer from live runs,
 * never from this surface.
 */
import { useEffect, useState } from "react";
import { Card } from "./primitives";
import { validationMatrix, type ValidationCellData } from "../lib/bridge";

function VerdictCell({ row }: { row: ValidationCellData }) {
  if (row.dogfood === null || row.dogfood === undefined) {
    return <span className="text-muted">pending</span>;
  }
  const pass = row.dogfood.verdict === "pass";
  return (
    <span className={pass ? "text-success" : "text-danger"}>
      {pass ? "pass" : "FAIL"} · {row.dogfood.at} ({row.dogfood.project_shape})
    </span>
  );
}

export function BetaMatrix(): JSX.Element {
  const [rows, setRows] = useState<ValidationCellData[] | null>(null);
  useEffect(() => {
    let alive = true;
    validationMatrix()
      .then((cells) => {
        if (alive) setRows(cells);
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, []);

  if (rows === null) {
    return (
      <Card className="p-4 text-xs text-muted" data-testid="beta-matrix-loading">
        Loading the validation matrix…
      </Card>
    );
  }
  const validated = rows.filter((row) => row.validated).length;
  const openDebt = rows.length - validated;
  return (
    <Card className="flex flex-col gap-3 p-4" data-testid="beta-matrix">
      <div>
        <p className="text-[11px] uppercase tracking-wider text-muted">Beta validation</p>
        <p className="mt-1 text-xs text-muted" data-testid="beta-matrix-summary">
          {validated === 0
            ? `All ${rows.length} provider pairs ship in beta — none validated yet. `
            : `${validated} of ${rows.length} pairs validated · ${openDebt} still open beta debt. `}
          Badges say what carried, never more: a pair graduates only when its CI row is green
          AND one live dogfood cell passed on real binaries.
        </p>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-[11px]" data-testid="beta-matrix-table">
          <thead>
            <tr className="border-b border-hairline text-muted">
              <th scope="col" className="py-1.5 pr-3 font-medium">Pair</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">Mechanism</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">Ceiling</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">CI fixtures</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">Live cell</th>
              <th scope="col" className="py-1.5 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={`${row.from}-${row.to}`}
                className="border-b border-hairline/50 align-top"
                data-testid="beta-matrix-row"
                data-validated={row.validated ? "true" : "false"}
              >
                <th scope="row" className="py-1.5 pr-3 font-mono font-normal text-ink">
                  {row.from} → {row.to}
                </th>
                <td className="py-1.5 pr-3 text-muted">{row.mechanism}</td>
                <td className="py-1.5 pr-3">
                  <span className={row.validated ? "text-success" : "text-warning"}>
                    {row.badge}
                  </span>
                  {row.advertised !== row.researched_ceiling && (
                    <span className="block text-[10px] text-danger">
                      lowered from {row.researched_ceiling} — a live cell failed
                    </span>
                  )}
                </td>
                <td className="py-1.5 pr-3 text-muted">
                  {row.mechanism_green ? "shape green" : "—"}
                  {row.ci_green ? " · real row green" : " · real row not landed"}
                </td>
                <td className="py-1.5 pr-3">
                  <VerdictCell row={row} />
                </td>
                <td className="py-1.5 text-muted">
                  <span className="block">
                    {row.validated ? "Validated ✓" : "Beta — not yet validated"}
                  </span>
                  <span className="block text-[10px] text-muted">{row.status}</span>
                  {row.notes.length > 0 && (
                    <span className="block text-[10px] text-muted">{row.notes.join(" · ")}</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="text-[10px] text-muted">
        Live cells are run by hand on real binaries and recorded with before/after
        hashes; a failing cell lowers that pair's ceiling one level until fixed.
      </p>
    </Card>
  );
}

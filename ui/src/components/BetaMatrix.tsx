/**
 * AJ-E210 — which switches are proven (Accounts, disclosure only): the
 * same ordered-pair table the switch badges derive from (D-04 — one
 * source, no drift). A REAL table (row headers, scoped columns, status
 * in TEXT) so a user can tell exactly which switches are proven:
 * proven ✓ only when the automatic tests pass with the real tools AND
 * it was tried by hand; everything else reads "Beta" with its reason.
 * Never "lossless" (D-11). Plain words (AJ-E1535): no pair, CI row,
 * dogfood cell or ceiling. No interaction: hand checks are recorded by
 * the maintainer from live runs, never from this surface.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { providerDisplayName } from "../lib/providers";
import { useTools } from "../lib/tools";
import { validationMatrix, type ValidationCellData } from "../lib/bridge";

function HandCheckCell({ row }: { row: ValidationCellData }) {
  if (row.dogfood === null || row.dogfood === undefined) {
    return <span className="text-muted">Not yet</span>;
  }
  const pass = row.dogfood.verdict === "pass";
  return <span className={pass ? "text-success" : "text-danger"}>{pass ? "Passed" : "Failed"} {row.dogfood.at}</span>;
}

/** One side of a switch: its tool, or (AJ-E1601) its tool through another
 * service. */
function sideName(tool: string, throughService?: boolean): string {
  return throughService ? `${providerDisplayName(tool)} through another service` : providerDisplayName(tool);
}

/** The automatic tests' state in words. */
function testWords(row: ValidationCellData): string {
  if (row.ci_green) return "Pass with the real tools";
  return row.mechanism_green ? "Pass with stand-ins only" : "Not yet";
}

export function BetaMatrix(): JSX.Element {
  const [cells, setRows] = useState<ValidationCellData[] | null>(null);
  // Only switches between tools AI Juggler has: a researched pair for a
  // tool it can't run yet is nothing the user can do (AJ-E1535).
  const known = new Set(useTools().map((tool) => tool.id));
  const rows = cells?.filter((row) => known.has(row.from_tool ?? row.from) && known.has(row.to_tool ?? row.to)) ?? null;
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
        Loading which switches are proven…
      </Card>
    );
  }
  const proven = rows.filter((row) => row.validated).length;
  return (
    <Card className="flex flex-col gap-3 p-4" data-testid="beta-matrix">
      <div>
        <p className="text-section font-semibold text-ink">Proven switches</p>
        <p className="mt-1 text-xs text-muted" data-testid="beta-matrix-summary">
          {proven === 0
            ? `None of the ${rows.length} switches is proven yet; all are in beta. `
            : `${proven} of ${rows.length} switches proven; ${rows.length - proven} still in beta. `}
          Each says what carries, never more. A switch is proven once its automatic tests pass with the real
          tools and it has been tried by hand.
        </p>
      </div>
      <div className="overflow-x-auto" tabIndex={0} role="region" aria-label="Proven switches">
        <table className="w-full text-left text-label" data-testid="beta-matrix-table">
          <thead>
            <tr className="border-b border-hairline text-muted">
              <th scope="col" className="py-1.5 pr-3 font-medium">Switch</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">What carries</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">Automatic tests</th>
              <th scope="col" className="py-1.5 pr-3 font-medium">Tried by hand</th>
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
                <th scope="row" className="py-1.5 pr-3 font-normal text-ink">
                  {sideName(row.from_tool ?? row.from, row.from_service)} → {sideName(row.to_tool ?? row.to, row.to_service)}
                </th>
                <td className="py-1.5 pr-3">
                  <span className={row.validated ? "text-success" : "text-warning"}>{row.badge}</span>
                  {row.advertised !== row.researched_ceiling && (
                    <span className="block text-label text-danger">one level less — a hand check failed</span>
                  )}
                </td>
                <td className="py-1.5 pr-3 text-muted">{testWords(row)}</td>
                <td className="py-1.5 pr-3">
                  <HandCheckCell row={row} />
                </td>
                <td className="py-1.5 text-muted">
                  <span className="block">{row.validated ? "Proven ✓" : "Beta — not proven yet"}</span>
                  <span className="block text-label text-muted">{row.status}</span>
                  {row.notes.length > 0 && (
                    <span className="block text-label text-muted">{row.notes.join(" · ")}</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="text-label text-muted">
        Hand checks use the real tools and record the files before and after. A failed one lowers what that
        switch promises by one level until it's fixed.
      </p>
    </Card>
  );
}

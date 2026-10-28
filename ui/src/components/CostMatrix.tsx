/**
 * AJ-E609 — Usage's "Prices and details": what your usage would cost at
 * list prices, with the permanent USD label ("Estimated — list pricing,
 * not your subscription fee"), the unpriced law (words, never $0.00),
 * the stale-pricing warning, credits alongside (never converted), and
 * the dual subscription view.
 *
 * 0.1.5: the rows are read a page at a time (`costRowsPage`, drawn by
 * `PagedList` only as they scroll into view); the totals and the
 * subscription view stay with `costRollups`.
 *
 * 0.1.8 (the maintainer: "prices and details is supposed to show I assume
 * prices???"): the money leads (the total at list prices, what you pay,
 * the models with no list price), then one row per session and day, named
 * as the rail names a session (never its id), USD right after the session
 * and model, tokens after, credits only when some row has them. Every
 * cell is one line, so rows keep one height; a session that used several
 * models that day opens their parts on demand. The total sits under the
 * table, under the USD column, always in view; nothing scrolls sideways.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "./primitives";
import { PagedList } from "./PagedList";
import { StatTile } from "./charts/StatTile";
import { exact, short } from "./charts/scale";
import { IconMoney, IconTokens } from "./usage/controls";
import { costCsvExport, costRollups, type CostRollupRow, type CostRollupsData } from "../lib/bridge";
import { costRowsKey, costRowsPage, costSessionName, type CostRow } from "../lib/ipc/costs";
import { usePagedList } from "../lib/paged";

const usdText = (value: number): string =>
  `$${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

/** Its tier in plain words (never "MEASURED" in capitals). */
function tierWords(confidence: string): string {
  const tier = confidence.toLowerCase();
  if (tier === "measured") return "measured";
  if (tier === "computed") return "worked out";
  return "estimated";
}

/** A model's name, or what to say when the usage didn't record one. */
const modelName = (model: string | null): string => model ?? "Model not recorded";

/** A UTC day, short: "Oct 21" this year, "Oct 21, 2025" before. */
function dayWords(day: string): string {
  const at = new Date(`${day}T00:00:00Z`);
  if (Number.isNaN(at.getTime())) return day;
  const sameYear = at.getUTCFullYear() === new Date(Date.now()).getUTCFullYear();
  return at.toLocaleDateString("en-US", { month: "short", day: "numeric", year: sameYear ? undefined : "numeric", timeZone: "UTC" });
}

/** Day · session · model · USD · tokens (· credits). */
const gridOf = (credits: boolean): string =>
  `grid grid-cols-[minmax(0,1fr)_7.5rem] ${credits ? "sm:grid-cols-[5.5rem_minmax(0,2fr)_minmax(0,1.1fr)_7.5rem_9rem_5rem]" : "sm:grid-cols-[5.5rem_minmax(0,2fr)_minmax(0,1.1fr)_7.5rem_9rem]"} items-center gap-x-4 px-4`;

/** A column a narrow window leaves out: the session and its USD stay. */
const WIDE = "hidden sm:block";

const keyOf = (row: CostRow): string => `${row.day}\u001f${row.session}`;

/** USD in a cell: a figure, "≥ figure" when a model has no list price,
 * or "No list price" (never $0.00, deep/10 line 74). */
function Usd({ usd, unpriced, testId }: { usd: number | null; unpriced: string[]; testId?: string }): JSX.Element {
  if (usd == null) {
    return (
      <span className="truncate text-muted" title={`No list price for ${unpriced.join(", ")}`} data-testid="cost-unpriced">
        No list price
      </span>
    );
  }
  const partial = unpriced.length > 0;
  return (
    <span className="truncate text-ink" title={partial ? `At least ${usdText(usd)}: ${unpriced.join(", ")} has no list price, so it isn't counted` : undefined} data-testid={testId}>
      {partial && (
        <>
          <span aria-hidden="true">≥ </span>
          <span className="sr-only">at least </span>
        </>
      )}
      {usdText(usd)}
    </span>
  );
}

function Tokens({ tokens, confidence }: { tokens: CostRow["tokens"]; confidence: string }): JSX.Element {
  return (
    <span className={`${WIDE} truncate text-right tabular-nums text-ink`} title={`${exact(tokens.input, "tokens")} in, ${exact(tokens.output, "tokens")} out (${tierWords(confidence)})`}>
      <span className="sr-only">Tokens in and out </span>
      {short(tokens.input, "tokens")} / {short(tokens.output, "tokens")}
    </span>
  );
}

const unpricedOf = (models: CostRollupRow[]): string[] => models.filter((part) => part.usd.usd == null).map((part) => modelName(part.model));

function Row({ row, credits }: { row: CostRow; credits: boolean }): JSX.Element {
  const [open, setOpen] = useState(false);
  const name = costSessionName(row);
  const [main, ...rest] = row.models;
  const grid = gridOf(credits);
  return (
    <div data-testid="cost-row">
      <div className={`${grid} h-11 text-body`}>
        <span className={`${WIDE} truncate tabular-nums text-muted`} title={`${row.day} (UTC)`}>
          <span className="sr-only">Day </span>
          {dayWords(row.day)}
        </span>
        {/* Reachable by ↑/↓ (PagedList moves focus row to row), even on
            a row with no button. */}
        <span tabIndex={-1} className="truncate rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-accent/60" title={`${name}${row.project ? ` · ${row.project}` : ""}${row.session ? `\nSession ${row.session}` : ""}`} data-testid="cost-session">
          <span className="sr-only">Session </span>
          <span className="text-ink">{name}</span>
          {row.project && <span className="text-muted"> · {row.project}</span>}
        </span>
        {rest.length > 0 ? (
          <button
            type="button"
            onClick={() => setOpen((now) => !now)}
            aria-expanded={open}
            className="hidden min-w-0 items-center gap-1.5 rounded-md sm:flex text-left outline-none hover:text-accent-hi focus-visible:ring-2 focus-visible:ring-accent/60"
            title={row.models.map((part) => modelName(part.model)).join(", ")}
            data-testid="cost-models"
          >
            {/* The fold chevron, as the Project lens's. */}
            <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className={"w-3 shrink-0 text-muted transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}>
              <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="sr-only">Model </span>
            <span className="truncate text-ink">{modelName(main?.model ?? null)}</span>
            <span className="shrink-0 text-meta text-muted">+{rest.length}</span>
          </button>
        ) : (
          <span className={`${WIDE} truncate text-ink`} title={modelName(main?.model ?? null)}>
            <span className="sr-only">Model </span>
            {modelName(main?.model ?? null)}
          </span>
        )}
        <span className="truncate text-right tabular-nums">
          <span className="sr-only">USD, estimate </span>
          <Usd usd={row.usd} unpriced={unpricedOf(row.models)} />
        </span>
        <Tokens tokens={row.tokens} confidence={row.confidence} />
        {credits && (
          <span className={`${WIDE} truncate text-right tabular-nums text-ink`}>
            <span className="sr-only">Credits </span>
            {row.credits != null ? row.credits.toFixed(2) : ""}
          </span>
        )}
      </div>
      {open && (
        <ul aria-label={`Models in ${name}, ${dayWords(row.day)}`} className="pb-2" data-testid="cost-parts">
          {row.models.map((part) => (
            <li key={part.model ?? ""} className={`${grid} h-8 text-meta`}>
              <span />
              <span />
              <span className="truncate pl-[1.125rem] text-ink" title={modelName(part.model)}>
                {modelName(part.model)}
              </span>
              <span className="truncate text-right tabular-nums">
                <Usd usd={part.usd.usd} unpriced={part.usd.usd == null ? [modelName(part.model)] : []} />
              </span>
              <Tokens tokens={part.tokens} confidence={part.confidence} />
              {credits && <span className="text-right tabular-nums text-ink">{part.credits != null ? part.credits.toFixed(2) : ""}</span>}
            </li>
          ))}
        </ul>
      )}
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

  const fee = data?.subscription.flat_fee_usd ?? null;
  const ratio = data?.subscription.value_ratio ?? null;
  const unpriced = (data?.unpriced_models ?? []).map(modelName);
  const none = !list.loading && list.items.length === 0;
  // Credits get a column only when some row has them (ZCode); the
  // totals' rows say so as the page's rows load.
  const credits = list.items.some((row) => row.credits != null) || (data?.rows ?? []).some((row) => row.credits != null);
  const grid = gridOf(credits);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4" data-testid="cost-matrix">
      {/* The money first: the total at list prices, what you pay, and
          what the total leaves out (never priced at $0). */}
      <div className="grid grid-cols-1 gap-grid sm:grid-cols-3" data-testid="cost-summary">
        <StatTile
          label="At API list prices"
          icon={<IconMoney width={15} height={15} />}
          value={data?.total_usd ?? null}
          unit="usd"
          full
          note={unpriced.length > 0 ? "Models with a list price" : "Every session, every model"}
          testId="cost-total-tile"
        />
        <StatTile
          label="You pay"
          icon={<IconMoney width={15} height={15} />}
          value={fee}
          unit="usd"
          full
          suffix={fee != null ? "flat" : undefined}
          note={fee == null ? "Your plan's fee isn't set" : ratio != null ? `API prices come to ${ratio.toFixed(2)}× your fee` : undefined}
          testId="cost-fee-tile"
        />
        <StatTile
          label="No list price"
          icon={<IconTokens width={15} height={15} />}
          value={data ? unpriced.length : null}
          unit="count"
          suffix={unpriced.length === 1 ? "model" : "models"}
          note={unpriced.length > 0 ? <span title={unpriced.join(", ")}>{unpriced.join(", ")}: not counted</span> : "Every model is priced"}
          testId="cost-unpriced-tile"
        />
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        {/* The permanent disclaimer — a label, never a tooltip (deep/10 §6). */}
        <p className="min-w-0 flex-1 text-meta text-muted">USD is Estimated — list pricing, not your subscription fee.</p>
        <Button size="sm" variant="secondary" disabled={none} onClick={() => void costCsvExport().then(setCsvPath)}>
          Export CSV
        </Button>
      </div>
      {data?.pricing_stale && (
        <p role="status" className="text-meta text-warning">
          Pricing table is over 30 days old — numbers may not match current list rates.
        </p>
      )}
      <div className="flex min-h-0 flex-col" role="region" aria-label="Cost table">
        <div aria-hidden="true" className={`${grid} pb-2 text-meta font-medium text-muted`} data-testid="cost-columns">
          <span className={WIDE}>Day</span>
          <span>Session</span>
          <span className={WIDE}>Model</span>
          <span className="text-right">Estimated USD</span>
          <span className={`${WIDE} text-right`}>Tokens (in / out)</span>
          {credits && <span className={`${WIDE} text-right`}>Credits</span>}
        </div>
        <PagedList
          list={list}
          keyOf={keyOf}
          renderRow={(row) => <Row row={row} credits={credits} />}
          rowHeight={45}
          label="Cost per session and day"
          empty={
            <p className="rounded-xl border border-hairline px-4 py-5 text-center text-body text-muted" data-testid="cost-empty">
              No usage recorded yet. Costs appear here as your sessions run.
            </p>
          }
          maxHeight="min(30rem, calc(100vh - 25rem))"
          testId="cost-rows"
        />
        {/* Under the table, under the USD column, always in view. */}
        {!none && data && (
          <p className={`${grid} mt-2 border-t border-hairline pt-3 text-body font-semibold text-ink`} data-testid="cost-total">
            <span className="truncate sm:col-span-3">
              Total, all sessions
              {unpriced.length > 0 && <span className="font-normal text-muted"> · without {unpriced.join(", ")}</span>}
            </span>
            <span className="truncate text-right tabular-nums">
              {data.total_usd != null ? (
                usdText(data.total_usd)
              ) : (
                <span className="font-normal text-muted">No list price</span>
              )}
            </span>
          </p>
        )}
      </div>
      {csvPath && (
        <p role="status" className="text-meta text-muted">
          CSV written: <span className="font-mono">{csvPath}</span>
        </p>
      )}
    </div>
  );
}

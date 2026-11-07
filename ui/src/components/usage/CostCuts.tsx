/**
 * AJ-E2902: cost you can act on — what a finished task costs, what went
 * on sessions that crashed or were probably stuck when they ended (with
 * the sessions a click away), and each Account's cost with its top
 * project and how sure its price is. From the backend's cuts of the same
 * summary as the charts (`usage_history_cuts`): every estimate wears "~",
 * an unknown is words, never $0. A finished task (D-77) is a session AI
 * Juggler ran that ended on its own as finished.
 */
import { useMemo, useState, type JSX } from "react";
import { useActionHost } from "../ItemMenu";
import { usageHistoryCuts, type UsageAccountCostInfo, type UsageCostCutsInfo, type UsageCutInfo, type UsageQueryInput } from "../../lib/ipc/usageHistory";
import { useCached } from "../../lib/usage/cached";
import { cutRange, type RangeId } from "../../lib/usage/range";
import { useMinute } from "./useUsage";
import { Section } from "../charts/ChartCard";
import { StatTile } from "../charts/StatTile";
import { exact, short, usdWords } from "../charts/scale";
import { IconMoney, IconTokens } from "./controls";
import { lazyNamed } from "../../lib/lazy";
import { toolSpec } from "../../lib/tools";

const WastedSheet = lazyNamed(() => import("./WastedSheet"), "WastedSheet");

/** The cuts for a range (all usage), read with the charts' range cut. */
export function useCostCuts(range: RangeId): { input: UsageQueryInput; data: UsageCostCutsInfo | null; loading: boolean } {
  const now = useMinute();
  const hour = Math.floor(now / 3_600_000);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const cut = useMemo(() => cutRange(range, now, null), [range, hour]);
  const input: UsageQueryInput = { edges_ms: [cut.edges[0]!, cut.edges[cut.edges.length - 1]!], group_by: "profile", profile: null, project: null, top: 0 };
  const key = `usage-cuts:${input.edges_ms.join(":")}`;
  const result = useCached(key, () => usageHistoryCuts(input), 60_000);
  return { input, data: result.data, loading: result.loading };
}

/** "List prices of 3 Nov 2026" (the price list the estimates use). */
export function priceListWords(prices: UsageCostCutsInfo["prices"]): string {
  const [year, month, day] = prices.captured.split("-").map(Number);
  const when = year && month && day ? new Date(Date.UTC(year, month - 1, day)).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric", timeZone: "UTC" }) : prices.captured;
  return `List prices of ${when}${prices.overridden ? ", with your own" : ""}`;
}

/** "~$1.92" a cut's average per session, or null when it has none or no price. */
export function perTask(cut: UsageCutInfo): number | null {
  return cut.count > 0 && cut.usd != null ? cut.usd / cut.count : null;
}

/** The two tiles: per finished task, and crashed or stuck. */
export function CostCutTiles({ data, input, projects, labels }: { data: UsageCostCutsInfo | null; input: UsageQueryInput; projects?: Map<string, string>; labels?: Map<string, string> }): JSX.Element {
  const [wasted, setWasted] = useState(false);
  const finished = data?.finished;
  const per = finished ? perTask(finished) : null;
  const tokens = finished && finished.count > 0 ? finished.io / finished.count : null;
  const lost = data?.wasted;
  return (
    <div className="grid grid-cols-1 gap-grid @min-[700px]:grid-cols-2" data-testid="usage-cost-cuts">
      <StatTile
        label="Per finished task"
        icon={<IconMoney width={15} height={15} />}
        value={per}
        words={finished && finished.count === 0 ? "No finished tasks in this range" : finished && per == null ? "No list price" : undefined}
        estimate
        unit="usd"
        note={
          finished && finished.count > 0
            ? `${short(tokens ?? 0, "tokens")} tokens a task · ${finished.count} finished${data && data.unknown_endings > 0 ? ` · not counting ${data.unknown_endings} ${data.unknown_endings === 1 ? "session" : "sessions"} other apps ran` : ""}`
            : "A session AI Juggler ran that ended on its own as finished"
        }
        holdSpace={data === null}
        testId="tile-per-task"
      />
      <StatTile
        label="Crashed or probably stuck"
        icon={<IconTokens width={15} height={15} />}
        value={lost && lost.count > 0 ? lost.usd : null}
        words={lost && lost.count === 0 ? "Nothing went on crashed or stuck sessions" : lost && lost.usd == null ? "No list price" : undefined}
        estimate
        unit="usd"
        note={
          lost && lost.count > 0 ? (
            <button type="button" className="text-left text-accent-hi hover:underline" onClick={() => setWasted(true)} data-testid="usage-wasted-open">
              {`On ${lost.count} ${lost.count === 1 ? "session" : "sessions"}: ${[data!.crashed ? `${data!.crashed} crashed` : null, data!.stuck ? `${data!.stuck} probably stuck` : null].filter(Boolean).join(", ")} — see them`}
            </button>
          ) : undefined
        }
        holdSpace={data === null}
        testId="tile-wasted"
      />
      {wasted && data && <WastedSheet input={input} count={data.wasted.count} projects={projects} labels={labels} onClose={() => setWasted(false)} />}
    </div>
  );
}

type SortBy = "cost" | "tokens";

/** How sure an Account's estimate is. */
function priceWords(row: UsageAccountCostInfo): string {
  if (row.usd == null) return row.io > 0 ? "No list price" : "—";
  return row.unpriced ? "list price, but one model has none" : LIST_PRICE;
}

const LIST_PRICE = "list price";

/** Each Account: tokens, ~cost, its top project, how sure the price is. */
export function CostByAccount({ data, busy, className = "" }: { data: UsageCostCutsInfo | null; busy: boolean; className?: string }): JSX.Element {
  const host = useActionHost();
  const [sort, setSort] = useState<SortBy>("cost");
  const rows = useMemo(() => {
    const list = [...(data?.by_account ?? [])];
    if (sort === "tokens") list.sort((a, b) => b.io - a.io || a.label.localeCompare(b.label));
    return list;
  }, [data, sort]);
  // "list price" on every row says nothing the subtitle doesn't: the
  // column only when a row's price differs (the 0.4 review, 58).
  const priced = rows.some((row) => priceWords(row) !== LIST_PRICE);
  const header = (id: SortBy, label: string) => (
    <th scope="col" aria-sort={sort === id ? "descending" : "none"} className="px-2 py-1.5 text-right font-semibold">
      <button type="button" className="hover:text-ink" onClick={() => setSort(id)} data-testid={`cost-by-account-sort-${id}`}>
        {label}
        {sort === id ? " ↓" : ""}
      </button>
    </th>
  );
  return (
    <Section title="Cost by account" icon={<IconMoney />} sub={data ? priceListWords(data.prices) : "at list prices"} className={className} busy={busy} testId="cost-by-account">
      {rows.length === 0 ? (
        <p className="py-6 text-center text-meta text-ink-2">{data ? "No usage in this range yet." : "Reading usage…"}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-body">
            <thead className="border-b border-hairline text-meta text-muted">
              <tr>
                <th scope="col" className="px-2 py-1.5 text-left font-semibold">
                  Account
                </th>
                {header("tokens", "Tokens")}
                {header("cost", "~Cost")}
                <th scope="col" className="px-2 py-1.5 text-left font-semibold">
                  Mostly on
                </th>
                {priced && (
                  <th scope="col" className="px-2 py-1.5 text-left font-semibold">
                    Price
                  </th>
                )}
              </tr>
            </thead>
            <tbody className="divide-y divide-hairline/60">
              {rows.map((row) => (
                <tr key={row.key} data-testid="cost-by-account-row">
                  <th scope="row" className="px-2 py-2 text-left font-semibold text-ink">
                    {row.account_id ? (
                      <button type="button" className="text-left hover:text-accent-hi hover:underline" onClick={() => host.openAccountById?.(row.account_id!)}>
                        {row.label}
                      </button>
                    ) : (
                      row.label
                    )}
                    {row.tool && <span className="block text-meta font-normal text-muted">{toolSpec(row.tool).display_name}</span>}
                  </th>
                  <td className="px-2 py-2 text-right tabular-nums" title={exact(row.io, "tokens")}>
                    {short(row.io, "tokens")}
                  </td>
                  <td className="px-2 py-2 text-right tabular-nums" title={row.usd == null ? "No list price" : exact(row.usd, "usd")}>
                    {row.usd == null ? <span className="text-muted">—</span> : `~${usdWords(row.usd)}`}
                  </td>
                  <td className="max-w-[14rem] truncate px-2 py-2">
                    {row.top_project ? (
                      <button type="button" className="truncate text-left hover:text-accent-hi hover:underline" onClick={() => host.openProjectById?.(row.top_project!.id)} title={row.top_project.label}>
                        {row.top_project.label}
                      </button>
                    ) : (
                      <span className="text-muted">No project</span>
                    )}
                  </td>
                  {priced && <td className="px-2 py-2 text-meta text-muted">{priceWords(row)}</td>}
                </tr>
              ))}
            </tbody>
          </table>
          <p className="pt-2 text-meta text-muted">API-equivalent estimate — your subscription price doesn't change per token. Subscription fees are never mixed in.</p>
        </div>
      )}
    </Section>
  );
}

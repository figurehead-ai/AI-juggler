/**
 * AJ-E2902: last week's cost review (on demand, from Usage): the last
 * full week, from the locale's first day, against the week before — the
 * ~cost and tokens, each Account with its top project, per finished
 * task, what went on crashed or stuck sessions, the five costliest
 * sessions — and Export CSV of that week. Its code loads when it opens.
 */
import { useMemo, useState, type JSX } from "react";
import { Modal } from "../form";
import { Button } from "../primitives";
import { StatTile } from "../charts/StatTile";
import { IconMoney, IconTokens } from "./controls";
import { usageHistory, usageHistoryCuts, type UsageHistoryInfo, type UsageCostCutsInfo, type UsageQueryInput } from "../../lib/ipc/usageHistory";
import { costCsvExport } from "../../lib/ipc/costs";
import { errorText } from "../../lib/bridge";
import { useCached } from "../../lib/usage/cached";
import { newTokens } from "../../lib/usage/tokens";
import { firstDayOfWeek, lastFullWeek, weekWords } from "../../lib/usage/week";
import { CostByAccount, CostCutTiles } from "./CostCuts";
import { SessionRow, SessionsHeader } from "./TopLists";
import { folderAppName } from "../../lib/app-name";

export function CostReview({ onClose, now = Date.now(), firstDay = firstDayOfWeek() }: { onClose: () => void; now?: number; firstDay?: number }): JSX.Element {
  const week = useMemo(() => lastFullWeek(now, firstDay), [now, firstDay]);
  const input: UsageQueryInput = { edges_ms: [week.start, week.end], group_by: "profile", profile: null, project: null, top: 5 };
  const history = useCached<UsageHistoryInfo>(`review:${week.start}`, () => usageHistory(input), 60_000);
  const cuts = useCached<UsageCostCutsInfo>(`review-cuts:${week.start}`, () => usageHistoryCuts(input), 60_000);
  const [csv, setCsv] = useState<string | null>(null);
  const data = history.data;
  const projects = new Map((data?.by_project ?? []).map((share) => [share.key, share.label]));
  const labels = new Map((data?.by_profile ?? []).map((share) => [share.key, folderAppName(share.key, share.label)]));
  const empty = data !== null && data.totals.messages === 0;
  const costliest = [...(data?.top_sessions ?? [])].sort((a, b) => (b.usd ?? -1) - (a.usd ?? -1)).slice(0, 5);
  const exportWeek = () => void costCsvExport(undefined, { fromMs: week.start, toMs: week.end }).then(setCsv, (cause: unknown) => setCsv(`Couldn't write the CSV: ${errorText(cause)}`));
  return (
    <Modal title="Last week's cost review" subtitle={`${weekWords(week.start, week.end)}, against the week before`} table onClose={onClose} testId="cost-review">
      <div className="@container flex flex-col gap-grid overflow-y-auto" data-testid="cost-review-body">
        {empty ? (
          <p className="rounded-xl border border-hairline px-4 py-5 text-center text-body text-ink-2" data-testid="cost-review-empty">
            No usage in that week.
          </p>
        ) : (
          <>
            <div className="grid grid-cols-1 gap-grid @min-[700px]:grid-cols-2">
              <StatTile
                label="Estimated cost"
                icon={<IconMoney width={15} height={15} />}
                value={data?.totals.usd ?? null}
                words={data && data.totals.usd == null ? "No list price" : undefined}
                estimate
                unit="usd"
                before={data?.previous.usd ?? null}
                better="down"
                period="the week before"
                holdSpace={data === null}
                testId="review-cost"
              />
              <StatTile
                label="New tokens"
                icon={<IconTokens width={15} height={15} />}
                value={data ? newTokens(data.totals.tokens) : null}
                unit="tokens"
                before={data ? newTokens(data.previous.tokens) : null}
                period="the week before"
                holdSpace={data === null}
                testId="review-tokens"
              />
            </div>
            <CostCutTiles data={cuts.data} input={input} projects={projects} labels={labels} />
            <CostByAccount data={cuts.data} busy={cuts.loading && cuts.data !== null} />
            <section aria-labelledby="review-costliest" data-testid="review-costliest">
              <h3 id="review-costliest" className="pb-2 text-section font-semibold text-ink">
                The five costliest sessions
              </h3>
              <SessionsHeader />
              <ul className="divide-y divide-hairline/60">
                {costliest.map((row) => (
                  <li key={`${row.profile}:${row.id}`}>
                    <SessionRow row={row} projects={projects} labels={labels} total={data ? newTokens(data.totals.tokens) : 0} />
                  </li>
                ))}
              </ul>
            </section>
          </>
        )}
        <div className="flex flex-wrap items-center gap-3">
          <p className="min-w-0 flex-1 text-meta text-ink-2">API-equivalent estimate — your subscription price doesn't change per token.</p>
          <Button size="sm" variant="secondary" disabled={empty} onClick={exportWeek} data-testid="cost-review-export">
            Export CSV of this week
          </Button>
        </div>
        {csv && (
          <p role="status" className="text-meta text-ink-2" data-testid="cost-review-csv">
            {csv.startsWith("Couldn't") ? csv : <>CSV written: <span className="font-mono">{csv}</span></>}
          </p>
        )}
      </div>
    </Modal>
  );
}

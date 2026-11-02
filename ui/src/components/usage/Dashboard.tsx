/**
 * 0.1.5 — the usage dashboard, for all usage (Usage), one project (its
 * Usage tab) or one profile (its page): the figures first, then usage
 * over time, who and what used it, when, and the top projects and
 * sessions. The frame draws at once; the last summary shows from memory
 * and is replaced when the fresh one comes.
 */
import { useMemo, useState, type JSX, type ReactNode } from "react";
import { useActionHost } from "../ItemMenu";
import { useSubscriptions } from "../../lib/subscriptions";
import { RANGES, bucketWords, perWords, tickWords, weekend, type RangeId } from "../../lib/usage/range";
import type { AccountInfo } from "../../lib/bridge";
import { ChartBody, Legend, Section, TableToggle } from "../charts/ChartCard";
import { Donut } from "../charts/Donut";
import { StatTile } from "../charts/StatTile";
import { TimeChart, timeTable } from "../charts/TimeChart";
import { READING_SHORT, readingLine } from "../../lib/history-reading";
import { API_PRICE, short, type Unit } from "../charts/scale";
import type { GroupBy } from "../charts/colors";
import { accountGroups, foldItems, foldSeries } from "./accounts";
import { ActiveHours, SessionsPerBucket } from "./Activity";
import { IconChart, IconClock, IconDonut, IconMoney, IconPlay, IconTokens, Segmented } from "./controls";
import { TopProjects, TopSessions } from "./TopLists";
import { chartSeries, shareItems, useUsageHistory, type Scope } from "./useUsage";

export type DashboardKind = "all" | "project" | "profile";
type Split = GroupBy | "account";

const SPLITS: Record<DashboardKind, { id: Split; label: string }[]> = {
  all: [
    { id: "account", label: "Account" },
    { id: "profile", label: "Profile" },
    { id: "tool", label: "Tool" },
    { id: "model", label: "Model" },
    { id: "project", label: "Project" },
  ],
  project: [
    { id: "profile", label: "Profile" },
    { id: "account", label: "Account" },
    { id: "tool", label: "Tool" },
    { id: "model", label: "Model" },
  ],
  profile: [
    { id: "project", label: "Project" },
    { id: "model", label: "Model" },
  ],
};

/** The grid every usage page lays its cards on: one column on a narrow
 * window, two from 700 px of content, three from 1150, four from 1400. */
export const GRID = "grid grid-flow-row-dense grid-cols-1 gap-grid @min-[700px]:grid-cols-2 @min-[1150px]:grid-cols-3 @min-[1400px]:grid-cols-4";
export const SPAN = {
  wide: "@min-[700px]:col-span-2 @min-[1400px]:col-span-3",
  half: "@min-[700px]:col-span-2",
  narrowLate: "@min-[700px]:col-span-2 @min-[1150px]:col-span-1",
  table: "@min-[700px]:col-span-2 @min-[1150px]:col-span-3 @min-[1400px]:col-span-2",
  full: "col-span-full",
} as const;

export function UsageDashboard({
  kind,
  scope,
  range,
  accounts,
  runway,
  testId = "usage-dashboard",
}: {
  kind: DashboardKind;
  scope: Scope;
  range: RangeId;
  accounts: AccountInfo[];
  /** Cards placed after the first row (Usage's runway). */
  runway?: ReactNode;
  testId?: string;
}): JSX.Element {
  const splits = SPLITS[kind];
  const [split, setSplit] = useState<Split>(splits[0]!.id);
  const [metric, setMetric] = useState<"tokens" | "usd">("tokens");
  const [table, setTable] = useState(false);
  const groupBy: GroupBy = split === "account" ? "profile" : split;
  const view = useUsageHistory(range, groupBy, scope);
  const subscriptions = useSubscriptions(0, kind !== "profile");
  const host = useActionHost();
  const labels = useMemo(() => new Map(accounts.map((account) => [account.id, account.label])), [accounts]);
  const groups = useMemo(() => accountGroups(subscriptions, labels), [subscriptions, labels]);
  const data = view.data;
  const busy = view.loading && data !== null;
  const empty = data !== null && data.totals.messages === 0;
  const before = RANGES.find((entry) => entry.id === range)!.before;
  const words = bucketWords(view.cut.step);
  const unit: Unit = metric === "usd" ? "usd" : "tokens";

  const series = useMemo(() => {
    if (!data || data.edges_ms.length !== view.cut.edges.length) return [];
    const raw = chartSeries(data, groupBy, metric);
    return split === "account" ? foldSeries(raw, groups) : raw;
  }, [data, groupBy, metric, split, groups, view.cut.edges.length]);
  const edges = data?.edges_ms ?? view.cut.edges;
  const n = edges.length - 1;
  const perBucket = (pick: (i: number) => number) => Array.from({ length: n }, (_, i) => pick(i));
  const tokensTrend = data ? perBucket((i) => data.series.reduce((sum, entry) => sum + (entry.io[i] ?? 0), 0)) : [];
  const usdTrend = data ? perBucket((i) => data.series.reduce((sum, entry) => sum + (entry.usd[i] ?? 0), 0)) : [];
  const hoursTrend = data ? perBucket((i) => data.active_hours.filter((at) => at >= edges[i]! && at < edges[i + 1]!).length) : [];

  const whoItems = useMemo(() => {
    if (!data) return [];
    if (kind === "profile") return shareItems(data.by_project, "project");
    const profiles = shareItems(data.by_profile, "profile", "tokens", 99);
    return kind === "all" ? foldItems(profiles, groups).slice(0, 7) : profiles.slice(0, 7);
  }, [data, kind, groups]);
  const whoBy: Split = kind === "profile" ? "project" : kind === "all" ? "account" : "profile";
  const models = data ? shareItems(data.by_model, "model") : [];
  const io = data ? data.totals.tokens.input + data.totals.tokens.output : null;
  const openWho = (key: string) => {
    if (whoBy === "project") host.openProjectById?.(key);
    else if (whoBy === "profile") {
      const account = accounts.find((entry) => entry.id === key);
      if (account) host.openAccount?.(account);
    }
  };
  // The 0.2 walk (F6): while the first read of the history runs, a zero
  // is only "not read yet": the tiles say so, and a line says how far.
  const reading = data?.reading ?? null;
  const pending = reading !== null && (io ?? 0) === 0;
  const emptyWords = data === null ? "Reading usage…" : reading !== null ? READING_SHORT : "No usage in this range yet";

  return (
    <div className="@container flex flex-col gap-grid" data-testid={testId} aria-busy={view.loading || reading !== null || undefined}>
      {reading !== null && (
        <p role="status" className="text-body text-ink-2" data-testid="usage-reading">
          {readingLine(reading)}
        </p>
      )}
      <div className="grid grid-cols-2 gap-grid @min-[860px]:grid-cols-4" data-testid="usage-tiles">
        <StatTile label="Tokens in and out" icon={<IconTokens width={15} height={15} />} value={pending ? null : io} words={pending ? READING_SHORT : undefined} unit="tokens" before={data ? data.previous.tokens.input + data.previous.tokens.output : null} period={before} trend={tokensTrend} holdSpace={data === null} testId="tile-tokens" />
        <StatTile
          label="Estimated cost"
          icon={<IconMoney width={15} height={15} />}
          // §4.11: an estimate that can't be made says so — never "$0".
          value={pending ? null : (data?.totals.usd ?? null)}
          words={pending ? READING_SHORT : data && data.totals.usd == null ? (io ? "No list price" : "No usage yet") : undefined}
          atLeast={!!data?.totals.unpriced && data.totals.usd != null}
          estimate
          unit="usd"
          before={data?.previous.usd ?? null}
          better="down"
          period={before}
          trend={usdTrend}
          // §4.11: every cost says it is the API's price, not the bill.
          // Before the first figures, the line it usually says holds its place.
          note={data?.totals.unpriced ? (data.totals.usd == null ? "None of these models has a list price" : `${API_PRICE}. Some models have no list price, so they aren't counted`) : data?.totals.usd != null || data === null ? API_PRICE : undefined}
          holdSpace={data === null}
          testId="tile-cost"
        />
        <StatTile
          label="Sessions"
          icon={<IconPlay width={15} height={15} />}
          value={pending ? null : (data?.totals.sessions ?? null)}
          words={pending ? READING_SHORT : undefined}
          unit="count"
          before={data?.previous.sessions ?? null}
          period={before}
          trend={data?.sessions_per_bucket}
          note={data && data.unmeasured_sessions > 0 ? `+${data.unmeasured_sessions} not measured` : undefined}
          holdSpace={data === null}
          testId="tile-sessions"
        />
        <StatTile label="Active hours" icon={<IconClock width={15} height={15} />} value={pending ? null : (data?.totals.active_hours ?? null)} words={pending ? READING_SHORT : undefined} unit="count" suffix="h" before={data?.previous.active_hours ?? null} period={before} trend={hoursTrend} holdSpace={data === null} testId="tile-hours" />
      </div>
      <div className={GRID}>
        <Section
          title="Usage over time"
          icon={<IconChart />}
          sub={`${metric === "usd" ? "estimated cost" : "tokens in and out"}, ${perWords(view.cut.step)}`}
          className={SPAN.wide}
          busy={busy}
          testId="usage-over-time"
          actions={
            <>
              <Segmented label="Split by" size="sm" options={splits} value={split} onChange={setSplit} testId="usage-split" />
              <Segmented
                label="Measure"
                size="sm"
                options={[
                  { id: "tokens", label: "Tokens" },
                  { id: "usd", label: "Cost" },
                ]}
                value={metric}
                onChange={setMetric}
              />
              <TableToggle on={table} onToggle={() => setTable(!table)} />
            </>
          }
        >
          <Legend label="Series" items={series.map((entry) => ({ key: entry.key, label: entry.label, color: entry.color, value: short(entry.values.reduce((a, b) => a + b, 0), unit) }))} />
          <ChartBody empty={data === null || empty || series.length === 0} emptyWords={emptyWords} height={280} table={timeTable(edges, series, unit, words)} showTable={table} caption="Usage over time">
            <TimeChart
              edges={edges}
              series={series}
              unit={unit}
              height={280}
              label={`${metric === "usd" ? "Estimated cost" : "Tokens"} ${perWords(view.cut.step)}, split by ${split}`}
              bucketWords={words}
              tickWords={tickWords(view.cut.step)}
              shade={weekend(view.cut.step)}
              markers={(data?.switches ?? []).map((move) => ({ at: move.at_ms, label: "Work moved to another account" }))}
            />
          </ChartBody>
        </Section>
        <Section title={whoBy === "project" ? "By project" : whoBy === "account" ? "By account" : "Who did the work"} icon={<IconDonut />} busy={busy} testId="usage-by-who">
          <ChartBody empty={data === null || empty} emptyWords={emptyWords} height={176} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <Donut items={whoItems} unit="tokens" label={`Tokens by ${whoBy}`} centre={{ value: short(io ?? 0, "tokens"), words: "tokens" }} onPick={whoBy === "account" ? undefined : openWho} />
          </ChartBody>
        </Section>
        {runway}
        <ActiveHours data={data} busy={busy} emptyWords={emptyWords} className={SPAN.half} />
        <Section title="By model" icon={<IconDonut />} busy={busy} testId="usage-by-model">
          <ChartBody empty={data === null || empty} emptyWords={emptyWords} height={176} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <Donut items={models} unit="tokens" label="Tokens by model" centre={{ value: models[0] && io ? `${Math.round((models[0].value / io) * 100)}%` : "—", words: models[0]?.label ?? "" }} />
          </ChartBody>
        </Section>
        <SessionsPerBucket data={data} cut={view.cut} busy={busy} emptyWords={emptyWords} className={SPAN.narrowLate} />
        {kind !== "project" && kind !== "profile" && <TopProjects data={data} busy={busy} className={SPAN.half} />}
        <TopSessions data={data} busy={busy} range={range} scope={scope} className={kind === "all" ? SPAN.table : "@min-[700px]:col-span-2 @min-[1400px]:col-span-4"} />
      </div>
    </div>
  );
}

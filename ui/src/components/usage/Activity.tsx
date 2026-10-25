/**
 * 0.1.5 — when the work happens: the hours with usage by weekday and
 * hour (in the user's time zone), and how many sessions used anything in
 * each bucket of the range.
 */
import { useState, type JSX } from "react";
import type { UsageHistoryInfo } from "../../lib/ipc/usageHistory";
import { bucketWords, perWords, tickWords, weekend, type Cut } from "../../lib/usage/range";
import { ChartBody, Section, TableToggle } from "../charts/ChartCard";
import { Heatmap, RampKey, WEEKDAYS, hourCells } from "../charts/Heatmap";
import { TimeChart, timeTable } from "../charts/TimeChart";
import { IconChart, IconClock } from "./controls";

const HOUR_LABELS = [0, 3, 6, 9, 12, 15, 18, 21].map((hour) => ({ col: hour, words: String(hour).padStart(2, "0") }));

export function ActiveHours({ data, busy, emptyWords, className = "" }: { data: UsageHistoryInfo | null; busy: boolean; emptyWords: string; className?: string }): JSX.Element {
  const [table, setTable] = useState(false);
  const cells = hourCells(data?.active_hours ?? []);
  const empty = data === null || data.active_hours.length === 0;
  const rows = WEEKDAYS.map((day, row) => [day, ...cells.filter((cell) => cell.row === row).map((cell) => cell.count)]);
  return (
    <Section
      title="When you work"
      icon={<IconClock />}
      sub="hours with usage, by weekday and hour, in your time zone"
      className={className}
      busy={busy}
      testId="usage-heatmap"
      actions={<TableToggle on={table} onToggle={() => setTable(!table)} />}
    >
      <ChartBody
        empty={empty}
        emptyWords={emptyWords}
        height={220}
        table={{ columns: [{ label: "Day" }, ...Array.from({ length: 24 }, (_, hour) => ({ label: String(hour).padStart(2, "0"), numeric: true }))], rows }}
        showTable={table}
        caption="Hours with usage by weekday and hour"
      >
        <Heatmap cells={cells} columns={24} label="Hours with usage by weekday and hour" columnLabels={HOUR_LABELS} />
        <RampKey less="Fewer" more="More weeks with usage" />
      </ChartBody>
    </Section>
  );
}

export function SessionsPerBucket({ data, cut, busy, emptyWords, className = "" }: { data: UsageHistoryInfo | null; cut: Cut; busy: boolean; emptyWords: string; className?: string }): JSX.Element {
  const [table, setTable] = useState(false);
  const edges = data?.edges_ms ?? cut.edges;
  const series =
    data && data.sessions_per_bucket.length === edges.length - 1
      ? [{ key: "sessions", label: "Sessions with usage", values: data.sessions_per_bucket, color: "var(--color-accent)" }]
      : [];
  const started = data?.started_per_bucket.reduce((sum, value) => sum + value, 0) ?? 0;
  const words = bucketWords(cut.step);
  return (
    <Section
      title="Sessions"
      icon={<IconChart />}
      sub={`with usage, ${perWords(cut.step)}`}
      className={className}
      busy={busy}
      testId="usage-sessions-chart"
      actions={<TableToggle on={table} onToggle={() => setTable(!table)} />}
    >
      <ChartBody empty={series.length === 0 || data?.totals.sessions === 0} emptyWords={emptyWords} height={200} table={timeTable(edges, series, "count", words)} showTable={table} caption="Sessions with usage">
        <TimeChart edges={edges} series={series} unit="count" height={200} label={`Sessions with usage ${perWords(cut.step)}`} bucketWords={words} tickWords={tickWords(cut.step)} shade={weekend(cut.step)} />
      </ChartBody>
      {data && (
        <p className="text-label text-muted">
          {started} started in this range.
          {data.unmeasured_sessions > 0 && ` ${data.unmeasured_sessions} more ran without token counts on this Mac, so they aren't in these numbers.`}
        </p>
      )}
    </Section>
  );
}

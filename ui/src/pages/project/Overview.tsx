/**
 * 0.1.5 — a project's Overview, visual first: what's open and how much
 * work happened this week, activity over the last 30 days and who did it,
 * the sessions open now, the checkout's state with what to do, which
 * account starts work here, and how the work travelled between accounts.
 */
import { useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { ChartBody, Legend, Section } from "../../components/charts/ChartCard";
import { Donut } from "../../components/charts/Donut";
import { StatTile } from "../../components/charts/StatTile";
import { TimeChart } from "../../components/charts/TimeChart";
import { Timeline } from "../../components/charts/Timeline";
import { short } from "../../components/charts/scale";
import { seriesColor } from "../../components/charts/colors";
import { GRID, SPAN } from "../../components/usage/Dashboard";
import { IconChart, IconClock, IconDonut, IconList, IconPlay, IconRoute, IconTokens, Segmented } from "../../components/usage/controls";
import { chartSeries, shareItems, useUsageHistory } from "../../components/usage/useUsage";
import { SwitchGraph } from "../../components/SwitchGraph";
import { switchGraphOverview } from "../../lib/bridge";
import { bucketWords, tickWords, weekend } from "../../lib/usage/range";
import { agoWords } from "../../lib/time-words";
import { lastWorked } from "../../lib/project-words";
import type { ProjectInfo } from "../../lib/bridge";
import type { Row } from "../SessionsList";
import { CheckoutCallout, OpenSessions } from "./Callouts";

export function ProjectOverviewView({
  project,
  rows,
  now,
  sharedBy,
  accounts,
  checkouts,
  onOpenSession,
  onAllSessions,
}: {
  project: ProjectInfo;
  rows: Row[];
  now: number;
  /** The most live sessions sharing one checkout. */
  sharedBy: number;
  /** The accounts card (who starts work here). */
  accounts: ReactNode;
  /** The checkouts card, when there are several. */
  checkouts: ReactNode;
  onOpenSession: (id: string) => void;
  onAllSessions: () => void;
}): JSX.Element {
  const [split, setSplit] = useState<"profile" | "tool" | "model">("profile");
  const month = useUsageHistory("30d", split, { project: project.id }, 0);
  const week = useUsageHistory("7d", "profile", { project: project.id }, 0);
  // History-wide numbers come from the summaries, never from the app's
  // working set of sessions (it holds the open and the newest only).
  const ever = useUsageHistory("all", "profile", { project: project.id }, 0);
  const data = month.data;
  const cut = month.cut;
  const series = useMemo(() => (data && data.edges_ms.length === cut.edges.length ? chartSeries(data, split, "tokens") : []), [data, split, cut.edges.length]);
  const who = data ? shareItems(data.by_profile, "profile") : [];
  const io = data ? data.totals.tokens.input + data.totals.tokens.output : 0;
  const weekIo = week.data ? week.data.totals.tokens.input + week.data.totals.tokens.output : null;
  const live = rows.filter((row) => row.live);
  const needs = live.filter((row) => row.railState === "asks_you").length;
  const last = lastWorked(project) || rows[0]?.last || 0;
  const lastBy = rows[0]?.accountLabel;
  const daily = data ? data.edges_ms.slice(1).map((_, i) => data.series.reduce((sum, entry) => sum + (entry.io[i] ?? 0), 0)) : [];
  const empty = data !== null && data.totals.messages === 0;
  const words = bucketWords(cut.step);
  return (
    <div className="@container flex flex-col gap-grid" data-testid="project-visual-overview">
      <div className="grid grid-cols-2 gap-grid @min-[860px]:grid-cols-4" data-testid="project-summary">
        <StatTile label="Open now" icon={<IconPlay width={15} height={15} />} value={live.length} unit="count" note={live.length === 0 ? "Nothing running here" : [needs > 0 && `${needs} ${needs === 1 ? "needs" : "need"} you`, `${live.length - needs} working or idle`].filter(Boolean).join(" · ")} testId="tile-open" />
        <StatTile
          label="Sessions this week"
          icon={<IconList width={15} height={15} />}
          value={week.data?.totals.sessions ?? null}
          unit="count"
          before={week.data?.previous.sessions ?? null}
          period="last week"
          note={
            <span data-testid="project-sessions-ever">
              {ever.data ? `${ever.data.session_count} with usage in all` : "Counting…"}
            </span>
          }
          testId="tile-week-sessions"
        />
        <StatTile label="Tokens this week" icon={<IconTokens width={15} height={15} />} value={weekIo} unit="tokens" before={week.data ? week.data.previous.tokens.input + week.data.previous.tokens.output : null} period="last week" trend={daily} testId="tile-week-tokens" />
        <div className="sheen flex min-w-0 flex-col gap-1.5 rounded-card border border-hairline bg-surface px-5 pb-3.5 pt-4 shadow-card" data-testid="tile-last-worked">
          <span className="flex items-center gap-1.5 text-meta text-muted">
            <IconClock width={15} height={15} aria-hidden="true" />
            Last worked
          </span>
          <span className="text-stat text-ink">{last > 0 ? agoWords(last, now) : "Not yet"}</span>
          {lastBy && <span className="text-label text-muted">on {lastBy}</span>}
        </div>
      </div>
      <div className={GRID}>
        <Section
          title="Activity"
          icon={<IconChart />}
          sub={`tokens in and out on ${project.name}, per day`}
          className={SPAN.wide}
          busy={month.loading && data !== null}
          testId="project-activity"
          actions={
            <Segmented
              label="Split by"
              size="sm"
              options={[
                { id: "profile", label: "Profile" },
                { id: "tool", label: "Tool" },
                { id: "model", label: "Model" },
              ]}
              value={split}
              onChange={setSplit}
            />
          }
        >
          <Legend label="Series" items={series.map((entry) => ({ key: entry.key, label: entry.label, color: entry.color, value: short(entry.values.reduce((a, b) => a + b, 0), "tokens") }))} />
          <ChartBody empty={data === null || empty} emptyWords={data === null ? "Reading usage…" : "No usage here in the last 30 days"} height={240} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <TimeChart edges={cut.edges} series={series} unit="tokens" height={240} label={`Tokens per day on ${project.name}`} bucketWords={words} tickWords={tickWords(cut.step)} shade={weekend(cut.step)} />
          </ChartBody>
        </Section>
        <Section title="Who did the work" icon={<IconDonut />} sub="last 30 days" testId="project-who">
          <ChartBody empty={data === null || empty} emptyWords={data === null ? "Reading usage…" : "No usage yet"} height={168} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <Donut items={who} unit="tokens" label={`Tokens on ${project.name} by profile`} centre={{ value: short(io, "tokens"), words: "tokens" }} />
          </ChartBody>
        </Section>
        <OpenSessions rows={rows} onAll={onAllSessions} className="@min-[1400px]:col-span-2" />
        <CheckoutCallout project={project} sharedBy={sharedBy} />
        {accounts}
        <Travelled project={project} data={data} cut={cut} onOpenSession={onOpenSession} />
        {checkouts}
      </div>
    </div>
  );
}

/** How the work travelled: who worked here day by day, and each move. */
function Travelled({ project, data, cut, onOpenSession }: { project: ProjectInfo; data: ReturnType<typeof useUsageHistory>["data"]; cut: ReturnType<typeof useUsageHistory>["cut"]; onOpenSession: (id: string) => void }): JSX.Element {
  const [moved, setMoved] = useState(false);
  useEffect(() => {
    let alive = true;
    switchGraphOverview(project.id).then(
      (works) => alive && setMoved(works.some((work) => work.epochs.length > 1)),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [project.id]);
  const lanes =
    data && data.group_by === "profile"
      ? data.series.slice(0, 5).map((entry) => ({ key: entry.key, label: entry.label, color: seriesColor("profile", entry.key), values: entry.io }))
      : (data?.by_profile ?? []).slice(0, 5).map((share) => ({ key: share.key, label: share.label, color: seriesColor("profile", share.key), values: [] as number[] }));
  const moves = (data?.switches ?? []).map((move) => ({ at: move.at_ms, from: move.from_profile, to: move.to_profile, words: move.state }));
  const only = lanes[0]?.label;
  return (
    <Section title="How this work travelled" icon={<IconRoute />} sub="who worked here, day by day; arrows are moves between accounts" className={SPAN.full} testId="project-switch-graph-card">
      {lanes.length === 0 || lanes.every((lane) => lane.values.length === 0) ? (
        <p className="rounded-lg border border-dashed border-hairline px-4 py-6 text-center text-meta text-muted">
          {data === null ? "Reading usage…" : "No work here in the last 30 days. When a session here moves to another account, the move shows on this line."}
        </p>
      ) : (
        <Timeline lanes={lanes} edges={cut.edges} moves={moves} label={`Who worked on ${project.name}, day by day`} bucketWords={bucketWords(cut.step)} tickWords={tickWords(cut.step)} />
      )}
      {data && moves.length === 0 && lanes.length > 0 && (
        <p className="text-label text-muted" data-testid="travelled-none">
          {lanes.length === 1 ? `No moves yet: all work here ran on ${only}.` : `No moves between accounts in this range: ${lanes.length} profiles worked here side by side.`} A move to another account, with its history and tasks, shows as an arrow.
        </p>
      )}
      {moved && (
        <details open className="rounded-lg border border-hairline px-3 py-2">
          <summary className="cursor-pointer text-meta font-medium text-ink">Every move, with resume and export</summary>
          <div className="pt-3">
            <SwitchGraph projectId={project.id} onOpenSession={onOpenSession} />
          </div>
        </details>
      )}
    </Section>
  );
}

/**
 * D-103.9 (persona R3, P7: "0 tokens" and "No usage yet" beside 14
 * sessions) — a project's Overview never says no work where work ran:
 * - "Who did the work" is tokens by App over 30 days; when none are
 *   recorded there, it falls back to sessions per Account ("Client B ·
 *   14 sessions"), said as such;
 * - the week's tokens tile, with sessions this week and no token use in
 *   their files, says "None recorded" and how many ran, not a bare 0.
 */
import { useMemo, type JSX } from "react";
import { ChartBody, Section } from "../../components/charts/ChartCard";
import { Donut } from "../../components/charts/Donut";
import { StatTile } from "../../components/charts/StatTile";
import { short } from "../../components/charts/scale";
import { IconDonut, IconTokens } from "../../components/usage/controls";
import { shareItems, type useUsageHistory } from "../../components/usage/useUsage";
import { newTokens } from "../../lib/usage/tokens";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { Row } from "../SessionsList";
import { useSessionsByAccount } from "./sessions-by-account";

type History = ReturnType<typeof useUsageHistory>["data"];
/** What these read of a session row: when it ran, and on which App's folder. */
type Ran = Pick<Row, "last" | "accountId">;

const DAY = 86_400_000;

/** How many of `rows` ran in the last `days` days. */
export function ranWithin(rows: readonly Ran[], now: number, days: number): number {
  return rows.filter((row) => row.last >= now - days * DAY).length;
}

export function WhoDidTheWork({ project, data, rows }: { project: Pick<ProjectInfo, "id" | "name">; data: History; rows: readonly Ran[] }): JSX.Element {
  const local = useMemo(() => rows.map((row) => ({ accountId: row.accountId })), [rows]);
  const sessions = useSessionsByAccount(project.id, local);
  const empty = data !== null && data.totals.messages === 0;
  const io = data ? newTokens(data.totals.tokens) : 0;
  const bySessions = data !== null && (empty || io === 0) && sessions.total > 0;
  if (bySessions) {
    const n = sessions.total;
    return (
      <Section title="Who did the work" icon={<IconDonut />} sub="sessions by account: no token use recorded in 30 days" testId="project-who">
        <div data-testid="project-who-sessions">
          <Donut items={sessions.items} unit="count" label={`Sessions on ${project.name} by account`} centre={{ value: String(n), words: n === 1 ? "session" : "sessions" }} />
        </div>
      </Section>
    );
  }
  return (
    <Section title="Who did the work" icon={<IconDonut />} sub="last 30 days" testId="project-who">
      <ChartBody empty={data === null || empty} emptyWords={data === null ? "Reading usage…" : "No usage yet"} height={168} table={{ columns: [], rows: [] }} showTable={false} caption="">
        <Donut items={data ? shareItems(data.by_profile, "profile") : []} unit="tokens" label={`Tokens on ${project.name} by app`} centre={{ value: short(io, "tokens"), words: "tokens" }} />
      </ChartBody>
    </Section>
  );
}

/** New tokens this week (`trend`: the month's per day); with sessions
 * this week that recorded none, it says so rather than a bare 0. */
export function WeekTokensTile({ week, trend, rows, now }: { week: History; trend: number[]; rows: readonly Ran[]; now: number }): JSX.Element {
  const io = week ? newTokens(week.totals.tokens) : null;
  const ran = ranWithin(rows, now, 7);
  if (week !== null && io === 0 && ran > 0)
    return (
      <StatTile
        label="New tokens this week"
        icon={<IconTokens width={15} height={15} />}
        value={null}
        unit="tokens"
        words="None recorded"
        note={`${ran === 1 ? "1 session" : `${ran} sessions`} this week; their files hold no token use`}
        testId="tile-week-tokens"
      />
    );
  return (
    <StatTile
      label="New tokens this week"
      icon={<IconTokens width={15} height={15} />}
      value={io}
      unit="tokens"
      before={week ? newTokens(week.previous.tokens) : null}
      period="last week"
      trend={trend}
      holdSpace={week === null}
      testId="tile-week-tokens"
    />
  );
}

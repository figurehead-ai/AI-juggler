/**
 * 0.1.5 — one session's Usage tab: what it used turn by turn, by kind of
 * token, by model, and how much of its profile's 5-hour and weekly use
 * it is. Built to sit in the session page's Usage tab (`profile` and
 * `conversation` name it); the frame draws at once.
 */
import { useMemo, useState, type JSX } from "react";
import { ChartBody, Legend, Section, TableToggle } from "../../components/charts/ChartCard";
import { Donut } from "../../components/charts/Donut";
import { ShareBar, StatTile } from "../../components/charts/StatTile";
import { TimeChart, timeTable } from "../../components/charts/TimeChart";
import { modelName, seriesColor } from "../../components/charts/colors";
import { exact, short } from "../../components/charts/scale";
import { IconChart, IconDonut, IconGauge, IconLayers, IconList, IconMoney, IconTokens } from "../../components/usage/controls";
import { useCached } from "../../lib/usage/cached";
import { usageSession, type UsageSessionDetailInfo } from "../../lib/ipc/usageHistory";
import { accountColor } from "../../lib/palette";

/** At most this many bars: more turns are summed into even slices of time. */
const BARS = 120;

const KINDS = [
  { key: "input", label: "Input", color: "var(--color-accent)" },
  { key: "output", label: "Output", color: "var(--color-live)" },
  { key: "cache_read", label: "Cache read", color: "var(--color-faint)" },
  { key: "cache_write", label: "Cache written", color: "var(--color-warning)" },
] as const;

const time = (at: number) => new Date(at).toLocaleString([], { weekday: "short", hour: "numeric", minute: "2-digit" });

/** The turns as bars: one per turn, or summed into even slices of time. */
export function turnBars(detail: UsageSessionDetailInfo): { edges: number[]; input: number[]; output: number[]; perTurn: boolean } {
  const turns = detail.turns;
  if (turns.length === 0) return { edges: [0, 1], input: [0], output: [0], perTurn: true };
  if (turns.length <= BARS) {
    // A bar per turn, each edge after the one before (two turns can share a millisecond).
    const edges: number[] = [];
    for (const turn of turns) edges.push(Math.max(turn.at_ms, (edges[edges.length - 1] ?? -Infinity) + 1));
    edges.push(edges[edges.length - 1]! + 1);
    return { edges, input: turns.map((turn) => turn.tokens.input), output: turns.map((turn) => turn.tokens.output), perTurn: true };
  }
  const start = turns[0]!.at_ms;
  const end = turns[turns.length - 1]!.at_ms + 1;
  const step = (end - start) / BARS;
  const edges = Array.from({ length: BARS + 1 }, (_, i) => Math.round(start + i * step));
  edges[BARS] = end;
  const input = new Array<number>(BARS).fill(0);
  const output = new Array<number>(BARS).fill(0);
  for (const turn of turns) {
    const at = Math.min(BARS - 1, Math.floor((turn.at_ms - start) / step));
    input[at]! += turn.tokens.input;
    output[at]! += turn.tokens.output;
  }
  return { edges, input, output, perTurn: false };
}

export function SessionUsage({ profile, conversation, profileLabel }: { profile: string; conversation: string; profileLabel?: string }): JSX.Element {
  const result = useCached(`session-usage:${profile}:${conversation}`, () => usageSession(profile, conversation), 30_000);
  const detail = result.data;
  const [table, setTable] = useState(false);
  const bars = useMemo(() => (detail ? turnBars(detail) : null), [detail]);
  const io = detail ? detail.totals.tokens.input + detail.totals.tokens.output : null;
  const series = bars
    ? [
        { key: "input", label: "Input", values: bars.input, color: KINDS[0].color },
        { key: "output", label: "Output", values: bars.output, color: KINDS[1].color },
      ]
    : [];
  const kinds = detail ? KINDS.map((kind) => ({ key: kind.key, label: kind.label, value: detail.totals.tokens[kind.key], color: kind.color })) : [];
  const models = (detail?.by_model ?? []).map((share) => ({ key: share.key, label: modelName(share.key), value: share.io, color: seriesColor("model", share.key) }));
  const words = (start: number, end: number) => (bars?.perTurn ? `Turn at ${time(start)}` : `${time(start)} to ${time(end)}`);
  const empty = detail !== null && detail.totals.messages === 0;
  const busy = result.loading && detail !== null;
  const color = accountColor(profile);
  return (
    <div className="@container flex flex-col gap-grid" data-testid="session-usage-visual">
      <div className="grid grid-cols-2 gap-grid @min-[860px]:grid-cols-4">
        <StatTile label="Tokens in and out" icon={<IconTokens width={15} height={15} />} value={io} unit="tokens" testId="session-tile-tokens" />
        <StatTile label="Estimated cost" icon={<IconMoney width={15} height={15} />} value={detail ? (detail.totals.usd ?? 0) : null} unit="usd" note={detail?.totals.unpriced ? "Some of it has no list price" : "at list prices"} testId="session-tile-cost" />
        <StatTile label="Turns" icon={<IconList width={15} height={15} />} value={detail?.totals.messages ?? null} unit="count" note={detail?.truncated ? `the last ${detail.turns.length} drawn below` : undefined} testId="session-tile-turns" />
        <StatTile label="Cache read" icon={<IconLayers width={15} height={15} />} value={detail?.totals.tokens.cache_read ?? null} unit="tokens" note="served from the prompt cache" testId="session-tile-cache" />
      </div>
      <div className="grid grid-cols-1 gap-grid @min-[900px]:grid-cols-3">
        <Section
          title="Tokens per turn"
          icon={<IconChart />}
          sub={bars && !bars.perTurn ? `in ${BARS} slices of its time` : "each turn, oldest first"}
          className="@min-[900px]:col-span-2"
          busy={busy}
          testId="session-usage-turns"
          actions={<TableToggle on={table} onToggle={() => setTable(!table)} />}
        >
          <Legend label="Kinds" items={series.map((entry) => ({ key: entry.key, label: entry.label, color: entry.color, value: short(entry.values.reduce((a, b) => a + b, 0), "tokens") }))} />
          <ChartBody
            empty={detail === null || empty}
            emptyWords={detail === null ? "Reading its turns…" : "No measured turns yet: its tool keeps no token counts here, or it hasn't answered yet"}
            height={240}
            table={timeTable(bars?.edges ?? [0, 1], series, "tokens", words)}
            showTable={table}
            caption="Tokens per turn"
          >
            <TimeChart edges={bars!.edges} series={series} unit="tokens" height={240} label="Input and output tokens per turn" bucketWords={words} tickWords={(at) => new Date(at).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })} mode="bars" />
          </ChartBody>
        </Section>
        <Section title="By kind" icon={<IconDonut />} busy={busy} testId="session-usage-kinds">
          <ChartBody empty={detail === null || empty} emptyWords={detail === null ? "Reading…" : "Nothing yet"} height={168} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <Donut items={kinds} unit="tokens" label="Tokens by kind" centre={{ value: short(kinds.reduce((sum, kind) => sum + kind.value, 0), "tokens"), words: "in all" }} />
          </ChartBody>
        </Section>
        <Section title="Models" icon={<IconDonut />} busy={busy} testId="session-usage-models">
          <ChartBody empty={detail === null || models.length === 0} emptyWords={detail === null ? "Reading…" : "No model recorded"} height={168} table={{ columns: [], rows: [] }} showTable={false} caption="">
            <Donut items={models} unit="tokens" label="Tokens by model" centre={{ value: String(models.length), words: models.length === 1 ? "model" : "models" }} />
          </ChartBody>
        </Section>
        <Section title="Its share of the limits" icon={<IconGauge />} sub={`of ${profileLabel ?? "its profile"}'s use`} className="@min-[900px]:col-span-2" busy={busy} testId="session-usage-share">
          {(detail?.windows ?? []).length === 0 ? (
            <p className="text-meta text-muted">{detail === null ? "Reading…" : "Nothing measured in its windows."}</p>
          ) : (
            <ul className="flex flex-col gap-3">
              {detail!.windows.map((window) => (
                <li key={window.window} className="flex flex-col gap-1.5">
                  <span className="flex flex-wrap items-baseline justify-between gap-2">
                    <span className="text-body font-semibold text-ink">{window.window === "five_hour" ? "This 5-hour window" : "This week"}</span>
                    <span className="text-meta text-muted" title={`${exact(window.session_io, "tokens")} of ${exact(window.profile_io, "tokens")}`}>
                      {short(window.session_io, "tokens")} of {short(window.profile_io, "tokens")} · {window.basis === "window" ? `since ${time(window.since_ms)}` : window.window === "five_hour" ? "the last 5 hours" : "the last 7 days"}
                    </span>
                  </span>
                  <ShareBar parts={[{ key: "this", label: "This session", value: window.session_io, color }]} total={window.profile_io} unit="tokens" label={window.window === "five_hour" ? "This 5-hour window" : "This week"} />
                </li>
              ))}
            </ul>
          )}
        </Section>
      </div>
    </div>
  );
}

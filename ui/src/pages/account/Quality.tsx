/**
 * AJ-E2903: "How its sessions went" on an Account's page (wf07): over the
 * last 7 days, errors and retries over turns, the times its sessions
 * were probably stuck, their context at finish, and tokens per finished
 * task — each with where it comes from, as a session's signals say it,
 * and never a composite score (D-07). Nothing ended in the 7 days: the
 * section waits until there is something to show (D-25).
 */
import { useEffect, useState, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { IconGauge } from "../../components/usage/controls";
import { accountQuality, type AccountQualityInfo } from "../../lib/ipc/quality";
import { toolSpec } from "../../lib/tools";
import { tokenWords } from "../../lib/quotaWindow";

/** Where each figure comes from, in the session signals' words. */
const FROM_FILES = "read from each session's own files when it ended";
const FROM_STATES = "from the states AI Juggler recorded; sessions it didn't run aren't counted";
const WORKED_OUT = "worked out from what's measured";

export interface QualityLine {
  key: string;
  label: string;
  value: string;
  source: string;
  muted?: boolean;
}

/** The section's lines, in words. */
export function qualityLines(info: AccountQualityInfo): QualityLine[] {
  const tool = toolSpec(info.tool).display_name || "This tool";
  const plural = (n: number, one: string, many: string) => `${n.toLocaleString("en-US")} ${n === 1 ? one : many}`;
  return [
    info.errors_measured
      ? { key: "errors", label: "Errors and retries", value: `${plural(info.errors, "error or retry", "errors or retries")} / ${plural(info.turns, "turn", "turns")}`, source: FROM_FILES }
      : { key: "errors", label: "Errors and retries", value: `${tool} doesn't record errors in its files`, source: FROM_FILES, muted: true },
    { key: "stuck", label: "Probably stuck", value: info.stuck_episodes === 0 ? "Not once" : `${plural(info.stuck_episodes, "time", "times")} probably stuck`, source: FROM_STATES },
    info.context_avg_pct != null
      ? { key: "context", label: "Context at finish", value: `${Math.round(info.context_avg_pct)}% on average, over ${plural(info.context_sessions, "session", "sessions")}`, source: FROM_FILES }
      : { key: "context", label: "Context at finish", value: `Not reported by ${tool}`, source: FROM_FILES, muted: true },
    info.tokens_per_finished != null
      ? { key: "per-task", label: "Tokens per finished task", value: `${tokenWords(info.tokens_per_finished)} over ${plural(info.finished, "finished task", "finished tasks")}`, source: WORKED_OUT }
      : { key: "per-task", label: "Tokens per finished task", value: "No finished tasks in these 7 days", source: WORKED_OUT, muted: true },
  ];
}

export function AccountQuality({ folders }: { folders: string[] }): JSX.Element | null {
  const [info, setInfo] = useState<AccountQualityInfo | null>(null);
  const key = folders.join("|");
  useEffect(() => {
    let alive = true;
    void accountQuality(folders).then(
      (answer) => alive && setInfo(answer),
      () => alive && setInfo(null),
    );
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  // Nothing ended in the window: it waits until it has something to show.
  if (!info || info.sessions === 0) return null;
  return (
    <Section title="How its sessions went" icon={<IconGauge />} sub={`the last 7 days, ${info.sessions} ${info.sessions === 1 ? "session" : "sessions"} AI Juggler ran`} testId="account-quality">
      <dl className="flex flex-col divide-y divide-hairline/60">
        {qualityLines(info).map((line) => (
          <div key={line.key} className="flex flex-col gap-0.5 py-2 @min-[560px]:flex-row @min-[560px]:items-baseline @min-[560px]:gap-4" data-testid={`account-quality-${line.key}`}>
            <dt className="w-48 shrink-0 text-meta text-muted">{line.label}</dt>
            <dd className="flex min-w-0 flex-1 flex-col">
              <span className={line.muted ? "text-body text-ink-2" : "text-body font-semibold text-ink"}>{line.value}</span>
              <span className="text-label text-muted">{line.source}</span>
            </dd>
          </div>
        ))}
      </dl>
      {info.pending > 0 && (
        <p className="pt-2 text-label text-muted" data-testid="account-quality-pending">
          {info.pending === 1 ? "1 session just ended: its figures come in a moment." : `${info.pending} sessions just ended: their figures come in a moment.`}
        </p>
      )}
    </Section>
  );
}

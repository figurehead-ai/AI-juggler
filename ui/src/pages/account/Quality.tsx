/**
 * AJ-E2903: "How its sessions went" on an Account's page (wf07): over the
 * last 7 days, errors and retries over turns, the times its sessions
 * were probably stuck, their context at finish, and tokens per finished
 * task — the figures alone, where they come from said once under the
 * title (a row says its own only where it differs), and never a
 * composite score (D-07). What its tool doesn't record is left out, said
 * in one line. Nothing ended in the 7 days: the section waits until
 * there is something to show (D-25).
 */
import { useEffect, useState, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { IconGauge } from "../../components/usage/controls";
import { accountQuality, type AccountQualityInfo } from "../../lib/ipc/quality";
import { toolSpec } from "../../lib/tools";
import { tokenWords } from "../../lib/quotaWindow";

/** Where each figure comes from, in the session signals' words. Most
 * come from the files; that's said once, under the section's title, and
 * a row says its own only where it differs (the 0.4 review). */
export const FROM_FILES = "read from each session's own files when it ended";
const FROM_STATES = "from the states AI Juggler recorded; sessions it didn't run aren't counted";
const WORKED_OUT = "worked out from what's measured";

export interface QualityLine {
  key: string;
  label: string;
  /** The figure alone: the label says what it is. */
  value: string;
  source: string;
}

export interface QualityView {
  lines: QualityLine[];
  /** What's left out and why, in one line ("Codex doesn't record errors
   * or context, so those rows are left out"), or null. */
  leftOut: string | null;
}

/** The section's lines, in words: only the figures there are. A row the
 * tool doesn't record, or with nothing to count, is left out, and one
 * line says which and why (the 0.4 review: "not known" said three ways). */
export function qualityLines(info: AccountQualityInfo): QualityView {
  const tool = toolSpec(info.tool).display_name || "This tool";
  const plural = (n: number, one: string, many: string) => `${n.toLocaleString("en-US")} ${n === 1 ? one : many}`;
  const lines: QualityLine[] = [];
  const unrecorded: string[] = [];
  if (info.errors_measured) lines.push({ key: "errors", label: "Errors and retries", value: `${info.errors.toLocaleString("en-US")} in ${plural(info.turns, "turn", "turns")}`, source: FROM_FILES });
  else unrecorded.push("errors");
  lines.push({ key: "stuck", label: "Probably stuck", value: info.stuck_episodes === 0 ? "Not once" : plural(info.stuck_episodes, "time", "times"), source: FROM_STATES });
  if (info.context_avg_pct != null) lines.push({ key: "context", label: "Context at finish", value: `${Math.round(info.context_avg_pct)}% on average, over ${plural(info.context_sessions, "session", "sessions")}`, source: FROM_FILES });
  else unrecorded.push("context");
  const noTasks = info.tokens_per_finished == null;
  if (!noTasks) lines.push({ key: "per-task", label: "Tokens per finished task", value: `${tokenWords(info.tokens_per_finished!)}, over ${plural(info.finished, "task", "tasks")}`, source: WORKED_OUT });
  const why = [unrecorded.length > 0 ? `${tool} doesn't record ${unrecorded.join(" or ")}` : null, noTasks ? `no task finished in these 7 days` : null].filter((part): part is string => part !== null);
  const rows = unrecorded.length + (noTasks ? 1 : 0);
  const leftOut = why.length === 0 ? null : `${why.join(", and ")}, so ${rows === 1 ? "that row is" : "those rows are"} left out.`;
  return { lines, leftOut: leftOut && leftOut[0]!.toUpperCase() + leftOut.slice(1) };
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
  const { lines, leftOut } = qualityLines(info);
  // The files' source once, under the title, where a row comes from them.
  const fromFiles = lines.some((line) => line.source === FROM_FILES);
  const sub = `the last 7 days, ${info.sessions} ${info.sessions === 1 ? "session" : "sessions"} AI Juggler ran${fromFiles ? `, ${FROM_FILES}` : ""}`;
  return (
    <Section title="How its sessions went" icon={<IconGauge />} sub={sub} testId="account-quality">
      {leftOut && (
        <p className="text-meta text-ink-2" data-testid="account-quality-left-out">
          {leftOut}
        </p>
      )}
      <dl className="flex flex-col divide-y divide-hairline/60">
        {lines.map((line) => (
          <div key={line.key} className="flex flex-col gap-0.5 py-2 @min-[560px]:flex-row @min-[560px]:items-baseline @min-[560px]:gap-4" data-testid={`account-quality-${line.key}`}>
            <dt className="w-48 shrink-0 text-meta text-muted">{line.label}</dt>
            <dd className="flex min-w-0 flex-1 flex-col">
              <span className="text-body font-semibold text-ink">{line.value}</span>
              {line.source !== FROM_FILES && <span className="text-label text-muted">{line.source}</span>}
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

/**
 * AJ-E1004 — the morning summary on the home view: what happened since
 * the user last looked, severity-ordered (crashed → needs you → still
 * running → finished → ended), every row one button into its Instance.
 * Severity is words and an icon, never colour alone; unmeasured cost
 * says Unavailable (D-06/D-07); no composite health score. Built from
 * the status journal, so it is the same after a restart.
 */
import { useEffect, useState, type JSX } from "react";
import { overnightSummary, overnightSummarySeen, type OvernightSummaryInfo, type SummaryRowInfo } from "../lib/bridge";

const GROUPS: Array<{ id: string; title: string; icon: string }> = [
  { id: "crashed", title: "Crashed", icon: "✕" },
  { id: "needs_you", title: "Needs you", icon: "!" },
  { id: "running", title: "Still running", icon: "▶" },
  { id: "finished", title: "Finished", icon: "✓" },
  { id: "ended", title: "Ended", icon: "■" },
];

function since(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

function Row({ row, onOpen }: { row: SummaryRowInfo; onOpen: (id: string) => void }): JSX.Element {
  const last = row.transitions[row.transitions.length - 1];
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpen(row.session_id)}
        className="flex w-full flex-wrap items-center gap-x-2 gap-y-0.5 rounded-lg border border-hairline px-2.5 py-1.5 text-left text-[11px] hover:border-accent/40 hover:bg-surface"
        data-testid="summary-row"
        data-group={row.group}
      >
        <span className="font-medium text-ink">{row.account_label}</span>
        <span className="text-muted">{row.provider}</span>
        {row.project && <span className="text-muted">· {row.project}</span>}
        <span className="text-ink">· {row.words}</span>
        <span className="flex-1" />
        <span className="text-muted">
          {row.tokens !== null ? `${row.tokens.toLocaleString()} tokens` : "tokens: Unavailable"} · cost: Unavailable
        </span>
        {row.leftovers > 0 && (
          <span className="text-ink">
            · {row.leftovers} leftover {row.leftovers === 1 ? "process" : "processes"}
          </span>
        )}
        {last && <span className="w-full text-muted">last change {since(last.at_ms)}</span>}
      </button>
    </li>
  );
}

export function MorningSummary({ onOpenSession }: { onOpenSession: (id: string) => void }): JSX.Element | null {
  const [summary, setSummary] = useState<OvernightSummaryInfo | null>(null);
  useEffect(() => {
    let alive = true;
    overnightSummary()
      .then((next) => alive && setSummary(next))
      .catch(() => alive && setSummary(null));
    return () => {
      alive = false;
    };
  }, []);
  if (!summary || summary.rows.length === 0) return null;
  const headline = GROUPS.filter((group) => (summary.counts[group.id] ?? 0) > 0)
    .map((group) => `${summary.counts[group.id]} ${group.title.toLowerCase()}`)
    .join(" · ");
  return (
    <section
      aria-labelledby="summary-title"
      className="flex w-full max-w-2xl flex-col gap-2 rounded-xl border border-hairline bg-surface p-4"
      data-testid="morning-summary"
    >
      <div className="flex flex-wrap items-baseline gap-2">
        <h2 id="summary-title" className="text-sm font-semibold">
          Since you were away
        </h2>
        <span className="text-[11px] text-muted">since {since(summary.since_ms)}</span>
        <span className="flex-1" />
        <button
          type="button"
          onClick={() => void overnightSummarySeen().then(() => setSummary(null))}
          className="rounded border border-hairline px-2 py-0.5 text-[11px] text-ink"
          data-testid="summary-seen"
        >
          Mark all seen
        </button>
      </div>
      <p className="text-[11px] text-ink" data-testid="summary-headline">
        {headline}
        {summary.leftovers > 0
          ? ` · ${summary.leftovers} leftover ${summary.leftovers === 1 ? "process" : "processes"} to clean up`
          : ""}
      </p>
      {GROUPS.map((group) => {
        const rows = summary.rows.filter((row) => row.group === group.id);
        if (rows.length === 0) return null;
        return (
          <div key={group.id} className="flex flex-col gap-1">
            <p className="text-[10px] font-medium uppercase tracking-wider text-muted">
              <span aria-hidden="true">{group.icon} </span>
              {group.title} ({rows.length})
            </p>
            <ul className="flex flex-col gap-1" aria-label={group.title}>
              {rows.map((row) => (
                <Row key={row.session_id} row={row} onOpen={onOpenSession} />
              ))}
            </ul>
          </div>
        );
      })}
    </section>
  );
}

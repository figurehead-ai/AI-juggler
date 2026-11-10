/**
 * AJ-E1004 — the morning summary on the home view: what happened since
 * the user last looked, severity-ordered (crashed → needs you → still
 * running → finished → ended), every row one button into its Instance.
 * Severity is words and an icon, never colour alone; tokens when known,
 * never a 0 for what wasn't measured (D-06/D-07), and silent otherwise
 * (0.4.1, principle 5); no composite health score. Built from
 * the status journal, so it is the same after a restart.
 *
 * AJ-E3307 (D2): a waiting row says how long it has waited (since it
 * began to wait, the rule Home's Needs you and the bell read), the
 * longest first; an ended one says when it ended. The last summary is
 * kept, so Home opened again paints it at once and reads behind it (it
 * popped in a second or two later, pushing Home down).
 */
import { useState, type JSX } from "react";
import { overnightSummary, overnightSummarySeen } from "../lib/ipc/park";
import { sharedRead, useSharedRead } from "../lib/shared-read";
import { waitingWords } from "../lib/waiting";
import { summaryHeadline } from "../lib/ended-how";
import type { OvernightSummaryInfo } from "../types/generated/OvernightSummaryInfo";
import type { SummaryRowInfo } from "../types/generated/SummaryRowInfo";
import { sessionAppName } from "../lib/app-name";

/** Rows a group shows before "Show all". */
const SHOWN = 5;

const GROUPS: Array<{ id: string; title: string; icon: string }> = [
  { id: "crashed", title: "Crashed", icon: "✕" },
  { id: "error", title: "Error", icon: "✕" },
  { id: "needs_you", title: "Needs you", icon: "!" },
  { id: "running", title: "Still running", icon: "▶" },
  { id: "finished", title: "Finished", icon: "✓" },
  { id: "stopped", title: "Stopped by you", icon: "■" },
  { id: "switched", title: "Stopped by a switch", icon: "■" },
  { id: "ended", title: "Ended", icon: "■" },
];


function since(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

/** The summary, kept between Home's openings (lib/shared-read). */
const summaryRead = sharedRead(() => overnightSummary());

const ENDED_GROUPS = new Set(["crashed", "error", "finished", "stopped", "switched", "ended"]);

/** When, under a row: how long it has waited, when it ended, else its
 * last change. */
function whenLine(row: SummaryRowInfo, now: number): string | null {
  if (row.at_ms !== null && row.group === "needs_you") return waitingWords(row.at_ms, now);
  if (row.at_ms !== null && ENDED_GROUPS.has(row.group)) return `ended ${since(row.at_ms)}`;
  const last = row.transitions[row.transitions.length - 1];
  return last ? `last change ${since(last.at_ms)}` : null;
}

function Row({ row, now, onOpen }: { row: SummaryRowInfo; now: number; onOpen: (id: string) => void }): JSX.Element {
  const when = whenLine(row, now);
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpen(row.session_id)}
        className="flex w-full flex-wrap items-center gap-x-2 gap-y-0.5 rounded-lg border border-hairline px-2.5 py-1.5 text-left text-label hover:border-accent/40 hover:bg-surface"
        data-testid="summary-row"
        data-group={row.group}
      >
        {/* Its App by its full name (the 0.4 review). */}
        <span className="font-medium text-ink">{sessionAppName(row)}</span>
        {row.project && <span className="text-muted">· {row.project}</span>}
        <span className="text-ink">· {row.detail ? `${row.words} · ${row.detail}` : row.words}</span>
        <span className="flex-1" />
        {/* Its tokens when they're known; nothing when they aren't (an
            unknown value stays silent, never a 0: the persona walk found
            "tokens: Unavailable · cost: Unavailable" on every row, the
            runway pushed below the fold). The row has no cost to say. */}
        {row.tokens !== null && <span className="text-muted">{row.tokens.toLocaleString()} tokens</span>}
        {row.leftovers > 0 && (
          <span className="text-ink">
            · {row.leftovers} leftover {row.leftovers === 1 ? "process" : "processes"}
          </span>
        )}
        {when && <span className="w-full text-muted" data-testid="summary-when">{when}</span>}
      </button>
    </li>
  );
}

/** `now`: the clock its waits are read against (Home's); without it, when
 * the summary was built. */
export function MorningSummary({ onOpenSession, now }: { onOpenSession: (id: string) => void; now?: number }): JSX.Element | null {
  const kept: OvernightSummaryInfo | undefined = useSharedRead(summaryRead);
  // Marked seen: gone at once, whatever the read behind it still holds.
  const [seenAt, setSeenAt] = useState<number | null>(null);
  const summary = kept && (seenAt === null || kept.generated_ms > seenAt) ? kept : null;
  // The groups opened past their first few rows.
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());
  if (!summary || summary.rows.length === 0) return null;
  const headline = summaryHeadline(summary.counts);
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
        <span className="text-label text-muted">since {since(summary.since_ms)}</span>
        <span className="flex-1" />
        <button
          type="button"
          onClick={() => {
            setSeenAt(summary.generated_ms);
            void overnightSummarySeen().then(() => summaryRead.refresh());
          }}
          className="rounded border border-hairline px-2 py-0.5 text-label text-ink"
          data-testid="summary-seen"
        >
          Mark all seen
        </button>
      </div>
      <p className="text-label text-ink" data-testid="summary-headline">
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
            <p className="text-section font-semibold text-ink">
              <span aria-hidden="true">{group.icon} </span>
              {group.title} ({rows.length})
            </p>
            <ul className="flex flex-col gap-1" aria-label={group.title}>
              {(open.has(group.id) ? rows : rows.slice(0, SHOWN)).map((row) => (
                <Row key={row.session_id} row={row} now={now ?? summary.generated_ms} onOpen={onOpenSession} />
              ))}
            </ul>
            {/* Walk K1: a long night is a few rows and the rest a click
                away, never a wall of them. */}
            {rows.length > SHOWN && !open.has(group.id) && (
              <button
                type="button"
                onClick={() => setOpen((shown) => new Set(shown).add(group.id))}
                className="self-start rounded text-label text-muted underline-offset-2 hover:text-ink hover:underline"
                data-testid="summary-show-all"
              >
                Show all {rows.length}
              </button>
            )}
          </div>
        );
      })}
    </section>
  );
}

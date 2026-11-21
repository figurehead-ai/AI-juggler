/**
 * AJ-E1004 — the morning summary on the home view: what happened since
 * the user last looked (its heading says when: "Since you last looked,
 * 01:56", D-97), severity-ordered (crashed → needs you → still
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
 *
 * 0.4.5 (persona D3): every row leads with the session's own name, by the
 * rule every list names sessions ([`namesApart`]: its name, its
 * conversation's title or first ask, its branch, else "<folder> ·
 * started …", alike ones told apart); its App follows. The rows named
 * only their App, so fifty of them read alike.
 *
 * 0.4.5 (persona D3, runs 3 and 4): Home lists only what changed in its
 * window, and nothing on it said that search finds the rest. When the
 * rail holds a session older than the card's window, the card's foot says
 * "Older sessions: search ⌘K", and a click runs ⌘K.
 */
import { useEffect, useState, type JSX } from "react";
import { overnightSummary, overnightSummarySeen } from "../lib/ipc/park";
import { sharedRead, useSharedRead } from "../lib/shared-read";
import { waitingWords } from "../lib/waiting";
import { summaryHeadline } from "../lib/ended-how";
import type { OvernightSummaryInfo } from "../types/generated/OvernightSummaryInfo";
import type { SummaryRowInfo } from "../types/generated/SummaryRowInfo";
import { sessionAppName } from "../lib/app-name";
import { namesApart } from "../lib/session-titles";
import { onConversationsChanged } from "../lib/ipc/conversations";
import { coalesced } from "../lib/shown-window";
import { pastWhen } from "../lib/time-words";
import { useKeys } from "../lib/use-keys";
import type { RailItem } from "../lib/rail-items";

/** Rows a group shows before "Show all". */
const SHOWN = 5;

/** The card's quiet links: "Show all N", "Older sessions: search ⌘K". */
const LINK = "self-start rounded text-label text-muted underline-offset-2 hover:text-ink hover:underline";

const GROUPS: Array<{ id: string; title: string; icon: string }> = [
  { id: "crashed", title: "Crashed", icon: "✕" },
  { id: "error", title: "Error", icon: "✕" },
  { id: "needs_you", title: "Needs you", icon: "!" },
  // AJ-E2312 1 (PM5): apart from Needs you, as Home lists it ("Needs you
  // (1)" read under Home's "All clear").
  { id: "parked", title: "Parked until a reset", icon: "⏸" },
  { id: "running", title: "Still running", icon: "▶" },
  { id: "finished", title: "Finished", icon: "✓" },
  { id: "stopped", title: "Stopped by you", icon: "■" },
  { id: "switched", title: "Stopped by a switch", icon: "■" },
  { id: "ended", title: "Ended", icon: "■" },
];


/** Its time by the one rule every started and ended time follows (P3):
 * "6:20 PM" today, "yesterday 6:20 PM", "Mon 6:20 PM" (the 0.4.3 walk read
 * "ended Thu 06:20 PM" on a Thursday). */
function since(ms: number): string {
  return pastWhen(ms);
}

/** D-97 (5): the digest's start, plain: "Since you last looked, 01:56"
 * (its day too when it wasn't the day the summary was built); with no
 * last look kept, the default window in words ("In the last 12 hours"). */
export function sinceHeading(summary: Pick<OvernightSummaryInfo, "since_ms" | "generated_ms" | "default_window">): string {
  if (summary.default_window) {
    const hours = Math.max(1, Math.round((summary.generated_ms - summary.since_ms) / 3_600_000));
    return `In the last ${hours} ${hours === 1 ? "hour" : "hours"}`;
  }
  const start = new Date(summary.since_ms);
  const sameDay = start.toDateString() === new Date(summary.generated_ms).toDateString();
  const time = start.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return `Since you last looked, ${sameDay ? time : since(summary.since_ms)}`;
}

/** The summary, kept between Home's openings (lib/shared-read). */
const summaryRead = sharedRead(() => overnightSummary());

const ENDED_GROUPS = new Set(["crashed", "error", "finished", "stopped", "switched", "ended"]);

/** A time a row can say: a real epoch ms. A row without one (`null`, or
 * a read that left it off) leaves the time out: never "NaN d" or
 * "Invalid Date" (0.4.1 baseline pass). */
function known(ms: number | null | undefined): ms is number {
  return typeof ms === "number" && Number.isFinite(ms);
}

/** When, under a row: how long it has waited, when it ended, else its
 * last change; nothing when it has no time to say. */
export function whenLine(row: SummaryRowInfo, now: number): string | null {
  if (known(row.at_ms) && (row.group === "needs_you" || row.group === "parked")) return waitingWords(row.at_ms, now);
  if (known(row.at_ms) && ENDED_GROUPS.has(row.group)) return `ended ${since(row.at_ms)}`;
  const last = [...row.transitions].reverse().find((step) => known(step.at_ms));
  return last ? `last change ${since(last.at_ms)}` : null;
}

/** A session as the rail holds it, as far as the card's foot asks. */
export type HeldSession = Pick<RailItem, "lastMs" | "archived"> & { session: { id: string } | null };

/** Whether the rail holds a session older than the card's window: last
 * active before the window starts, not on the card, not put away. Only
 * search finds those from Home (persona D3). */
export function olderThanCard(held: readonly HeldSession[], summary: Pick<OvernightSummaryInfo, "since_ms" | "rows">): boolean {
  const onCard = new Set(summary.rows.map((row) => row.session_id));
  return held.some((item) => !item.archived && item.lastMs < summary.since_ms && !(item.session && onCard.has(item.session.id)));
}

function Row({ row, name, now, onOpen }: { row: SummaryRowInfo; name: string; now: number; onOpen: (id: string) => void }): JSX.Element {
  const when = whenLine(row, now);
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpen(row.session_id)}
        className="flex w-full flex-col gap-0.5 rounded-lg border border-hairline px-2.5 py-1.5 text-left text-label hover:border-accent/40 hover:bg-surface"
        data-testid="summary-row"
        data-group={row.group}
      >
        {/* Its name on a line of its own, as "Pick up where you left off"
            names a session; what it ran in and how it went under it (a
            long name pushed "· Finished" onto a line alone). */}
        <span className="w-full truncate font-medium text-ink" data-testid="summary-name" title={name}>
          {name}
        </span>
        <span className="flex w-full flex-wrap items-center gap-x-2 gap-y-0.5">
          {/* Its App by its full name (the 0.4 review). */}
          <span className="text-muted" data-testid="summary-app">{sessionAppName(row)}</span>
          {row.project && <span className="text-muted">· {row.project}</span>}
          <span className="text-ink">· {row.detail ? `${row.words} · ${row.detail}` : row.words}</span>
          {when && (
            <span className="text-muted">
              · <span data-testid="summary-when">{when}</span>
            </span>
          )}
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
        </span>
      </button>
    </li>
  );
}

/** `now`: the clock its waits are read against (Home's); without it, when
 * the summary was built. `held`: the sessions the rail holds (Home's,
 * already read), and `onSearch` ⌘K's search: with both, the card's foot
 * points to older sessions when there are any. */
export function MorningSummary({
  onOpenSession,
  now,
  held,
  onSearch,
}: {
  onOpenSession: (id: string) => void;
  now?: number;
  held?: readonly HeldSession[];
  onSearch?: () => void;
}): JSX.Element | null {
  const kept: OvernightSummaryInfo | undefined = useSharedRead(summaryRead);
  const searchKeys = useKeys("palette");
  // Marked seen: gone at once, whatever the read behind it still holds.
  const [seenAt, setSeenAt] = useState<number | null>(null);
  const summary = kept && (seenAt === null || kept.generated_ms > seenAt) ? kept : null;
  // The groups opened past their first few rows.
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());
  // 0.4.5: a session's name can come after the card's first read (the
  // conversation pass's first run at launch; a title a tool writes later),
  // and the card read only when Home opened, so its rows stayed
  // "<folder> · started …" while Pick up named them. It reads again when
  // the conversation index changes (not on each live turn), at most every
  // 5 s, and only while the window shows.
  useEffect(() => {
    const again = coalesced(() => void summaryRead.refresh(), 5_000);
    const stop = onConversationsChanged((change) => {
      if (change === "index") again.ask();
    });
    return () => {
      stop();
      again.stop();
    };
  }, []);
  if (!summary || summary.rows.length === 0) return null;
  const headline = summaryHeadline(summary.counts);
  const at = now ?? summary.generated_ms;
  const names = namesApart(summary.rows.map((row) => ({ ...row, id: row.session_id })), at);
  return (
    <section
      aria-labelledby="summary-title"
      className="flex w-full max-w-2xl flex-col gap-2 rounded-xl border border-hairline bg-surface p-4"
      data-testid="morning-summary"
    >
      <div className="flex flex-wrap items-baseline gap-2">
        <h2 id="summary-title" className="text-sm font-semibold" data-testid="summary-since">
          {sinceHeading(summary)}
        </h2>
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
                <Row key={row.session_id} row={row} name={names.get(row.session_id) ?? ""} now={at} onOpen={onOpenSession} />
              ))}
            </ul>
            {/* Walk K1: a long night is a few rows and the rest a click
                away, never a wall of them. */}
            {rows.length > SHOWN && !open.has(group.id) && (
              <button
                type="button"
                onClick={() => setOpen((shown) => new Set(shown).add(group.id))}
                className={LINK}
                data-testid="summary-show-all"
              >
                Show all {rows.length}
              </button>
            )}
          </div>
        );
      })}
      {/* Persona D3: the card holds only its window; search finds the rest. */}
      {onSearch && held && olderThanCard(held, summary) && (
        <button type="button" onClick={onSearch} className={LINK} aria-keyshortcuts={searchKeys.aria} data-testid="summary-older">
          {searchKeys.words ? `Older sessions: search ${searchKeys.words}` : "Older sessions: search"}
        </button>
      )}
    </section>
  );
}

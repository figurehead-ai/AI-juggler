/**
 * AJ-E2503 (§4.14, wireframe 27; the approved design of 2026-10-27) — the
 * notification centre, from the bell or ⌥⌘N. **Needs action** mirrors
 * Needs you (the same items, the same count; snoozed ones folded under
 * "Snoozed"); **Earlier** keeps what happened — resets, finishes, crashes
 * and restarts, updates — 30 days, a page at a time. Mark all read, the
 * day's digest, and the way to Notification settings. Empty, it says
 * "You're all caught up."; in the quiet hours, what is muted and until
 * when. A labelled dialog: focus moves in, Tab stays inside, Esc closes
 * and focus goes back to the bell. Nothing here acts by itself.
 */
import { useEffect, useId, useRef, useState, type JSX } from "react";
import { keepTabInside } from "./form";
import { NoticeRow } from "./NoticeRow";
import { RowPlaceholders } from "./Placeholder";
import { usePagedList } from "../lib/paged";
import { notificationsPage, notificationsRead, type NoticeInfo, type NoticeSummaryInfo } from "../lib/ipc/notices";
import { acted } from "../lib/notices-store";
import { useNoticeActions } from "../lib/notice-actions";
import { digestLine, mutedLine } from "../lib/notice-words";
import type { InstanceInfo } from "../lib/bridge";

/** Earlier notices read at a time. */
const EARLIER_PAGE = 20;

export function NotificationCentre({ summary, sessions, onClose }: { summary: NoticeSummaryInfo | null; sessions: InstanceInfo[]; onClose: () => void }): JSX.Element {
  const titleId = useId();
  const ref = useRef<HTMLDivElement | null>(null);
  const actions = useNoticeActions(sessions);
  const needs = usePagedList<NoticeInfo>("notices:needs", (request) => notificationsPage("needs", request), { keyOf: (row) => String(row.id), pageSize: 200 });
  const earlier = usePagedList<NoticeInfo>("notices:earlier", (request) => notificationsPage("earlier", request), { keyOf: (row) => String(row.id), pageSize: EARLIER_PAGE });
  const [showSnoozed, setShowSnoozed] = useState(false);
  const [now] = useState(() => Date.now());
  // Focus in on open; back to what opened it on close.
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      if (before && document.contains(before)) before.focus();
    };
  }, []);
  // A click outside closes it (the bell toggles it itself).
  useEffect(() => {
    const outside = (event: MouseEvent) => {
      const target = event.target as Element | null;
      if (ref.current?.contains(target) || target?.closest?.("[data-notification-bell]") || target?.closest?.('[role="menu"]')) return;
      onClose();
    };
    window.addEventListener("mousedown", outside);
    return () => window.removeEventListener("mousedown", outside);
  }, [onClose]);
  const awake = needs.items.filter((row) => !row.snoozed);
  const snoozed = needs.items.filter((row) => row.snoozed);
  // The 0.2 walk (F4): "Needs action · 2" over three rows. The heading
  // counts the rows under it, read in the same answer.
  const count = awake.length;
  const digest = summary ? digestLine(summary.digest, now) : null;
  const empty = !needs.loading && !earlier.loading && needs.items.length === 0 && earlier.items.length === 0;
  const unread = (summary?.unread ?? 0) > 0;
  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabIndex={-1}
      data-testid="notification-centre"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !document.querySelector('[role="menu"]')) {
          event.stopPropagation();
          onClose();
        }
        keepTabInside(ref.current, event);
      }}
      className="absolute right-0 top-full z-40 mt-2 flex max-h-[min(640px,calc(100vh-9rem))] w-[min(440px,calc(100vw-2rem))] flex-col overflow-hidden rounded-xl border border-hairline bg-surface-2 shadow-card outline-none"
    >
      <div className="flex items-center gap-2 border-b border-hairline px-3.5 py-3">
        <h2 id={titleId} className="text-section font-semibold text-ink">
          Notifications
        </h2>
        <span className="flex-1" />
        <button type="button" disabled={!unread} onClick={() => void acted(() => notificationsRead(null))} className="rounded px-1 text-meta font-medium text-accent-hi hover:underline disabled:text-muted disabled:no-underline" data-testid="notices-read-all">
          Mark all read
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {summary?.quiet && summary.quiet_until && (
          <p className="border-b border-hairline/50 bg-surface px-3.5 py-2 text-meta text-muted" data-testid="notices-muted">
            {mutedLine(summary.quiet_until, summary.count)}
          </p>
        )}
        {empty ? (
          <div className="flex flex-col items-center gap-1 px-6 py-10 text-center" data-testid="notices-empty">
            <p className="text-body font-medium text-ink">You're all caught up.</p>
            <p className="text-meta text-muted">What needs you, and what happened, shows up here.</p>
          </div>
        ) : (
          <>
            <section aria-labelledby={`${titleId}-needs`}>
              <h3 id={`${titleId}-needs`} className="flex items-center justify-between px-3.5 pb-1 pt-2.5 text-label font-semibold uppercase tracking-wide text-muted">
                <span data-testid="notices-needs-heading">{needs.loading ? "Needs action" : `Needs action · ${count}`}</span>
                {snoozed.length > 0 && (
                  <button type="button" aria-expanded={showSnoozed} onClick={() => setShowSnoozed((open) => !open)} className="rounded px-1 font-medium normal-case tracking-normal text-muted hover:text-ink" data-testid="notices-snoozed">
                    Snoozed: {snoozed.length}
                  </button>
                )}
              </h3>
              {needs.loading ? (
                <RowPlaceholders shape="home" rows={3} label="Loading notifications" />
              ) : awake.length === 0 && !showSnoozed ? (
                <p className="px-3.5 pb-2.5 text-meta text-muted">Nothing needs you right now.</p>
              ) : (
                <ul data-testid="notices-needs">
                  {[...awake, ...(showSnoozed ? snoozed : [])].map((notice) => (
                    <NoticeRow key={notice.id} notice={notice} actions={actions} now={now} />
                  ))}
                </ul>
              )}
            </section>
            {earlier.items.length > 0 && (
              <section aria-labelledby={`${titleId}-earlier`} className="mt-1">
                <h3 id={`${titleId}-earlier`} className="px-3.5 pb-1 pt-2.5 text-label font-semibold uppercase tracking-wide text-muted">
                  Earlier
                </h3>
                <ul data-testid="notices-earlier">
                  {earlier.items.map((notice) => (
                    <NoticeRow key={notice.id} notice={notice} actions={actions} now={now} />
                  ))}
                </ul>
                {earlier.hasMore && (
                  <div className="px-3.5 py-2">
                    <button type="button" onClick={earlier.loadMore} disabled={earlier.loadingMore} className="rounded text-meta font-medium text-accent-hi hover:underline disabled:text-muted" data-testid="notices-more">
                      {earlier.loadingMore ? "Loading…" : "Show earlier"}
                    </button>
                  </div>
                )}
              </section>
            )}
          </>
        )}
      </div>
      <div className="flex items-center gap-3 border-t border-hairline px-3.5 py-2.5 text-label text-muted">
        <span className="min-w-0 flex-1" data-testid="notices-digest">
          {digest ?? "History is kept for 30 days."}
        </span>
        <button
          type="button"
          onClick={() => {
            onClose();
            window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "notifications" } }));
          }}
          className="shrink-0 rounded font-medium text-accent-hi hover:underline"
        >
          Notification settings
        </button>
      </div>
    </div>
  );
}

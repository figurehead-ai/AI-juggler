/**
 * AJ-E611 — the alert/notification region: polite live-region
 * semantics (never assertive — anti-fatigue), badge-carrying copy
 * (Estimated reads differently), and the reset-queue rows with their
 * countdown / ready states + an explicit Resume click (the click is
 * the ONLY action; nothing here launches by itself).
 */
import { useEffect, useState } from "react";
import { alertsTick, resetQueueDismiss, resetQueueList, type AlertEventData, type ResetQueueRow } from "../lib/bridge";

export function NotificationRegion({ onQueueResume }: { onQueueResume?: (row: ResetQueueRow) => void }): JSX.Element {
  const [events, setEvents] = useState<AlertEventData[]>([]);
  const [queue, setQueue] = useState<ResetQueueRow[]>([]);

  useEffect(() => {
    let alive = true;
    const poll = () => {
      alertsTick().then((emitted) => {
        if (alive && emitted.length > 0) setEvents((prior) => [...emitted, ...prior].slice(0, 5));
      });
      resetQueueList().then((rows) => {
        if (alive) setQueue(rows);
      });
    };
    poll();
    const timer = window.setInterval(poll, 30_000); // ≥30 s — never a busy loop
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div aria-live="polite" role="status" data-testid="notification-region" className="flex flex-col gap-1">
      {events.map((event) => (
        <p
          key={`${event.at_ms}-${event.account}-${event.kind}`}
          data-testid={`alert-${event.kind}`}
          className={`rounded border px-2 py-1 text-[11px] ${
            event.kind === "exhausted" || event.kind === "actionable"
              ? "border-amber-500/50 text-amber-500"
              : event.kind === "digest"
                ? "border-dashed border-hairline text-muted"
                : "border-hairline text-muted"
          }`}
        >
          {event.kind === "digest" ? "Digest · " : ""}
          {event.text}
        </p>
      ))}
      {queue.map((row) => (
        <p key={row.id} data-testid={`queue-${row.state}`} className="flex items-center gap-2 rounded border border-hairline px-2 py-1 text-[11px]">
          <span>
            {row.state === "ready"
              ? `reset reached — ${row.account} ready to resume`
              : row.state === "stale"
                ? `queued resume went stale (session ${row.session} is gone)`
                : `queued at ${new Date(row.resume_at_ms).toLocaleTimeString()} — ${row.account}`}
          </span>
          {row.state === "ready" && onQueueResume && (
            <button
              type="button"
              className="underline focus:outline-none focus:ring-2 focus:ring-accent"
              onClick={() => onQueueResume(row)}
            >
              Resume
            </button>
          )}
          <button
            type="button"
            className="text-muted underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => void resetQueueDismiss(row.id).then(() => setQueue((rows) => rows.filter((entry) => entry.id !== row.id)))}
          >
            dismiss
          </button>
        </p>
      ))}
    </div>
  );
}

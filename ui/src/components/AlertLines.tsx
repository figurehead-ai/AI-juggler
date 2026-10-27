/**
 * AJ-E611 + D-25 (wireframe 27): the alert lines and the queued pick-ups
 * in the notification band, in plain words with real actions:
 *
 * - a quota alert says who, how much is left, in which window and until
 *   when (local time), and offers **Hand over** (the limit dialog, for
 *   the account's latest live session) and **Queue for the reset**;
 * - a queued pick-up says the account by name and the time, never an id.
 *
 * Nothing here acts by itself: every action is a click (AJ-E611).
 */
import { useState, type JSX } from "react";
import { resetQueueDismiss, resetQueueQueue, type AlertEventData, type InstanceInfo, type ResetQueueRow } from "../lib/bridge";
import { alertEventWords, untilWords } from "../lib/alert-words";
import { isActiveStatus } from "./indicator";

const LINK = "underline focus:outline-none focus:ring-2 focus:ring-accent";

/** The account's latest live session, the one an action is for. */
function sessionOn(sessions: InstanceInfo[], accountId: string): InstanceInfo | null {
  return (
    sessions
      .filter((session) => session.account_id === accountId && isActiveStatus(session.status))
      .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0] ?? null
  );
}

function AlertLine({ event, sessions, onHandOver }: { event: AlertEventData; sessions: InstanceInfo[]; onHandOver?: (sessionId: string) => void }): JSX.Element {
  const [queued, setQueued] = useState<string | null>(null);
  const warn = event.kind === "exhausted" || event.kind === "actionable";
  const session = warn && event.quota ? sessionOn(sessions, event.account) : null;
  const resetAt = event.quota?.resets_at_ms ?? null;
  return (
    <p
      data-testid={`alert-${event.kind}`}
      className={`flex flex-wrap items-center gap-x-3 gap-y-1 rounded border px-2 py-1 text-label ${
        warn ? "border-warning/50 text-warning" : event.kind === "digest" ? "border-dashed border-hairline text-muted" : "border-hairline text-muted"
      }`}
    >
      <span className="min-w-0 flex-1">
        {event.kind === "digest" ? "Digest · " : ""}
        {alertEventWords(event)}
        {queued && <span className="text-ink"> · {queued}</span>}
      </span>
      {session && onHandOver && (
        <button type="button" className={LINK} onClick={() => onHandOver(session.id)} data-testid="alert-hand-over">
          Hand over
        </button>
      )}
      {session && resetAt !== null && !queued && (
        <button
          type="button"
          className={LINK}
          data-testid="alert-queue"
          onClick={() =>
            void resetQueueQueue(session.id, event.account, resetAt).then(
              () => setQueued(`it picks up again at ${untilWords(resetAt)}, when you choose Resume`),
              () => setQueued("couldn't queue it; try again"),
            )
          }
        >
          Queue for the reset
        </button>
      )}
    </p>
  );
}

export function AlertLines({ events, sessions = [], onHandOver }: { events: AlertEventData[]; sessions?: InstanceInfo[]; onHandOver?: (sessionId: string) => void }): JSX.Element {
  return (
    <>
      {events.map((event) => (
        <AlertLine key={`${event.at_ms}-${event.account}-${event.kind}`} event={event} sessions={sessions} onHandOver={onHandOver} />
      ))}
    </>
  );
}

/** A queued pick-up, by the account's name (from its sessions). */
export function QueueLines({
  queue,
  sessions = [],
  onResume,
  onDismissed,
}: {
  queue: ResetQueueRow[];
  sessions?: InstanceInfo[];
  onResume?: (row: ResetQueueRow) => void;
  onDismissed: (id: string) => void;
}): JSX.Element {
  const label = (row: ResetQueueRow) => sessions.find((session) => session.id === row.session || session.account_id === row.account)?.account_label ?? "Its account";
  return (
    <>
      {queue.map((row) => (
        <p key={row.id} data-testid={`queue-${row.state}`} className="flex items-center gap-2 rounded border border-hairline px-2 py-1 text-label">
          <span>
            {row.state === "ready"
              ? `${label(row)} has reset: its session can pick up again`
              : row.state === "stale"
                ? "A queued pick-up was dropped: its session is gone"
                : `${label(row)}'s session picks up again at ${untilWords(row.resume_at_ms)}, when you choose Resume`}
          </span>
          {row.state === "ready" && onResume && (
            <button type="button" className={LINK} onClick={() => onResume(row)}>
              Resume
            </button>
          )}
          <button type="button" className={`text-muted ${LINK}`} onClick={() => void resetQueueDismiss(row.id).then(() => onDismissed(row.id))}>
            dismiss
          </button>
        </p>
      ))}
    </>
  );
}

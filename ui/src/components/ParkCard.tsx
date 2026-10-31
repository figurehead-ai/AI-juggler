/**
 * AJ-E1003 — parking a quota-blocked Instance until its window resets,
 * in the inspector. Park stages its context; at the reset the user gets
 * one offer to resume the same conversation on the same Account. A
 * Measured reset shows a countdown; an Estimated one says so and offers
 * "Check now" instead. Nothing here resumes by itself.
 */
import { useState, type JSX } from "react";
import { errorText, instancePark, parkedCheckNow, parkedResume, type InstanceInfo } from "../lib/bridge";
import { parkedWords } from "../lib/parked";

export function ParkCard({
  session,
  now,
  onChanged,
}: {
  session: InstanceInfo;
  now: number;
  onChanged?: () => void;
}): JSX.Element | null {
  const [note, setNote] = useState<string | null>(null);
  const words = parkedWords(session, now);
  if (!words && session.status !== "quota_blocked") return null;
  const act = async (action: () => Promise<unknown>, done: string) => {
    try {
      await action();
      setNote(done);
      onChanged?.();
    } catch (cause) {
      setNote(errorText(cause));
    }
  };
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border border-hairline px-2.5 py-2 text-label" data-testid="park-card">
      {words ? (
        <>
          <span className="min-w-0 flex-1 text-ink" data-testid="park-state">
            <span aria-hidden="true">⏸ </span>
            {words.long}
          </span>
          {words.estimated && (
            <button
              type="button"
              onClick={() => void act(() => parkedCheckNow(session.id), "Checked — the reset time is current.")}
              className="rounded border border-hairline px-2 py-0.5 text-ink"
              data-testid="park-check"
            >
              Check now
            </button>
          )}
          {words.due && (
            <button
              type="button"
              onClick={() => void act(() => parkedResume(session.id), "Resuming the same conversation…")}
              className="rounded border border-accent/40 bg-accent-soft px-2 py-0.5 text-accent-hi"
              data-testid="park-resume"
            >
              Resume the same conversation
            </button>
          )}
        </>
      ) : (
        <>
          <span className="min-w-0 flex-1 text-ink">
            Blocked by its quota. Park it until the reset — its context is saved, and you'll be offered the same
            conversation back once, when the quota resets.
          </span>
          <button
            type="button"
            onClick={() => void act(() => instancePark(session.id), "Parked until the reset.")}
            className="rounded border border-accent/40 bg-accent-soft px-2 py-0.5 text-accent-hi"
            data-testid="park-go"
          >
            Park until reset
          </button>
        </>
      )}
      {note && (
        <p className="w-full text-ink-2" aria-live="polite">
          {note}
        </p>
      )}
    </div>
  );
}

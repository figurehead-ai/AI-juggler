/**
 * AJ-E1003 — parking a quota-blocked Instance until its window resets,
 * in the inspector. Park stages its context; at the reset the user gets
 * one offer to resume the same conversation on the same Account. A
 * Measured reset shows a countdown; an Estimated one says so and offers
 * "Check now" instead. Nothing here resumes by itself.
 *
 * AJ-E2312: one at its limit that wasn't parked is told its limit reset
 * too, with the same way on (Resume the same conversation, on the same
 * Account), never "Blocked by its quota" past its reset (item 1). Its
 * words keep a sentence's width beside the button, which goes under them
 * where the card is narrow: never one word per line (item 2, the PM5 walk).
 */
import { useState, type JSX } from "react";
import { errorText } from "../lib/bridge";
import { instancePark, parkedCheckNow, parkedResume } from "../lib/ipc/park";
import { sessionResume } from "../lib/ipc/launch";
import { accountNameOf, useSubscriptions } from "../lib/subscriptions";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { parkedWords } from "../lib/parked";

/** The card's words beside its buttons: a sentence's width at least
 * (14rem), so in a wrapping row the button goes under them before they
 * break a word a line (a basis of 0, `flex-1` alone, gave them none). */
export const PARK_WORDS = "min-w-0 flex-1 basis-56 text-ink";

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
  const subscriptions = useSubscriptions();
  const words = parkedWords(session, now);
  // AJ-E2312 1: its limit has reset since it stopped there (the backend's
  // one word for it, read the same by every view).
  const reset = !words && session.needs_attention === "limit_reset";
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
          <span className={PARK_WORDS} data-testid="park-state">
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
      ) : reset ? (
        <>
          <span className={PARK_WORDS} data-testid="park-state">
            {accountNameOf(subscriptions, session.account_id, session.account_label)} has reset. Resume its work — the same conversation, on the same
            Account.
          </span>
          <button
            type="button"
            onClick={() => void act(() => sessionResume(session.id), "Resuming the same conversation…")}
            className="rounded border border-accent/40 bg-accent-soft px-2 py-0.5 text-accent-hi"
            data-testid="park-resume"
          >
            Resume the same conversation
          </button>
        </>
      ) : (
        <>
          <span className={PARK_WORDS}>
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

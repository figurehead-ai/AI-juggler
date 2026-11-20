/**
 * E18 — messages in plain words: a message's kind, where it is for each
 * recipient, and a thread's state. One vocabulary for the pool's and the
 * session's message views and Needs you.
 */

export const KIND_WORDS: Record<string, string> = {
  note: "Note",
  question: "Question",
  answer: "Answer",
  handoff: "Handoff",
  review_request: "Review request",
  status: "Status",
};

/** The kinds you can pick when you write, in the order offered. */
export const KINDS = ["note", "question", "handoff", "review_request", "answer", "status"] as const;

export const RECIPIENT_WORDS: Record<string, string> = {
  unread: "Waiting in its inbox",
  read: "Read",
  acked: "Handled",
  held: "Waits on your OK",
  queued: "Goes in at its next turn",
  delivered: "Started its turn",
  refused: "Not sent",
};

/** You, as a recipient, read it or not. */
export function recipientWords(state: string, name: string): string {
  if (name === "You") return state === "unread" ? "New for you" : "To you";
  return RECIPIENT_WORDS[state] ?? state;
}

export const THREAD_WORDS: Record<string, string> = {
  open: "Open",
  paused: "Paused",
  stopped: "Stopped",
};

/** A thread's count against its cap (F-rc3g-10: its pool's hop limit when
 * the pool puts messages into sessions, never more than 6), as Needs you
 * says it: "2 of 2 agent messages in a row without you" is the last one
 * before you're asked; past it, one more waits for you. Only when it is
 * getting close. */
export function hopWords(hop: number, cap: number): string | null {
  if (hop < 2 && hop < cap) return null;
  // AJ-E3014 item 3: past the limit, counted as the limit ("N of 2"), as
  // Needs you and the agents say it.
  if (hop > cap) return `${cap} of ${cap} agent messages in a row without you; one more waits for you`;
  return `${hop} of ${cap} agent messages in a row without you`;
}

/** An answer to a held message (`deliver`: approved and put into its
 * recipients' sessions at once; `once`: approved for their next turn). */
export type HeldAnswer = "deliver" | "once" | "always" | "deny";

/** D-139: a held message's answers, one list for every surface that
 * offers them (Needs you, its thread, the pool's diagram). One press
 * approves and delivers it, "Allow and deliver"; "Allow, deliver later"
 * leaves it in the recipient's inbox for its next turn. Where it can't go
 * into a session now (D-127: a terminal AI Juggler can't type into, or
 * another app's session), "Allow once" stands for both. "Always allow"
 * from the second time; "Deny". */
export function heldChoices(
  need: { delivers_now?: boolean | null; offer_always: boolean },
  always = "Always allow in this pool",
): { id: HeldAnswer; label: string }[] {
  const allow: { id: HeldAnswer; label: string }[] = need.delivers_now
    ? [
        { id: "deliver", label: "Allow and deliver" },
        { id: "once", label: "Allow, deliver later" },
      ]
    : [{ id: "once", label: "Allow once" }];
  return [...allow, ...(need.offer_always ? [{ id: "always" as const, label: always }] : []), { id: "deny", label: "Deny" }];
}

/** A message's time: today's as a clock, older ones with the day. */
export function when(ms: number, now: number): string {
  const date = new Date(ms);
  const sameDay = new Date(now).toDateString() === date.toDateString();
  return sameDay
    ? date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : date.toLocaleDateString([], { month: "short", day: "numeric" });
}

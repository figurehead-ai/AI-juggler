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

/** "3 agent messages in a row of 6" — only when it is getting close. */
export function hopWords(hop: number, cap: number): string | null {
  if (hop < 2) return null;
  return `${hop} of ${cap} agent messages in a row without you`;
}

/** A message's time: today's as a clock, older ones with the day. */
export function when(ms: number, now: number): string {
  const date = new Date(ms);
  const sameDay = new Date(now).toDateString() === date.toDateString();
  return sameDay
    ? date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : date.toLocaleDateString([], { month: "short", day: "numeric" });
}

/** A byte size in words. */
export function sizeWords(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  return `${Math.round(bytes / 102.4) / 10} KB`;
}

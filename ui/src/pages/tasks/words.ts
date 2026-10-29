/**
 * E19 — tasks in plain words: a status, a kind, a priority, a link, how a
 * session came to a task. One vocabulary for the pool's Tasks tab, a
 * session's tasks, Needs you and the switch history.
 */

export const STATUS_WORDS: Record<string, string> = {
  ready: "Ready",
  in_progress: "In progress",
  in_review: "In review",
  blocked: "Blocked",
  todo: "To do",
  done: "Done",
  cancelled: "Cancelled",
};

/** The statuses in the order a list shows them: ready work first. */
export const STATUSES = ["ready", "in_progress", "in_review", "blocked", "todo", "done", "cancelled"] as const;

/** Each status's tone (the same tones as the rail's marks). */
export const STATUS_TONE: Record<string, "accent" | "live" | "warning" | "danger" | "neutral" | "success"> = {
  ready: "accent",
  in_progress: "live",
  in_review: "warning",
  blocked: "danger",
  todo: "neutral",
  done: "success",
  cancelled: "neutral",
};

export const KIND_WORDS: Record<string, string> = { epic: "Epic", story: "Story", task: "Task" };

export const PRIORITY_WORDS: Record<string, string> = { urgent: "Urgent", high: "High", normal: "Normal", low: "Low" };
export const PRIORITIES = ["urgent", "high", "normal", "low"] as const;

export const LINK_WORDS: Record<string, string> = {
  blocks: "Blocks",
  blocked_by: "Waits on",
  relates_to: "Relates to",
  duplicates: "Duplicates",
  duplicated_by: "Duplicated by",
};

export const HOW_WORDS: Record<string, string> = {
  claimed: "took it",
  started_for: "started for it",
  assigned: "was given it",
  worked: "worked on it",
  // AJ-E1904: a switch carried it to this session.
  switched: "continued it after a switch",
};

/** Where a task is, for a sentence ("in progress with Builder"). */
export function holderWords(status: string, holder: string | null): string {
  const word = (STATUS_WORDS[status] ?? status).toLowerCase();
  if (!holder) return status === "ready" ? "ready for anyone" : word;
  return `${word} · ${holder === "You" ? "yours" : holder}`;
}

/** How many items under an epic or story are done. */
export function itemsWords(items: number, done: number): string | null {
  if (items === 0) return null;
  return `${done} of ${items} done`;
}

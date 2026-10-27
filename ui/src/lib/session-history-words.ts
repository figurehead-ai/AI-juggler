/**
 * wf09 — a session's history in words: "Started on Claude Code · Work",
 * "Switched to Personal (Codex)", "Archived". Never a status id
 * (`quota_blocked`) or an edge state (D-25).
 */
import type { SessionEventInfo } from "./ipc/sessions";
import { toolSpec } from "./tools";

/** A status it moved to, as something that happened. */
const STATUS_EVENT: Record<string, string> = {
  starting: "Started",
  running: "Working",
  waiting_user: "Asked you something",
  stuck: "Looked stuck",
  quota_blocked: "Hit its usage limit",
  finished: "Ended",
  stopped: "Stopped",
  failed: "Stopped with an error",
  handed_over: "Handed over to another account",
};

/** A lifecycle step's id, in words; a step already in words stays. */
const LIFECYCLE: Record<string, string> = {
  archived: "Archived",
  unarchived: "Brought back from the archive",
  reviewed: "Marked reviewed",
  unreviewed: "Marked not reviewed",
};

function sentence(text: string): string {
  const trimmed = text.trim();
  return trimmed.length === 0 ? "Something happened" : trimmed[0]!.toUpperCase() + trimmed.slice(1);
}

/** Where the session started: its tool and account. */
export type HistoryOrigin = { provider: string; account: string };

/** One event's words. The very first status says where it started. */
export function historyWords(event: SessionEventInfo, first: boolean, origin: HistoryOrigin): string {
  if (event.kind === "switch" && event.switch) {
    const step = event.switch;
    const where = `${step.other_account} (${toolSpec(step.other_provider).display_name})`;
    const head = step.direction === "in" ? `Arrived from ${where}` : `Switched to ${where}`;
    if (step.state === "failed") return `${head}: didn't complete`;
    if (step.state === "staged" || step.state === "verified") return `${head}: getting ready`;
    return head;
  }
  if (event.kind === "lifecycle") return LIFECYCLE[event.what] ?? sentence(event.what);
  if (first && (event.what === "starting" || event.what === "running")) {
    return `Started on ${toolSpec(origin.provider).display_name} · ${origin.account}`;
  }
  return STATUS_EVENT[event.what] ?? "Changed state";
}

/** When, as the table says it: "27 Oct, 09:12". */
export function historyWhen(atMs: number): string {
  return new Date(atMs).toLocaleString([], { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

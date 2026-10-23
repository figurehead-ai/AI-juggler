/**
 * AJ-E1503 Req.2 — stopping one session is held for the undo window,
 * then sent: Undo in time and the stop never reaches the session. Every
 * single-session Stop (the inspector's button and Actions row, ⌘.) comes
 * through here. "Stop all" is different: it asks first and cannot be
 * undone (the maintainer's plan, 2026-10-21).
 */
import { holdAction } from "../components/Toast";
import { terminalKill, type InstanceInfo, type ProjectInfo } from "./bridge";

/** The undo window for a held stop, in seconds. */
export const STOP_HOLD_SECONDS = 5;

/** A session in words: its project and account ("api · Work Max"). */
export function sessionLabel(session: InstanceInfo, projects: ProjectInfo[]): string {
  const project = projects.find((row) => row.id === session.project_id)?.name;
  return project ? `${project} · ${session.account_label}` : session.account_label;
}

/** Holds the stop of one session; `after` runs once it is sent. */
export function holdStop(session: { id: string; label: string }, after?: () => void): void {
  holdAction({
    id: `stop:${session.id}`,
    message: `Stopping ${session.label}.`,
    detail: "Undo keeps it running.",
    seconds: STOP_HOLD_SECONDS,
    testId: "stop-undo",
    undone: `${session.label} keeps running.`,
    failed: (cause) => `Couldn’t stop ${session.label}: ${cause}`,
    run: async () => {
      await terminalKill(session.id);
      after?.();
    },
  });
}

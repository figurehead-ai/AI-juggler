/**
 * AJ-E3510: the holder the Tasks screen opens with when a door asks for
 * it (the Floor's whiteboard: `app:<id>`, an App's tasks). One-shot: the
 * screen takes it on opening, or at once when it is already open. Kept
 * apart from the screen so the shell doesn't load the screen early.
 */
let pending: string | null = null;

/** Fired when a holder waits to be taken. */
export const TASKS_HOLDER_EVENT = "aij:tasks-holder";

/** Asks the Tasks screen to show what `holder` holds. */
export function showTasksHeldBy(holder: string): void {
  pending = holder;
  window.dispatchEvent(new CustomEvent(TASKS_HOLDER_EVENT));
}

/** The holder waiting, if any (taken: the next call says none). */
export function takeTasksHolder(): string | null {
  const holder = pending;
  pending = null;
  return holder;
}

/**
 * E19 — where a task links out to views other features own. A task shows
 * its sessions and its messages itself; its processes are the Processes
 * view's (D-41, built in `e24-processes`), so the task shows a Processes
 * link only once that view says it's there, through this hook. Until
 * then nothing shows: no link to a view that doesn't exist.
 */

/** A link from a task's session to its processes. */
export interface ProcessesLink {
  /** The words on it ("Its processes"). */
  label: string;
  /** Opens the session's processes. */
  open: () => void;
}

type Provider = (sessionId: string) => ProcessesLink | null;

let provider: Provider | null = null;

/** The Processes view registers how to open a session's processes
 * (`null` takes it back). */
export function provideProcessesLink(next: Provider | null): void {
  provider = next;
}

/** A session's processes link, when the Processes view is there. */
export function processesLinkFor(sessionId: string): ProcessesLink | null {
  return provider?.(sessionId) ?? null;
}

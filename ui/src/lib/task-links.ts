/**
 * E19 — where a task links out to views other features own. A task shows
 * its sessions and its messages itself; its processes are the Processes
 * view's (D-41), so the task shows a Processes link only once that view
 * says it's there, through this hook: the session page that holds the
 * Processes tab registers it (`RightInspector`). The task view opens the
 * holder's session after `open` picks the tab. Without a provider nothing
 * shows: no link to a view that isn't there.
 */

/** A link from a task's session to its processes. */
export interface ProcessesLink {
  /** The words on it ("Its processes"). */
  label: string;
  /** Picks the session's Processes tab (the task view then opens the
   * session). */
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

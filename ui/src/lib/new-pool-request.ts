/**
 * New pool, asked for from elsewhere (a project's "New pool for web…", its
 * "Keep it apart…"): the Pools screen opens its setup on that request.
 */
/** A new pool to set up from elsewhere (the 0.4.1 persona walk: a
 * project's "Keep it apart…" opens the sealed template on it): the Pools
 * screen takes it once and opens its wizard. */
export interface NewPoolRequest {
  template: string;
  project: string | null;
}
let newPoolRequest: { request: NewPoolRequest; at: number } | null = null;
const newPoolListeners = new Set<() => void>();

/** How long a request waits for the Pools screen to open it (round 6 #4:
 * in Casey's run "New pool for web…" landed on the Pools list; a screen
 * mounted twice took it once, then showed the list). */
const REQUEST_WAITS_MS = 5_000;

export function requestNewPool(request: NewPoolRequest): void {
  newPoolRequest = { request, at: Date.now() };
  for (const listener of newPoolListeners) listener();
}

/** The waiting request, if one is waiting; it stays until the setup it
 * opened closes ({@link doneNewPoolRequest}) or it's too old, so a screen
 * that mounts again still opens it. */
export function takeNewPoolRequest(): NewPoolRequest | null {
  if (newPoolRequest && Date.now() - newPoolRequest.at > REQUEST_WAITS_MS) newPoolRequest = null;
  return newPoolRequest?.request ?? null;
}

/** The setup a request opened is closed (made or cancelled). */
export function doneNewPoolRequest(): void {
  newPoolRequest = null;
}

export function onNewPoolRequest(listener: () => void): () => void {
  newPoolListeners.add(listener);
  return () => newPoolListeners.delete(listener);
}

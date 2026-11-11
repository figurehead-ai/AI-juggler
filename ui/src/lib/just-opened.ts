/**
 * 0.4.1 (the rc7 walk): right after "Opened in Claude app · aij-test", the
 * switched copy's page read "Ended / Not open now": the app hadn't
 * reported it yet. A switch that opens a conversation in a desktop app
 * says so here, and the copy reads "Starting in <app>…" until the app
 * reports it open, or for a minute and a half at most (then what the app
 * says stands).
 */

/** How long a just-opened copy reads Starting without the app's word. */
export const OPENING_MS = 90_000;

const opened = new Map<string, { app: string; at: number }>();

function key(account: string, conversation: string): string {
  return `${account}:${conversation}`;
}

/** A switch opened conversation `conversation` of `account` in `app` now. */
export function markOpening(account: string, conversation: string, app: string, now: number = Date.now()): void {
  opened.set(key(account, conversation), { app, at: now });
}

/** The app a conversation was just opened in, while it may still be
 * starting; `null` once that's past. */
export function openingIn(account: string, conversation: string, now: number = Date.now()): string | null {
  const id = key(account, conversation);
  const found = opened.get(id);
  if (!found) return null;
  if (now - found.at > OPENING_MS) {
    opened.delete(id);
    return null;
  }
  return found.app;
}

/** Forgets every mark (tests). */
export function resetOpening(): void {
  opened.clear();
}

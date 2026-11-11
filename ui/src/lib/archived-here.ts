/**
 * NY1 (rc32): the sessions archived from this window, so an archived
 * session leaves Needs you at once, with its rail row. The sessions list
 * dropped it on the backend's sessions event, but its need stayed in
 * the needs list until that list was read again: for about three seconds
 * a row in the need's own words ("“…” stopped with an error", the stand-in
 * for a session the list hasn't loaded) took its place. Kept until the
 * session is unarchived; the backend leaves archived sessions out of its
 * needs anyway, so this only bridges the wait.
 */
let archived: ReadonlySet<string> = new Set<string>();
const listeners = new Set<() => void>();

function changed(next: Set<string>): void {
  archived = next;
  for (const listener of listeners) listener();
}

/** `sessionId` was archived here (its needs leave now). */
export function markArchived(sessionId: string): void {
  if (!archived.has(sessionId)) changed(new Set([...archived, sessionId]));
}

/** `sessionId` is back (unarchived, or its archive didn't take). */
export function markUnarchived(sessionId: string): void {
  if (archived.has(sessionId)) changed(new Set([...archived].filter((id) => id !== sessionId)));
}

export function subscribeArchived(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The sessions archived here now (a new set whenever it changes). */
export function archivedHere(): ReadonlySet<string> {
  return archived;
}

/** A need as this filter reads it: its subject and its session. */
interface OfSession {
  subject: string;
  target: { session_id: string | null };
}

/** `rows` without the needs of the sessions in `gone`. */
export function withoutArchived<Row extends OfSession>(rows: Row[], gone: ReadonlySet<string> = archived): Row[] {
  if (gone.size === 0) return rows;
  return rows.filter((row) => !(row.target.session_id !== null && gone.has(row.target.session_id)) && !(row.subject.startsWith("s:") && gone.has(row.subject.slice(2))));
}

/** Test seam: forget them. */
export function resetArchived(): void {
  changed(new Set<string>());
}

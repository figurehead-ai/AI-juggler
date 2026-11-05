/**
 * AJ-E2901: what was chosen from ⌘K lately (at most 8), shown before
 * anything is typed. Kept on this computer as kinds and ids only — no
 * titles, no message text; one whose item is gone is dropped; Settings ›
 * Advanced › Local cache's Clear clears them.
 */
export type RecentKind = "session" | "conversation" | "app" | "project" | "account" | "command";

export interface Recent {
  kind: RecentKind;
  id: string;
}

const KEY = "aij.palette.recents";
export const MAX_RECENTS = 8;
const KINDS: readonly string[] = ["session", "conversation", "app", "project", "account", "command"];

export function loadRecents(): Recent[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((entry): entry is Recent => typeof entry === "object" && entry !== null && KINDS.includes((entry as Recent).kind) && typeof (entry as Recent).id === "string")
      .slice(0, MAX_RECENTS);
  } catch {
    return [];
  }
}

function save(recents: Recent[]): void {
  try {
    if (recents.length === 0) localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, JSON.stringify(recents.slice(0, MAX_RECENTS)));
  } catch {
    // Not kept this time: the palette works without it.
  }
}

/** `entry` first, once. */
export function rememberRecent(entry: Recent): void {
  save([entry, ...loadRecents().filter((recent) => recent.kind !== entry.kind || recent.id !== entry.id)]);
}

/** Drops the ones whose item is gone. */
export function keepRecents(exists: (recent: Recent) => boolean): void {
  const recents = loadRecents();
  const kept = recents.filter(exists);
  if (kept.length !== recents.length) save(kept);
}

export function clearRecents(): void {
  save([]);
}

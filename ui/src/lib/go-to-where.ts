/**
 * Go to it's words (moved out of rail-items.ts so the rail's one list
 * stays within its size; rail-items re-exports it).
 */
import type { RailItem } from "./rail-items";

/** Where Go to it takes it, in words: the desktop App ("Claude app ·
 * b-work"), or the terminal a CLI session is in ("Terminal"); `null` for
 * a session in AI Juggler's own terminal (its page is here: the row
 * already goes there). */
export function goToWhere(item: Pick<RailItem, "place" | "accountLabel" | "session">): string | null {
  if (item.session?.kind === "desktop") return ["its app", item.place.profile ?? item.accountLabel].filter(Boolean).join(" · ");
  if (item.place.kind === "own") return null;
  if (item.place.host) return item.place.host;
  return [item.place.app, item.place.profile].filter(Boolean).join(" · ") || null;
}

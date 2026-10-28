/**
 * D-25: a session's status and kind in words, wherever a wire value
 * (`waiting_user`, `desktop`) would otherwise reach the screen.
 */
import { STATE_WORDS } from "../components/indicator";

/** "Asks you", "Limit reached"; "Unknown" for a value this build doesn't know. */
export function statusWords(status: string): string {
  return STATE_WORDS[status] ?? "Unknown";
}

/** "Terminal", "Desktop app". */
export function kindWords(kind: string): string {
  return kind === "desktop" ? "Desktop app" : kind === "terminal" ? "Terminal" : "Session";
}

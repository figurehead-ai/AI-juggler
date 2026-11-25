/**
 * AJ-E2327 — the rail's empty state, only when Open now is really empty:
 * no App open (by App), nothing open (by Account or Pool) and nothing
 * needing you. 0.5.0 said "Nothing open yet" whenever no session was
 * known, under "Open now 3" (three running Apps whose sessions it
 * couldn't read). Worded for the lens it's under.
 */
import type { Line } from "./rail-lines";

export interface EmptyWords {
  title: string;
  body: string;
}

/** The empty state's words, or `null` when Open now (or Needs you) has
 * anything: `lens` is the rail's grouping; `offered` whether "Running,
 * not added" lists any app. */
export function railEmptyWords(lens: string, lines: readonly Line[]): EmptyWords | null {
  const busy = lines.some((line) => (line.kind === "section" && (line.id === "open" || line.id === "needs")) || line.kind === "need" || line.kind === "notice");
  if (busy) return null;
  const offered = lines.some((line) => line.kind === "offer");
  const then = offered ? "Add a running app below to see its sessions here." : "Start one with New session, or open one in your apps: it shows here.";
  switch (lens) {
    case "account":
      return { title: "Nothing open on your accounts yet", body: then };
    case "pool":
      return { title: "Nothing open in your pools yet", body: then };
    default:
      return { title: "No apps open yet", body: then };
  }
}

/**
 * Round 7 #9 (aij-test run 3): a list of candidates (New pool's Who's in,
 * "Add a session") said how a desktop session takes part on every row, a
 * long amber paragraph as many times as there were rows. The list says
 * it once ({@link DesktopListNote}); each row says only where it stands,
 * in a few words ({@link DesktopReachTag}): "Can talk", or "Watched".
 */
import type { JSX } from "react";

import { useApps } from "../../../lib/apps";
import { desktopAppsOf, isAdded, useConnectorStatus } from "../../../lib/connector";

/** The list's one sentence on how desktop sessions take part, when any
 * row is one. */
export function DesktopListNote({ notes, testId }: { notes: readonly (string | undefined)[]; testId: string }): JSX.Element | null {
  const once = [...new Set(notes.filter((note): note is string => !!note))];
  if (once.length === 0) return null;
  return (
    <p className="text-meta text-ink-2" data-testid={testId}>
      {once.join(" ")}
    </p>
  );
}

/** Where one row's app stands: "Can talk" with AI Juggler's connector in
 * it, else "Watched"; nothing until it's read. */
export function DesktopReachTag({ member, testId }: { member: string; testId: string }): JSX.Element | null {
  const apps = useApps();
  const app = desktopAppsOf([member], apps ?? [])[0];
  const status = useConnectorStatus(app?.id ?? null);
  if (!app || !status) return null;
  const can = isAdded(status.state);
  return (
    <span className={"shrink-0 text-label font-medium " + (can ? "text-success" : "text-warning")} data-testid={testId} data-reach={can ? "talk" : "watched"}>
      {can ? "Can talk" : "Watched"}
    </span>
  );
}

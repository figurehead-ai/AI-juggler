/**
 * The app's list of sessions is its working set (open, pinned and the
 * recent ended ones: `instances_list`), never the whole history (0.1.5
 * review). A session past it, opened from History or a project's
 * Sessions, is read by its id and joins the list while it's selected.
 */
import { useEffect, useMemo, useState } from "react";
import type { InstanceInfo } from "./bridge";
import { sessionsGet } from "./ipc/sessions";

/** `sessions`, with the selected one when it's past the working set. */
export function useWithSelected(sessions: InstanceInfo[], selectedId: string | null): InstanceInfo[] {
  const [extra, setExtra] = useState<InstanceInfo | null>(null);
  const missing = selectedId !== null && !sessions.some((session) => session.id === selectedId);
  useEffect(() => {
    if (!missing || selectedId === null) return;
    let alive = true;
    void sessionsGet([selectedId]).then(
      (rows) => alive && setExtra(rows[0] ?? null),
      () => null,
    );
    return () => {
      alive = false;
    };
  }, [missing, selectedId]);
  return useMemo(
    () => (missing && extra && extra.id === selectedId ? [...sessions, extra] : sessions),
    [missing, extra, selectedId, sessions],
  );
}

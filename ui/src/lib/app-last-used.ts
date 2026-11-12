/**
 * D-100 (the 0.4.2 review): where a CLI App was last used, when the
 * sessions the window has read say nothing of it (an older terminal
 * session AI Juggler started: personal-max's read "No sessions yet" while
 * it had one). One row of the backend's own list for that App (the same
 * attribution as its counts), newest first.
 */
import { useEffect, useState } from "react";
import { hasLiveBackend } from "./bridge";
import { sessionsPage, sessionsQuery } from "./ipc/sessions";

/** When and where an App was last used. */
export interface LastUsed {
  lastMs: number;
  cwd: string | null;
}

/** Each of `appIds`' last use, read once per id (none for one without). */
export function useLastUsed(appIds: readonly string[]): ReadonlyMap<string, LastUsed> {
  const [found, setFound] = useState<ReadonlyMap<string, LastUsed>>(new Map());
  const wanted = appIds.join("\n");
  useEffect(() => {
    let live = true;
    if (!hasLiveBackend() || wanted === "") return () => {
      live = false;
    };
    void Promise.all(
      wanted.split("\n").map(async (id) => {
        const page = await sessionsPage(sessionsQuery({ app_id: id }), { cursor: null, limit: 1 }).catch(() => null);
        const row = page?.items[0];
        const lastMs = row?.session?.last_activity_ms ?? row?.session?.started_at_ms ?? row?.conversation?.last_activity_ms ?? null;
        return lastMs ? ([id, { lastMs, cwd: row?.session?.cwd ?? row?.conversation?.cwd ?? null }] as const) : null;
      }),
    ).then((rows) => live && setFound(new Map(rows.filter((row): row is NonNullable<typeof row> => row !== null))));
    return () => {
      live = false;
    };
  }, [wanted]);
  return found;
}

/**
 * AJ-E3824 (D-144): ⌘K is the one search for the whole app. What's typed
 * is also looked up in AI Juggler's records (sessions, tasks, their
 * pools, usage) through Bot's one look-up, a moment after typing rests;
 * an answer for older words is never shown for newer ones. A found
 * record opens its page by id; nothing found says what was searched.
 */
import { useEffect, useState, type JSX } from "react";
import { NOTHING_FOUND, botSearch, type SearchRecordsInfo } from "../../lib/ipc/search";
import type { AppDoings } from "../../lib/palette-commands";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { PaletteRow } from "./rows";

/** How long typing rests before the records are looked up. */
const RECORDS_WAIT_MS = 150;

export function useFoundRecords(open: boolean, typed: string): SearchRecordsInfo | null {
  const asked = typed.trim();
  const [found, setFound] = useState<{ text: string; records: SearchRecordsInfo } | null>(null);
  useEffect(() => {
    let live = open && asked !== "";
    const timer = live
      ? window.setTimeout(() => {
          void botSearch(asked)
            .catch(() => NOTHING_FOUND)
            .then((records) => {
              if (live) setFound({ text: asked, records });
            });
        }, RECORDS_WAIT_MS)
      : undefined;
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [open, asked]);
  return asked && found?.text === asked ? found.records : null;
}

/** Opens a found record's page by id (⌘↩ on a desktop session: its window). */
export function openFound(
  entry: Extract<PaletteRow, { where: "record" | "pool" }>,
  toApp: boolean,
  to: { app?: AppDoings; sessions: InstanceInfo[]; onOpenSession: (id: string) => void; onFocusWindow: (session: InstanceInfo) => void },
): void {
  if (entry.where === "pool") return to.app?.openPool?.(entry.pool.id);
  const row = entry.row;
  if (row.kind === "session") {
    const live = toApp && row.desktop ? to.sessions.find((session) => session.id === row.id) : undefined;
    if (live) to.onFocusWindow(live);
    else to.onOpenSession(row.id);
  } else if (row.kind === "task" && row.pool_id) to.app?.openPool?.(row.pool_id);
  else if (row.kind === "account") to.app?.openAccount?.(row.id);
}

/** Nothing found: says so, and what was searched (Suggested follows). */
export function NoMatches({ typed }: { typed: string }): JSX.Element {
  return (
    <li role="presentation" className="px-2.5 py-2 text-meta text-muted">
      <span className="block" data-testid="goto-no-matches">{`No matches for “${typed}”.`}</span>
      <span className="block text-label" data-testid="goto-searched">Looked in sessions, tasks, pools, usage, apps, projects, accounts and settings.</span>
    </li>
  );
}

/**
 * E31 (AJ-E3103) — a related list in the design preview (no backend):
 * its rows narrowed, searched and ordered here by the same rules the
 * backend's `sessions_page` uses (`order.ts`), so the gallery shows each
 * state the app does.
 */
import { useCallback } from "react";
import { appIdOfRailKey } from "../../lib/apps";
import { useKnownPlaces } from "../../lib/places";
import { accountOfProfile, useSubscriptions } from "../../lib/subscriptions";
import type { Row } from "../../pages/SessionsList";
import { ordered, passes, type Belongs, type Filters } from "./order";

/** What each row belongs to, as the backend groups it: its project, its
 * App (the desktop App it's open in, else its folder's CLI App), its
 * folder's Account, its Mac. */
export function useBelongs(wanted: boolean): (row: Row) => Belongs {
  const places = useKnownPlaces();
  const accounts = useSubscriptions(0, wanted);
  return useCallback(
    (row: Row) => {
      const session = row.item.kind === "conversation" ? null : row.item.session;
      return {
        project: row.projectId ?? "",
        app: appIdOfRailKey(row.rail.place.key, row.accountId, places) ?? row.rail.place.key,
        account: row.accountId ? (accountOfProfile(accounts ?? [], row.accountId)?.account_id ?? null) : null,
        machine: session?.machine_id ?? "this-mac",
      };
    },
    [places, accounts],
  );
}

/** Does every word of the search appear in what the row says? */
function matchesWords(row: Row, words: string): boolean {
  const wanted = words.toLowerCase().split(/\s+/).filter(Boolean);
  if (wanted.length === 0) return true;
  const text = [row.title, row.rail.project, row.accountLabel, row.tool, row.detail, row.rail.place.app].filter(Boolean).join(" ").toLowerCase();
  return wanted.every((word) => text.includes(word));
}

/** The preview's rows for a list: narrowed, searched, in the list's order. */
export function localRows(rows: readonly Row[], belongs: (row: Row) => Belongs, filters: Filters, words: string, now: number): Row[] {
  const shown = rows.filter((row) => passes(row.rail, belongs(row), filters, now) && matchesWords(row, words));
  return ordered(shown, (row) => row.rail, (row) => row.key, now);
}

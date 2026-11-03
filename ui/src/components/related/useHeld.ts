/**
 * E31 (AJ-E3103) — a paged list that never jumps while it is read: its
 * first fresh read is shown as it comes; every read after it keeps the
 * shown rows in place (`hold.ts`), with "N new" (or "Show in order") to
 * take the new order.
 */
import { useCallback, useMemo, useRef, useState } from "react";
import type { PagedList } from "../../lib/paged";
import { holdAgain, holdAll, waitingWords, type Held } from "./hold";

export interface HeldList<T, B extends string> {
  /** The list with its rows in the order shown. */
  list: PagedList<T>;
  /** A shown row's section (where it was placed). */
  bandOf: (row: T) => B;
  /** "3 new", "Show in order", or `null`. */
  waiting: string | null;
  /** Shows the list as read, in order. */
  showAll: () => void;
}

export function useHeld<T, B extends string>(list: PagedList<T>, keyOf: (row: T) => string, bandOf: (row: T) => B, scope: string): HeldList<T, B> {
  const held = useRef<{ scope: string; settled: boolean; hold: Held<B> | null }>({ scope, settled: false, hold: null });
  const [version, setVersion] = useState(0);
  const byKey = useMemo(() => new Map(list.items.map((row) => [keyOf(row), row])), [list.items, keyOf]);
  const hold = useMemo(() => {
    const state = held.current;
    if (state.scope !== scope) {
      held.current = { scope, settled: false, hold: null };
    }
    const now = held.current;
    const fresh = list.items.map(keyOf);
    const band = (key: string) => bandOf(byKey.get(key)!);
    // Until the first fresh read has shown, every read is shown as it is
    // (the remembered rows, then the fresh ones).
    now.hold = now.settled && now.hold ? holdAgain(now.hold, fresh, band) : holdAll(fresh, band);
    if (!list.loading && !list.stale && list.items.length > 0) now.settled = true;
    return now.hold;
    // `version`: Show all asks for the order again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [list.items, list.loading, list.stale, scope, version, byKey]);
  const showAll = useCallback(() => {
    held.current.settled = false;
    setVersion((n) => n + 1);
  }, []);
  const items = useMemo(() => hold.keys.flatMap((key) => byKey.get(key) ?? []), [hold, byKey]);
  const shownBand = useCallback((row: T) => hold.bands.get(keyOf(row)) ?? bandOf(row), [hold, keyOf, bandOf]);
  return {
    list: { ...list, items, total: list.total === null ? null : list.total - hold.waiting.size },
    bandOf: shownBand,
    waiting: waitingWords(hold),
    showAll,
  };
}

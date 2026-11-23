/**
 * AJ-E1552 — the conditions a watch's targets can all report (one
 * catalogue read per target, each with a deadline; a failed read is an
 * error with Retry, never a skeleton for ever).
 */
import { useEffect, useMemo, useState } from "react";
import { watchCatalogue, type CatalogueEntry } from "../../lib/ipc/watch-view";
import { logLoadFailure, withDeadline } from "../../lib/loading";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { intersect, targetKey } from "./watches-view";

/** The conditions every target can report, as cards. */
export function useOffered(targets: readonly WatchTarget[]): { offered: CatalogueEntry[]; loading: boolean; failed: boolean; round: number; retry: () => void } {
  const [lists, setLists] = useState<Record<string, CatalogueEntry[]>>({});
  const [missed, setMissed] = useState<string | null>(null);
  const [round, setRound] = useState(0);
  const keys = targets.map(targetKey);
  const want = keys.join("|");
  useEffect(() => {
    let live = true;
    const missing = targets.filter((t) => lists[targetKey(t)] === undefined);
    // AJ-E1552: each read has a deadline; a failed one is an error with Retry.
    if (missing.length > 0)
      void Promise.all(missing.map(async (t) => [targetKey(t), await withDeadline("the conditions", watchCatalogue(t))] as const)).then(
        (got) => {
          if (live) setLists((was) => ({ ...was, ...Object.fromEntries(got) }));
        },
        (cause: unknown) => {
          if (!live) return;
          logLoadFailure("the conditions", cause);
          setMissed(`${round}|${want}`);
        },
      );
    return () => {
      live = false;
    };
    // `want` names the targets, `round` is a Retry; `lists` only fills in.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [want, round]);
  const waiting = keys.some((key) => lists[key] === undefined);
  // `loading` stays true while the list is missing (the form waits on
  // it); `failed` says the read failed rather than still runs.
  const failed = waiting && missed === `${round}|${want}`;
  const offered = useMemo(() => (waiting ? [] : intersect(keys.map((key) => lists[key]!))), [waiting, lists, want]); // eslint-disable-line react-hooks/exhaustive-deps
  return { offered, loading: waiting, failed, round, retry: () => setRound((was) => was + 1) };
}


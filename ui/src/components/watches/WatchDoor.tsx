/**
 * AJ-E3816 — the door on a session's or an App's page, beside "Ask Bot
 * about this": "Watched · 2" when watches name it (opening the Watches
 * page showing only those), else "Watch…" in the same spot (opening the
 * form with it picked). One shared read of the live watches (at most 50)
 * serves every page; a change on the Watches page reads it again.
 */
import type { JSX } from "react";
import { sharedRead, useSharedRead } from "../../lib/shared-read";
import { openWatches } from "../../lib/watches-open";
import { watchesPage, type WatchInfo } from "../../lib/ipc/watch-view";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { DOORS } from "./watches-words";
import { IconEye } from "./watch-icons";

const LIVE = sharedRead(async () => (await watchesPage("live", { cursor: null, limit: 50 })).items);

/** Read the live watches again (a watch was made, changed or removed). */
export function watchesChanged(): void {
  void LIVE.refresh();
}

/** How many live watches name `target`. */
export function watchedBy(watches: readonly WatchInfo[], target: WatchTarget): number {
  const key = target.kind === "any" ? "any" : `${target.kind}:${target.id}`;
  return watches.filter((info) => info.watch.targets.some((t) => (t.kind === "any" ? "any" : `${t.kind}:${t.id}`) === key)).length;
}

const LOOK = "inline-flex h-7 shrink-0 items-center gap-1.5 rounded-lg px-2.5 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-accent/60 ";

export function WatchDoor({ target, name }: { target: WatchTarget; name: string }): JSX.Element {
  const live = useSharedRead(LIVE, { maxAgeMs: 30_000 });
  const n = live ? watchedBy(live, target) : 0;
  if (n > 0) {
    return (
      <button type="button" onClick={() => openWatches({ kind: "filter", target, name })} className={LOOK + "bg-accent-soft text-accent-hi inset-ring-1 inset-ring-accent/30 hover:bg-accent/15"} data-testid="watch-door" data-watched={n}>
        <IconEye width={14} height={14} aria-hidden="true" />
        {DOORS.watched(n)}
      </button>
    );
  }
  return (
    <button type="button" onClick={() => openWatches({ kind: "new", target })} className={LOOK + "bg-surface-2 text-ink inset-ring-1 inset-ring-hairline hover:bg-surface-3"} data-testid="watch-door" data-watched={0}>
      <IconEye width={14} height={14} aria-hidden="true" />
      {DOORS.watch}
    </button>
  );
}

/**
 * AJ-E3816 — the Watches page's two lists: Watching (the rows, compact
 * and paged past eight, with the toolbar), and Ended (folded, read a page
 * at a time once opened, kept 30 days).
 */
import { useState, type JSX } from "react";
import { Button } from "../primitives";
import { usePagedList } from "../../lib/paged";
import { watchesPage, type WatchInfo } from "../../lib/ipc/watch-view";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { WatchRow, type WatchRowActions } from "./WatchRow";
import { WatchEndedRow, wayOf, type EndedActions } from "./WatchEndedRow";
import { FoldHead } from "./WatchParts";
import { WatchesToolbar, type ToolbarState } from "./WatchesHead";
import { FIRST_ROWS, MANY_AT, MORE_ROWS, optionOf, shownWatches, type ListFilter, type TargetOption } from "./watches-view";
import { MANY, PAGE } from "./watches-words";
import { IconCross } from "./watch-icons";

/** Ended watches read at a time. */
const ENDED_PAGE = 10;

export function WatchingSection(props: {
  live: WatchInfo[];
  only: { target: WatchTarget; name: string } | null;
  onClearOnly: () => void;
  toolbar: ToolbarState;
  onToolbar: (next: Partial<ToolbarState>) => void;
  counts: { watching: number; paused: number; toldToday: number; never: number };
  book: Map<string, TargetOption>;
  now: number;
  actions: WatchRowActions;
  quietTo: string | null;
}): JSX.Element {
  const { book, now, toolbar } = props;
  const [rows, setRows] = useState(FIRST_ROWS);
  const nameOf = (info: WatchInfo) => info.echo.head.find((s) => s.name)?.text ?? optionOf(book, info.watch.targets[0]!).name;
  const scoped = props.only ? props.live.filter((w) => w.watch.targets.some((t) => optionOf(book, t).key === optionOf(book, props.only!.target).key)) : props.live;
  const many = scoped.length > MANY_AT;
  const shown = shownWatches(scoped, many ? { ...toolbar, now, nameOf } : { query: "", filter: "all", sort: "listed", now, nameOf });
  const visible = many ? shown.slice(0, rows) : shown;
  const counts: Record<ListFilter, number> = { all: scoped.length, told_today: props.counts.toldToday, never: props.counts.never, paused: props.counts.paused };
  return (
    <section aria-labelledby="watches-watching" className="mt-1 flex flex-col gap-3">
      <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
        <h3 id="watches-watching" className="text-section font-semibold text-ink">
          {PAGE.watching}
        </h3>
        <span className="text-meta text-muted">{PAGE.watchingLine(props.counts.watching, props.counts.paused)}</span>
        {props.only && (
          <button type="button" onClick={props.onClearOnly} className="ml-1 inline-flex h-6 items-center gap-1 rounded-full bg-accent-soft px-2 text-label font-medium text-accent-hi outline-none focus-visible:ring-2 focus-visible:ring-accent/60" aria-label={`${props.only.name}: show every watch`} data-testid="watches-only">
            {props.only.name}
            <IconCross width={11} height={11} aria-hidden="true" />
          </button>
        )}
      </div>
      {many && (
        <WatchesToolbar
          state={toolbar}
          counts={counts}
          onChange={(next) => {
            props.onToolbar(next);
            setRows(FIRST_ROWS);
          }}
        />
      )}
      <div className="rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
        {visible.length === 0 ? (
          <p className="px-3 py-4 text-meta text-muted">{MANY.none}</p>
        ) : (
          <ul aria-labelledby="watches-watching" data-testid="watches-list">
            {visible.map((info) => (
              <WatchRow key={info.watch.id} info={info} book={book} now={now} compact={many} actions={props.actions} quietTo={props.quietTo} />
            ))}
          </ul>
        )}
        {many && shown.length > visible.length && (
          <div className="flex items-center gap-3 border-t border-divider px-4 py-3 text-meta text-muted" data-testid="watches-pager">
            <span className="flex-1">{PAGE.shown(visible.length, shown.length)}</span>
            <Button size="sm" onClick={() => setRows((n) => n + MORE_ROWS)} data-testid="watches-more">
              {PAGE.showMore(Math.min(MORE_ROWS, shown.length - visible.length))}
            </Button>
          </div>
        )}
      </div>
    </section>
  );
}

export function EndedSection({ count, open, onToggle, book, now, appOf, actions }: { count: number; open: boolean; onToggle: () => void; book: Map<string, TargetOption>; now: number; appOf: (sessionId: string) => { id: string; name: string } | null; actions: EndedActions }): JSX.Element {
  const ended = usePagedList<WatchInfo>(open ? "watches:ended" : null, (request) => watchesPage("ended", request), { pageSize: ENDED_PAGE, keyOf: (row) => row.watch.id });
  return (
    <section aria-label={PAGE.ended} className="flex flex-col gap-3">
      <FoldHead open={open} onToggle={onToggle} title={PAGE.ended} line={PAGE.endedLine(count)} controls="watches-ended" testId="watches-ended-fold" />
      {open && (
        <div id="watches-ended" className="rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
          {ended.loading ? (
            <div aria-hidden="true" className="m-2 h-14 animate-pulse rounded-lg bg-skeleton/60" />
          ) : (
            <ul data-testid="watches-ended">
              {ended.items.map((info) => (
                <WatchEndedRow key={info.watch.id} info={info} book={book} now={now} way={wayOf(info, appOf)} actions={actions} />
              ))}
            </ul>
          )}
          {ended.hasMore && (
            <div className="border-t border-divider px-4 py-2.5">
              <Button size="sm" onClick={ended.loadMore} disabled={ended.loadingMore} data-testid="watches-ended-more">
                {PAGE.showMore(ENDED_PAGE)}
              </Button>
            </div>
          )}
        </div>
      )}
    </section>
  );
}

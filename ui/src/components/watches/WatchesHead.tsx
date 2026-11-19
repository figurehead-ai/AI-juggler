/**
 * AJ-E3816 — the Watches page's head and its lines: the title with the
 * count of the limit, what a watch is with "How watches work", Ask Bot
 * and New watch; the quiet-hours line (only when the user has quiet
 * hours); the limit's amber alert (only once New watch is pressed at the
 * limit); and the toolbar past eight watches (search, filters with
 * counts, sort).
 */
import type { JSX } from "react";
import { Button } from "../primitives";
import { Picker, Segmented } from "../controls";
import { trustOpen } from "../../lib/ipc/trust";
import type { ListFilter, ListSort } from "./watches-view";
import { MANY, PAGE, QUIET } from "./watches-words";
import { IconAdd, IconBotHead, IconChevronR, IconFind, IconMoon, IconWarnTriangle } from "./watch-icons";

/** The guide's section on watches. */
export const WATCHES_GUIDE = "notifications#watches-you-set";

export function WatchesHead({ count, limit, empty, onNew, onAskBot }: { count: number; limit: number; empty: boolean; onNew: () => void; onAskBot: (() => void) | null }): JSX.Element {
  return (
    <div className="flex flex-wrap items-end gap-x-4 gap-y-3">
      <div className="flex min-w-0 flex-1 basis-96 flex-col gap-1">
        <div className="flex items-baseline gap-2.5">
          <h2 className="text-title text-ink">{PAGE.title}</h2>
          {!empty && (
            <span className="text-meta font-medium tabular-nums text-muted" data-testid="watches-count">
              {PAGE.of(count, limit)}
            </span>
          )}
        </div>
        <p className="max-w-[62ch] text-meta text-ink-2">
          {PAGE.lead}{" "}
          <button type="button" onClick={() => void trustOpen({ kind: "guide", id: WATCHES_GUIDE })} className="font-medium text-accent-hi hover:underline" data-testid="watches-how">
            {PAGE.how}
          </button>
        </p>
      </div>
      {!empty && (
        <div className="flex items-center gap-3">
          {onAskBot && (
            <button type="button" onClick={onAskBot} className="inline-flex h-9 items-center gap-1.5 rounded-lg px-2 text-meta font-medium text-accent-hi outline-none hover:bg-accent-soft focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watches-ask-bot">
              <IconBotHead width={15} height={15} aria-hidden="true" />
              {PAGE.askBot}
            </button>
          )}
          <Button variant="primary" onClick={onNew} data-testid="watches-new">
            <IconAdd width={15} height={15} aria-hidden="true" />
            {PAGE.newWatch}
          </Button>
        </div>
      )}
    </div>
  );
}

export function QuietLine({ from, to, breakThrough, onOpen }: { from: string; to: string; breakThrough: number; onOpen: () => void }): JSX.Element {
  return (
    <div className="flex items-center gap-2.5 rounded-xl bg-surface-2 px-3.5 py-[9px] text-meta text-ink-2" data-testid="watches-quiet-line">
      <IconMoon width={16} height={16} aria-hidden="true" className="shrink-0 text-faint" />
      <p className="min-w-0 flex-1">
        {QUIET.line(from, to, breakThrough).map((part, i) =>
          part.strong ? (
            <b key={i} className="font-semibold text-ink">
              {part.text}
            </b>
          ) : (
            <span key={i}>{part.text}</span>
          ),
        )}
      </p>
      <button type="button" onClick={onOpen} className="inline-flex shrink-0 items-center gap-1 rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watches-quiet-open">
        {QUIET.link}
        <IconChevronR width={12} height={12} aria-hidden="true" />
      </button>
    </div>
  );
}

export function LimitAlert({ limit, never, onShow }: { limit: number; never: number; onShow: () => void }): JSX.Element {
  return (
    <div role="alert" className="flex items-start gap-2.5 rounded-xl bg-surface-2 px-3.5 py-3 text-meta text-ink-2 shadow-[inset_3px_0_0_var(--color-warning)]" data-testid="watches-limit">
      <IconWarnTriangle width={16} height={16} aria-hidden="true" className="mt-px shrink-0 text-warning" />
      <p className="min-w-0 flex-1">
        <b className="font-semibold text-ink">{MANY.limit(limit)}</b> {MANY.limitLine(never)}
      </p>
      {never > 0 && (
        <button type="button" onClick={onShow} className="shrink-0 rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watches-limit-show">
          {MANY.showThose(never)}
        </button>
      )}
    </div>
  );
}

export interface ToolbarState {
  query: string;
  filter: ListFilter;
  sort: ListSort;
}

export function WatchesToolbar({ state, counts, onChange }: { state: ToolbarState; counts: Record<ListFilter, number>; onChange: (next: Partial<ToolbarState>) => void }): JSX.Element {
  return (
    <div className="flex flex-wrap items-center gap-2.5" data-testid="watches-toolbar">
      <label className="flex h-9 min-w-[200px] flex-1 basis-56 items-center gap-2 rounded-[10px] bg-fill px-2.5 inset-ring-1 inset-ring-hairline/75">
        <IconFind width={15} height={15} aria-hidden="true" className="text-faint" />
        <span className="sr-only">{MANY.search}</span>
        <input value={state.query} onChange={(event) => onChange({ query: event.target.value })} placeholder={MANY.search} className="min-w-0 flex-1 bg-transparent text-meta text-ink outline-none placeholder:text-faint" data-testid="watches-search" />
      </label>
      <Segmented<ListFilter>
        label={MANY.filter}
        value={state.filter}
        onChange={(filter) => onChange({ filter })}
        segments={[
          { value: "all", label: MANY.all, count: counts.all },
          { value: "told_today", label: MANY.toldToday, count: counts.told_today },
          { value: "never", label: MANY.neverTold, count: counts.never },
          { value: "paused", label: MANY.paused, count: counts.paused },
        ]}
        testId="watches-filter"
      />
      <span className="flex-1" />
      <span className="text-meta text-muted">{MANY.sort}</span>
      <Picker<ListSort>
        choice
        size="sm"
        value={state.sort === "listed" ? "last" : state.sort}
        ariaLabel={MANY.sort}
        options={[
          { value: "last", label: MANY.sortLast },
          { value: "newest", label: MANY.sortNewest },
          { value: "target", label: MANY.sortTarget },
        ]}
        onChange={(sort) => onChange({ sort })}
        testId="watches-sort"
      />
    </div>
  );
}

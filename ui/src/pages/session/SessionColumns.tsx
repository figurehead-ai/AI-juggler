/**
 * AJ-E2108 — a session page's body: its conversation as the main pane
 * (the last messages of a session another app runs, the terminal of one
 * AI Juggler runs), filling the height under the header, and beside it a
 * fixed column (where it runs, its runway, where it could continue) that
 * scrolls within itself. One layout for every kind of session.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";

/** Whether a box that scrolls has more below what shows (a short window:
 * the side column says so with a "More below" chip; never a fade, which
 * made the last card look disabled). */
function useMoreBelow(): [React.RefObject<HTMLDivElement | null>, boolean, () => void] {
  const box = useRef<HTMLDivElement | null>(null);
  const [more, setMore] = useState(false);
  const measure = () => {
    const el = box.current;
    if (el) setMore(el.scrollHeight - el.clientHeight - el.scrollTop > 4);
  };
  useEffect(() => {
    measure();
    const el = box.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    for (const child of el.children) watch.observe(child);
    return () => watch.disconnect();
  });
  return [box, more, measure];
}

export function SessionColumns({ main, side, testId = "session-overview" }: { main: ReactNode; side: ReactNode; testId?: string }): JSX.Element {
  const [box, moreBelow, onSideScroll] = useMoreBelow();
  return (
    <div className="@container flex min-h-0 flex-1 flex-col" data-testid={testId}>
      <div className="mx-auto grid w-full max-w-[1440px] min-h-0 flex-1 grid-cols-1 gap-4 @4xl:grid-cols-[minmax(0,1fr)_340px] @4xl:grid-rows-[minmax(0,1fr)]">
        <div className="flex min-h-[28rem] min-w-0 flex-col @4xl:min-h-0">{main}</div>
        <div className="flex min-w-0 flex-col gap-2 @4xl:min-h-0">
          <div
            ref={box}
            onScroll={onSideScroll}
            className="flex min-w-0 flex-col gap-4 rounded-card outline-none focus-visible:ring-1 focus-visible:ring-accent/60 @4xl:min-h-0 @4xl:flex-1 @4xl:overflow-y-auto"
            data-testid="session-side"
            data-more-below={moreBelow ? "true" : undefined}
            tabIndex={0}
            role="region"
            aria-label="About this session"
          >
            {side}
          </div>
          {/* Under the column, never over a card: the cards stay whole and
              bright, and this says there's more (a click scrolls to it). */}
          {moreBelow && (
            <button
              type="button"
              onClick={() => box.current?.scrollBy({ top: box.current.clientHeight * 0.8, behavior: "smooth" })}
              className="hidden shrink-0 items-center justify-center gap-1.5 self-center rounded-full px-3 py-0.5 text-meta font-medium text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 @4xl:inline-flex"
              data-testid="session-side-more"
            >
              More below
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M6 9l6 6 6-6" />
              </svg>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

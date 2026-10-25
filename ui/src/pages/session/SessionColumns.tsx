/**
 * AJ-E2108 — a session page's body: its conversation as the main pane
 * (the last messages of a session another app runs, the terminal of one
 * AI Juggler runs), filling the height under the header, and beside it a
 * fixed column (where it runs, its runway, where it could continue) that
 * scrolls within itself. One layout for every kind of session.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";

/** Whether a box that scrolls has more below what shows (a short window:
 * the side column's last card under the fold says so with a fade). */
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
        <div
          ref={box}
          onScroll={onSideScroll}
          className={
            "flex min-w-0 flex-col gap-4 rounded-card outline-none focus-visible:ring-1 focus-visible:ring-accent/60 @4xl:min-h-0 @4xl:overflow-y-auto " +
            (moreBelow ? "@4xl:[mask-image:linear-gradient(to_bottom,black_calc(100%-48px),transparent)]" : "")
          }
          data-testid="session-side"
          data-more-below={moreBelow ? "true" : undefined}
          tabIndex={0}
          role="region"
          aria-label="About this session"
        >
          {side}
        </div>
      </div>
    </div>
  );
}

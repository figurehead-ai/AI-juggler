/**
 * AJ-E2108 — a session page's body: its conversation as the main pane
 * (the last messages of a session another app runs, the terminal of one
 * AI Juggler runs), filling the height under the header, and beside it a
 * fixed column (where it runs, its runway, where it could continue) that
 * scrolls within itself. One layout for every kind of session.
 */
import { useEffect, useState, type JSX, type ReactNode, type RefObject } from "react";
import { MoreBelow, useMoreBelow } from "../../components/MoreBelow";

/** The width a box's scroll bar takes from its content (0 while it
 * overlays or doesn't show). */
export function useScrollbarGutter(box: RefObject<HTMLElement | null>): number {
  const [gutter, setGutter] = useState(0);
  useEffect(() => {
    const el = box.current;
    if (!el) return undefined;
    const measure = () => setGutter(Math.max(0, el.offsetWidth - el.clientWidth));
    measure();
    if (typeof ResizeObserver === "undefined") return undefined;
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    for (const child of el.children) watch.observe(child);
    return () => watch.disconnect();
  });
  return gutter;
}

export function SessionColumns({ main, side, lead, testId = "session-overview" }: { main: ReactNode; side: ReactNode; lead?: ReactNode; testId?: string }): JSX.Element {
  const [box, moreBelow, onSideScroll] = useMoreBelow();
  // The 0.5.0 walk, F7: with a scroll bar shown, the cards under `lead`
  // were 13 px narrower than it; `lead` leaves the same room.
  const gutter = useScrollbarGutter(box);
  return (
    // R2 (the 0.5.0 re-walk): named, so what sits inside the main pane
    // (under the conversation switcher's own container) asks this one
    // whether the page has two columns (`@4xl/session:`).
    <div className="@container/session flex min-h-0 flex-1 flex-col" data-testid={testId}>
      {/* D-143 (PAT-SW): `lead`, the side column's first thing, never scrolls
          away: above the column (outside its scroll) when wide, above the
          main area when narrow (at 1024 wide the column stacks below it). */}
      <div className={"mx-auto grid w-full max-w-[1440px] min-h-0 flex-1 grid-cols-1 gap-4 @4xl:grid-cols-[minmax(0,1fr)_340px] " + (lead ? "@4xl:grid-rows-[auto_minmax(0,1fr)]" : "@4xl:grid-rows-[minmax(0,1fr)]")}>
        {lead && (
          <div className="min-w-0 @4xl:col-start-2 @4xl:row-start-1" style={gutter > 0 ? { paddingRight: gutter } : undefined} data-testid="session-lead">
            {lead}
          </div>
        )}
        <div className={"flex min-h-[28rem] min-w-0 flex-col @4xl:min-h-0 " + (lead ? "@4xl:col-start-1 @4xl:row-span-2 @4xl:row-start-1" : "")}>{main}</div>
        <div className={"flex min-w-0 flex-col gap-2 @4xl:min-h-0 " + (lead ? "@4xl:col-start-2 @4xl:row-start-2" : "")}>
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
          {moreBelow && <MoreBelow box={box} className="hidden @4xl:inline-flex" testId="session-side-more" />}
        </div>
      </div>
    </div>
  );
}

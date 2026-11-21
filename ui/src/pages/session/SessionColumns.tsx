/**
 * AJ-E2108 — a session page's body: its conversation as the main pane
 * (the last messages of a session another app runs, the terminal of one
 * AI Juggler runs), filling the height under the header, and beside it a
 * fixed column (where it runs, its runway, where it could continue) that
 * scrolls within itself. One layout for every kind of session.
 */
import type { JSX, ReactNode } from "react";
import { MoreBelow, useMoreBelow } from "../../components/MoreBelow";

export function SessionColumns({ main, side, lead, testId = "session-overview" }: { main: ReactNode; side: ReactNode; lead?: ReactNode; testId?: string }): JSX.Element {
  const [box, moreBelow, onSideScroll] = useMoreBelow();
  return (
    <div className="@container flex min-h-0 flex-1 flex-col" data-testid={testId}>
      {/* D-143 (PAT-SW): `lead`, the side column's first thing, never scrolls
          away: above the column (outside its scroll) when wide, above the
          main area when narrow (at 1024 wide the column stacks below it). */}
      <div className={"mx-auto grid w-full max-w-[1440px] min-h-0 flex-1 grid-cols-1 gap-4 @4xl:grid-cols-[minmax(0,1fr)_340px] " + (lead ? "@4xl:grid-rows-[auto_minmax(0,1fr)]" : "@4xl:grid-rows-[minmax(0,1fr)]")}>
        {lead && <div className="min-w-0 @4xl:col-start-2 @4xl:row-start-1">{lead}</div>}
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

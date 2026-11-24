/**
 * AJ-E1503 Req.4 / AJ-E1552: Home while its first load answers, in Home's
 * own shape (0.5.1 F64, D-103.6, AJ-E1551: the first frame holds the page
 * that lands, so nothing moves when the content arrives): its heading,
 * then D-118's two columns (`lib/home-layout`, stacked when narrow) — on
 * the left Needs you now's head, a need card and the feed's chips and
 * rows; on the right Your apps and Runway by account, each a head over its
 * card. Cards by tone, no outlines (PAT-SURF-1). "Still loading…" after
 * 3 s. The shell's first read has its own deadline (shell/use-app-data);
 * when it misses, the read counts as done and the "service stopped
 * answering" banner, with its Retry, takes over, so this skeleton never
 * stays. With no account, the welcome is Home's empty state.
 */
import type { JSX } from "react";
import { LoadingState } from "../components/LoadingState";
import { useLoadDeadline } from "../lib/loading";
import { COLUMNS, useHomeLayout } from "../lib/home-layout";

function Bar({ className }: { className: string }): JSX.Element {
  return <span className={`block rounded bg-skeleton ${className}`} />;
}

/** A line of `text` type's height holding a bar (as the words it stands for). */
function Line({ text, className }: { text: string; className: string }): JSX.Element {
  return (
    <span className={`flex h-[1lh] items-center ${text}`}>
      <Bar className={className} />
    </span>
  );
}

/** A section's head (HomeSection's `min-h-7` row, its label's width). */
function Head({ width }: { width: string }): JSX.Element {
  return (
    <span className="flex min-h-7 items-center">
      <Bar className={`h-2.5 ${width}`} />
    </span>
  );
}

/** The left column: Needs you now, one need card, the feed's filters and rows. */
function Left(): JSX.Element {
  return (
    <div className="flex w-full max-w-[720px] flex-col gap-5" data-testid="home-loading-left">
      <div className="flex flex-col gap-2">
        <Head width="w-28" />
        <div className="flex gap-3 rounded-xl bg-surface px-3.5 py-3" data-testid="home-loading-need">
          <span className="size-6 shrink-0 rounded-md bg-skeleton" />
          <span className="flex min-w-0 flex-1 flex-col gap-1">
            <Line text="text-meta" className="h-2.5 w-2/5" />
            <Line text="text-body" className="h-3 w-3/5" />
            <span className="flex gap-2 pt-1">
              <Bar className="h-7 w-28 rounded-lg" />
              <Bar className="h-7 w-32 rounded-lg" />
            </span>
          </span>
        </div>
      </div>
      <div className="flex flex-col gap-3" data-testid="home-loading-feed">
        <Bar className="h-9 w-full max-w-[500px] rounded-[10px]" />
        <div className="flex flex-col gap-1">
          {[0, 1, 2, 3].map((index) => (
            <div key={index} className="flex gap-3 px-3 py-2.5">
              <span className="size-5 shrink-0 rounded-md bg-skeleton" />
              <span className="flex flex-1 flex-col gap-1.5">
                <Bar className="h-3 w-40" />
                <Bar className={`h-3.5 ${index % 2 === 0 ? "w-4/5" : "w-3/5"}`} />
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** The right column: Your apps and Runway by account. */
function Right(): JSX.Element {
  return (
    <div className="flex min-w-0 flex-col gap-5" data-testid="home-loading-right">
      <div className="flex flex-col gap-2">
        <Head width="w-24" />
        <div className="flex flex-col gap-1 rounded-xl bg-surface p-2">
          {[0, 1, 2].map((index) => (
            <span key={index} className="flex items-center gap-3 px-2 py-1.5">
              <span className="size-4 shrink-0 rounded bg-skeleton" />
              <span className="flex min-w-0 flex-1 flex-col gap-1">
                <Line text="text-body" className={`h-3 ${index % 2 === 0 ? "w-2/5" : "w-1/2"}`} />
                <Line text="text-label" className="h-2 w-1/3" />
              </span>
              <Bar className="h-6 w-12 rounded-md" />
            </span>
          ))}
        </div>
      </div>
      <div className="flex flex-col gap-2">
        <Head width="w-36" />
        <div className="flex flex-col rounded-xl bg-surface px-4 py-1.5">
          {[0, 1].map((index) => (
            <span key={index} className="flex flex-col gap-1.5 py-2">
              <span className="flex items-center justify-between gap-3">
                <Line text="text-sm" className={`h-3 ${index === 0 ? "w-28" : "w-36"}`} />
                <Line text="text-body" className="h-3 w-14" />
              </span>
              <Bar className="h-1 w-full rounded-full" />
              <Line text="text-label" className="h-2 w-1/2" />
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

export function HomeLoading() {
  const { slow } = useLoadDeadline(true, "Home", { timeoutMs: Number.POSITIVE_INFINITY });
  const [layout, measure] = useHomeLayout();
  const stacked = layout !== "columns";
  return (
    <LoadingState
      state="loading"
      slow={slow}
      what="Home"
      skeleton={
        <output
          ref={measure}
          aria-label="Loading Home"
          aria-busy="true"
          className={"mx-auto flex w-full max-w-content flex-col gap-4 motion-safe:animate-pulse " + (stacked ? "pb-10" : "h-full min-h-0")}
          data-testid="home-loading"
          data-layout={layout}
        >
          {/* Home's heading ("5 need you  6 open"), its title's height. */}
          <span aria-hidden="true" className="flex h-[1lh] shrink-0 items-center text-title">
            <Bar className="h-6 w-56" />
          </span>
          {stacked ? (
            <div aria-hidden="true" className="flex flex-col gap-5">
              <Left />
              <Right />
            </div>
          ) : (
            <div aria-hidden="true" className={"grid min-h-0 flex-1 gap-6 " + COLUMNS} data-testid="home-loading-columns">
              <div className="min-w-0 overflow-hidden pr-1">
                <Left />
              </div>
              <div className="min-w-0 overflow-hidden pr-1">
                <Right />
              </div>
            </div>
          )}
        </output>
      }
      empty={null}
      // never asked for: the shell's banner has the Retry (see above)
      onRetry={() => undefined}
    >
      {null}
    </LoadingState>
  );
}

/**
 * D-118 (AJ-E3702) — Home's activity feed, the stream under the pinned
 * zone: the filters, then what happened, newest first, never ranked,
 * rolled up so it never spams (`lib/feed-rollup`), day separators as
 * section labels, the "While you were away" divider and digest (what
 * happened while away is rolled into it, never a card as well; above it
 * only what came since the return), every sentence from `lib/feed-words`.
 *
 * Paging (R8): three pages load by themselves as the end comes into view,
 * then Load more; the last visit's first page paints at once, or six
 * skeleton cards without one; an error keeps the cards and says when
 * they were loaded. Stacked (narrow, AJ-E3701 R4): one page's first 12
 * cards, then Show more, never loading by itself, so Apps and Runway stay
 * reachable below. The W3C feed pattern's roles are here (`role="feed"`,
 * articles with their place); AJ-E3707 finishes its keys and checks.
 */
import { Fragment, useCallback, useMemo, useRef, useState, type JSX } from "react";
import { HomeFeedCard, type CardActions } from "./HomeFeedCard";
import { HomeFeedAway, useLastLook } from "./HomeFeedAway";
import { HomeFeedFilters, type Facets } from "./HomeFeedFilters";
import { Button } from "./primitives";
import { cardsOf, dayStarts, splitAway, type FeedCard } from "../lib/feed-rollup";
import { dayLabel, EMPTY_FEED } from "../lib/feed-words";
import { markAllRead, markRead, readHorizon, readIds, unread, useFeedFilters } from "../lib/feed-state";
import { clock } from "../lib/time-words";
import { useFeedStream, type FeedStream } from "../lib/use-feed-stream";
import type { FeedItemInfo } from "../lib/ipc/feed";

/** Pages that load by themselves before Load more (NN/g). */
const AUTO_PAGES = 3;
/** Cards a stacked Home shows before Show more. */
const STACKED_CARDS = 12;

const GHOST = "btn-ghost self-start rounded-md px-3 py-1 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60";

function Skeleton(): JSX.Element {
  return (
    <div aria-hidden="true" className="flex flex-col gap-1" data-testid="feed-skeleton">
      {Array.from({ length: 6 }, (_, at) => (
        <div key={at} className="flex gap-3 px-3 py-2.5">
          <span className="size-5 shrink-0 rounded-md bg-skeleton motion-safe:animate-pulse" />
          <span className="flex flex-1 flex-col gap-1.5">
            <span className="h-3 w-40 rounded bg-skeleton motion-safe:animate-pulse" />
            <span className="h-3.5 w-4/5 rounded bg-skeleton motion-safe:animate-pulse" />
          </span>
        </div>
      ))}
    </div>
  );
}

/** Reads the next page when `active` and its end comes into view; looked
 * at again after each page (`read`: the items so far), so an end still in
 * view after a short page reads the next one too. */
function useAutoPage(active: boolean, loadMore: () => void, read: number): (node: HTMLElement | null) => void {
  const observer = useRef<IntersectionObserver | null>(null);
  return useCallback(
    (node: HTMLElement | null) => {
      observer.current?.disconnect();
      if (!node || !active || typeof IntersectionObserver === "undefined") return;
      observer.current = new IntersectionObserver((entries) => {
        if (entries.some((entry) => entry.isIntersecting)) loadMore();
      }, { rootMargin: "400px" });
      observer.current.observe(node);
    },
    // `read` is the point: a new page looks again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [active, loadMore, read],
  );
}

/** Loading, a failed read, or nothing yet (R8, R12). */
function States({ stream, quiet, onNewSession, onOpenApp }: { stream: FeedStream; quiet: boolean; onNewSession: () => void; onOpenApp: () => void }): JSX.Element | null {
  const { list } = stream;
  const retry = <button type="button" onClick={list.refresh} className="btn-ghost rounded-md px-2 py-0.5 font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60">Retry</button>;
  if (list.loading) return <Skeleton />;
  if (list.error !== null) {
    return (
      <output className="flex items-center gap-2 rounded-lg bg-surface px-3 py-2 text-meta text-ink-2" data-testid="feed-error">
        {list.items.length > 0 ? `Showing what was loaded${stream.loadedAt ? ` at ${clock(stream.loadedAt)}` : ""}.` : "The feed couldn't be read."}
        {retry}
      </output>
    );
  }
  if (list.items.length > 0) return null;
  return (
    <div className="flex flex-col items-start gap-3 rounded-xl bg-surface px-4 py-4" data-testid="feed-empty">
      <p className="text-body text-ink-2">{EMPTY_FEED}</p>
      <div className="flex gap-2">
        <Button size="sm" variant={quiet ? "primary" : "secondary"} onClick={onNewSession}>New session</Button>
        <Button size="sm" variant="ghost" onClick={onOpenApp}>Open an app</Button>
      </div>
    </div>
  );
}

/** The end of the list: Show more (stacked), the auto pages' sentinel,
 * then Load more. */
function Tail({ stream, stacked, more, hidden, onMore }: { stream: FeedStream; stacked: boolean; more: boolean; hidden: number; onMore: () => void }): JSX.Element {
  const { list } = stream;
  const auto = !stacked && list.hasMore && stream.pages < AUTO_PAGES && list.error === null;
  const sentinel = useAutoPage(auto, list.loadMore, list.items.length);
  const loadMore = list.hasMore && !auto && (!stacked || more) && list.error === null;
  return (
    <>
      {stacked && !more && hidden > 0 && (
        <button type="button" onClick={onMore} className={GHOST} data-testid="feed-show-more">Show more ({hidden})</button>
      )}
      {auto && <div ref={sentinel} aria-hidden="true" className="h-px" data-testid="feed-sentinel" />}
      {loadMore && (
        <button type="button" onClick={list.loadMore} disabled={list.loadingMore} className={GHOST} data-testid="feed-load-more">
          {list.loadingMore ? "Loading…" : "Load more"}
        </button>
      )}
    </>
  );
}

export function HomeFeed({ stacked, now, actions, nameOf, facets, toReview, onReviewAll, onNewSession, onOpenApp, scroller, quiet }: {
  stacked: boolean;
  /** A session's name as every list names it (when Home holds it). */
  nameOf: (item: FeedItemInfo) => string | null;
  now: number;
  actions: CardActions;
  facets: Facets;
  /** Sessions to review (the backend's count); `null` until counted. */
  toReview: number | null;
  onReviewAll: () => void;
  onNewSession: () => void;
  onOpenApp: () => void;
  /** The box the feed scrolls in. */
  scroller: () => HTMLElement | null;
  /** Nothing is pinned either: the empty feed's primary may be filled. */
  quiet: boolean;
}): JSX.Element {
  const [filters, setFilters] = useFeedFilters();
  const stream = useFeedStream(filters, scroller);
  // Show more holds for these filters only: a new choice starts at one page.
  const [moreFor, setMoreFor] = useState<FeedFiltersKey | null>(null);
  const more = moreFor === filters;
  const [read, setRead] = useState(readIds);
  const [horizon, setHorizon] = useState(() => readHorizon(now));
  const lastLook = useLastLook();
  const all = useMemo(() => cardsOf(stream.shown.map((item) => {
    const name = nameOf(item);
    return name ? { ...item, subject: name } : item;
  })), [stream.shown, nameOf]);
  // R5: what happened while away is the digest's, never a card as well.
  const { stream: cards, away } = useMemo(() => splitAway(all, lastLook, stream.list.hasMore), [all, lastLook, stream.list.hasMore]);
  const days = dayStarts(cards);
  const shownCards: FeedCard[] = stacked && !more ? cards.slice(0, STACKED_CARDS) : cards;
  const open = (item: FeedItemInfo): boolean => {
    markRead([item.id]);
    setRead(readIds());
    return actions.open(item);
  };
  const cardActions: CardActions = { ...actions, open };
  const readAll = () => {
    markAllRead(Date.now());
    setRead(readIds());
    setHorizon(Date.now());
  };
  return (
    <section aria-labelledby="activity-h" className="flex min-w-0 flex-col gap-3" data-testid="home-feed">
      <h2 id="activity-h" className="sr-only">Activity</h2>
      <HomeFeedFilters filters={filters} onChange={setFilters} facets={facets} toReview={toReview} onReviewAll={onReviewAll} onReadAll={readAll} />
      {stream.waiting > 0 && (
        <button type="button" onClick={stream.showNew} className="sticky top-1 z-10 self-center rounded-full bg-surface-3 px-3 py-1 text-meta font-medium text-ink shadow-card outline-none hover:bg-selected focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-new-pill">
          ↑ {stream.waiting} new {stream.waiting === 1 ? "update" : "updates"}
        </button>
      )}
      <States stream={stream} quiet={quiet} onNewSession={onNewSession} onOpenApp={onOpenApp} />
      {(cards.length > 0 || away) && (
        <div role="feed" aria-busy={stream.list.loadingMore || stream.list.stale} aria-labelledby="activity-h" className="flex flex-col gap-0.5" data-testid="feed">
          {shownCards.map((card, index) => (
            <Fragment key={card.key}>
              {away?.index === index && <HomeFeedAway away={away} now={now} onOpen={open} />}
              {days.has(index) && <p aria-hidden="true" className="section-label px-3 pb-1 pt-3" data-testid="feed-day">{dayLabel(card.head.at_ms, now)}</p>}
              <HomeFeedCard card={card} pos={index + 1} now={now} unread={unread(card.head, read, horizon)} fresh={stream.fresh(card.head)} actions={cardActions} />
            </Fragment>
          ))}
          {away?.index === cards.length && shownCards.length === cards.length && <HomeFeedAway away={away} now={now} onOpen={open} />}
        </div>
      )}
      <Tail stream={stream} stacked={stacked} more={more} hidden={cards.length - shownCards.length} onMore={() => setMoreFor(filters)} />
      <div aria-live="polite" className="sr-only" data-testid="feed-announce">{stream.announced}</div>
    </section>
  );
}

type FeedFiltersKey = ReturnType<typeof useFeedFilters>[0];

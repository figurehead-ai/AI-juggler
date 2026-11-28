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
import { HomeFeedFilters, stateOf as chipOf, tabOf, type Facets } from "./HomeFeedFilters";
import { Button } from "./primitives";
import { cardsOf, splitAway, type FeedCard } from "../lib/feed-rollup";
import { dayLabel, EMPTY_FEED } from "../lib/feed-words";
import { inState, STATES, TABS, type StateChip } from "../lib/feed-chips";
import { rank, useFeedLeans, type Section } from "../lib/feed-leans";
import { filtered, markAllRead, markRead, readHorizon, readIds, unread, useFeedFilters } from "../lib/feed-state";
import { clock } from "../lib/time-words";
import { usePagedState } from "../lib/paged-state";
import { LoadingState } from "./LoadingState";
import { useFeedStream, type FeedStream } from "../lib/use-feed-stream";
import type { FeedFiltersInfo, FeedItemInfo } from "../lib/ipc/feed";

/** Pages that load by themselves before Load more (NN/g). */
const AUTO_PAGES = 3;
/** Cards a stacked Home shows before Show more. */
const STACKED_CARDS = 12;

const GHOST = "btn-ghost self-start rounded-md px-3 py-1 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60";

/** The feed's rows while its first page is read. */
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

/** Whether the feed has nothing to show, read and unfiltered (0.5.1 F56,
 * D-127): its filters would have nothing to filter, so they hide, and
 * "Include my actions" moves into the empty card. A filtered empty feed
 * keeps them (the way back is there); so does one still loading. */
function feedIsEmpty(stream: Pick<FeedStream, "list" | "waiting">, filters: FeedFiltersInfo, away: boolean): boolean {
  const { list } = stream;
  return !list.loading && list.error === null && list.items.length === 0 && stream.waiting === 0 && !away && !filtered(filters);
}

/** Loading, a failed read, or nothing yet (R8, R12). */
function States({ stream, quiet, onNewSession, onOpenApp, includeMine, filteredAs, onShowAll }: { stream: FeedStream; quiet: boolean; onNewSession: () => void; onOpenApp: () => void; includeMine: (() => void) | null; filteredAs: string | null; onShowAll: () => void }): JSX.Element | null {
  const { list } = stream;
  // AJ-E1552: the skeleton only while the first page is read, Still
  // loading after 3 s; the error keeps its own words (what was loaded).
  const { state, slow } = usePagedState(list, "the activity feed");
  const retry = <button type="button" onClick={list.refresh} className="btn-ghost rounded-md px-2 py-0.5 font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60">Retry</button>;
  if (state === "loading") return <LoadingState state={state} slow={slow} what="the activity feed" skeleton={<Skeleton />} empty={null} onRetry={list.refresh}>{null}</LoadingState>;
  if (list.error !== null) {
    return (
      <output className="flex items-center gap-2 rounded-lg bg-surface px-3 py-2 text-meta text-ink-2" data-testid="feed-error">
        {list.items.length > 0 ? `Showing what was loaded${stream.loadedAt ? ` at ${clock(stream.loadedAt)}` : ""}.` : "The feed couldn't be read."}
        {retry}
      </output>
    );
  }
  if (list.items.length > 0) return null;
  // AJ-E3713 (usability answer 4): a filter with nothing says so and
  // offers All.
  if (filteredAs !== null) {
    return (
      <div className="flex flex-wrap items-center gap-2 rounded-xl bg-surface px-4 py-3" data-testid="feed-empty" data-filtered="yes">
        <p className="text-body text-ink-2">Nothing under {filteredAs} yet.</p>
        <Button size="sm" variant="secondary" onClick={onShowAll} data-testid="feed-show-all">Show all</Button>
      </div>
    );
  }
  return (
    <div className="flex flex-col items-start gap-3 rounded-xl bg-surface px-4 py-4" data-testid="feed-empty">
      <p className="text-body text-ink-2">{EMPTY_FEED}</p>
      <div className="flex gap-2">
        <Button size="sm" variant={quiet ? "primary" : "secondary"} onClick={onNewSession}>New session</Button>
        <Button size="sm" variant="ghost" onClick={onOpenApp}>Open an app</Button>
      </div>
      {/* F56: the hidden filters' one switch that can change an empty feed. */}
      {includeMine && (
        <p className="text-meta text-muted" data-testid="feed-empty-mine">
          {"Your own actions aren't shown. "}
          <button type="button" onClick={includeMine} className="rounded font-medium text-ink-2 underline decoration-hairline underline-offset-2 outline-none hover:text-ink hover:decoration-current focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-empty-include-mine">
            Include my actions
          </button>
        </p>
      )}
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

/** A section's label: Today, This morning (once it's afternoon),
 * Yesterday, then the date (the mock's day labels). */
function sectionOf(at: number, now: number): string {
  const label = dayLabel(at, now);
  return label === "Today" && new Date(now).getHours() >= 12 && new Date(at).getHours() < 12 ? "This morning" : label;
}

const START_OF_DAY = (now: number) => new Date(new Date(now).toDateString()).getTime();

/** Each state's count today, from the posts loaded: known only when what
 * is loaded reaches back past this morning (or is all there is), and,
 * with a state chip on, only that chip's (R5; a number said is measured). */
function countsOf(cards: readonly FeedCard[], now: number, more: boolean, on: StateChip | null, running: number | null): Record<StateChip, number | null> {
  const today = START_OF_DAY(now);
  const reaches = !more || cards.some((card) => card.head.at_ms < today);
  const count = (state: StateChip) => cards.filter((card) => card.head.at_ms >= today && inState(card.head.kind, state)).length;
  const known = (state: StateChip) => reaches && (on === null || on === state);
  return {
    running,
    finished: known("finished") ? count("finished") : null,
    failed: known("failed") ? count("failed") : null,
  };
}

/** How often each App and project comes up in the posts loaded (the
 * Showing picker's suggestions). */
function seenOf(items: readonly FeedItemInfo[]): Map<string, number> {
  const seen = new Map<string, number>();
  const add = (key: string) => seen.set(key, (seen.get(key) ?? 0) + 1);
  for (const item of items) {
    if (item.author.kind === "app" && item.author.folder) add(`app:${item.author.folder}`);
    const project = item.facts.project_id ?? item.target.project_id;
    if (project) add(`project:${project}`);
  }
  return seen;
}

/** One section as drawn: its label (when it starts a new one), its
 * posts with their places, and whether every post above was shown. */
interface Row {
  section: Section;
  label: string | null;
  cards: { card: FeedCard; pos: number }[];
  shown: number;
  room: boolean;
}

/** The sections as drawn within `budget` posts (stacked: 12, then Show
 * more), folded posts once shown, each post's place in the feed. */
function layOut(sections: readonly Section[], budget: number, unfolded: ReadonlySet<string>): { rows: Row[]; hidden: number; left: number } {
  const rows: Row[] = [];
  let left = budget;
  let pos = 0;
  let total = 0;
  for (const section of sections) {
    const label = section.key.slice(2);
    const room = left > 0;
    const shown = section.cards.slice(0, Math.max(0, left));
    left -= shown.length;
    total += section.cards.length;
    const folded = unfolded.has(section.key) ? section.folded : [];
    const cards = [...shown, ...folded].map((card) => ({ card, pos: ++pos }));
    // Each section is a new run (a day, or the same day past the away
    // divider): its label heads it.
    rows.push({ section, label: cards.length > 0 ? label : null, cards, shown: shown.length, room });
  }
  return { rows, hidden: Math.max(0, total - pos), left };
}

/** A section's posts a "less" folded: one quiet line, Show in place. */
function Folded({ cards, shown, onShow }: { cards: readonly FeedCard[]; shown: boolean; onShow: () => void }): JSX.Element | null {
  if (cards.length === 0 || shown) return null;
  const words = `${cards.length} ${cards.length === 1 ? "post" : "posts"} shown less`;
  // In the feed, as one of its articles (a feed holds articles only; the
  // 0.5.2 RW1 walk first saw this line, and axe its place).
  return (
    <article aria-label={words} className="flex">
      <button type="button" onClick={onShow} className={GHOST + " text-muted"} data-testid="feed-folded">
        {words} · Show
      </button>
    </article>
  );
}

export function HomeFeed({ stacked, now, actions, nameOf, facets, toReview, onReviewAll, onNewSession, onOpenApp, scroller, quiet, running = null }: {
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
  /** Sessions running now (the Running chip's count), `null` unknown. */
  running?: number | null;
}): JSX.Element {
  const [filters, setFilters] = useFeedFilters();
  const stream = useFeedStream(filters, scroller);
  const leans = useFeedLeans();
  // Show more holds for these filters only: a new choice starts at one page.
  const [moreFor, setMoreFor] = useState<FeedFiltersKey | null>(null);
  const more = moreFor === filters;
  const [read, setRead] = useState(readIds);
  const [horizon, setHorizon] = useState(() => readHorizon(now));
  // R3: a post just leaned stays in place (its why and Undo at hand);
  // a section's folded posts, once shown.
  const [pinned, setPinned] = useState<ReadonlySet<string>>(() => new Set());
  const [unfolded, setUnfolded] = useState<ReadonlySet<string>>(() => new Set());
  const lastLook = useLastLook();
  const all = useMemo(() => cardsOf(stream.shown.map((item) => {
    const name = nameOf(item);
    return name ? { ...item, subject: name } : item;
  })), [stream.shown, nameOf]);
  // R5: what happened while away is the digest's, never a card as well.
  const { stream: cards, away } = useMemo(() => splitAway(all, lastLook, stream.list.hasMore), [all, lastLook, stream.list.hasMore]);
  // AJ-E3713 R3: ranked by the leans within each section (the part since
  // the return apart from the part before the last look, so nothing
  // crosses the away divider).
  const sections: Section[] = useMemo(
    () => rank(cards, leans, pinned, (card) => `${away && lastLook && card.head.at_ms <= lastLook.backMs ? "b" : "a"}|${sectionOf(card.head.at_ms, now)}`),
    [cards, leans, pinned, away, lastLook, now],
  );
  const empty = feedIsEmpty(stream, filters, away !== null);
  const counts = countsOf(cards, now, stream.list.hasMore, chipOf(filters), running);
  const seen = useMemo(() => seenOf(stream.shown), [stream.shown]);
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
  const plan = layOut(sections, stacked && !more ? STACKED_CARDS : Number.POSITIVE_INFINITY, unfolded);
  const awayAt = away ? sections.filter((section) => section.key.startsWith("a|")).length : -1;
  const tab = tabOf(filters);
  const state = chipOf(filters);
  return (
    <section aria-labelledby="activity-h" className="flex min-w-0 flex-col gap-3" data-testid="home-feed">
      <h2 id="activity-h" className="sr-only">Activity</h2>
      {!empty && <HomeFeedFilters filters={filters} onChange={setFilters} facets={facets} seen={seen} counts={counts} toReview={toReview} onReviewAll={onReviewAll} onReadAll={readAll} />}
      {stream.waiting > 0 && (
        // 0.5.2 G63 (RW1 F-RW2-9): in a scrolled feed the pill sat on a
        // post's header line, the words running out on both sides of it.
        // It rides a band of the page's own colour, fading below, so the
        // post slides under it (Bot's conversation head, the same way).
        // 0.5.2 G70 (RW1 S15): in the app's web view a sliver of the post
        // showed above the band's top edge while it stuck; the band's
        // colour reaches 12 px above it (clipped by the column itself).
        <div className="pointer-events-none sticky top-0 z-10 -mb-6 flex justify-center bg-linear-to-b from-canvas from-60% to-transparent pb-6 pt-1 before:absolute before:inset-x-0 before:-top-3 before:h-3 before:bg-canvas before:content-['']" data-testid="feed-new-band">
          <button type="button" onClick={stream.showNew} className="pointer-events-auto rounded-full bg-surface-3 px-3 py-1 text-meta font-medium text-ink shadow-card outline-none hover:bg-selected focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-new-pill">
            ↑ {stream.waiting} new {stream.waiting === 1 ? "update" : "updates"}
          </button>
        </div>
      )}
      <States stream={stream} quiet={quiet} onNewSession={onNewSession} onOpenApp={onOpenApp} includeMine={empty && !filters.include_mine ? () => setFilters({ ...filters, include_mine: true }) : null}
        filteredAs={filtered(filters) ? [tab !== "all" ? TABS.find((one) => one.value === tab)?.label : null, state ? STATES.find((one) => one.value === state)?.label : null].filter(Boolean).join(" · ") || "this filter" : null}
        onShowAll={() => setFilters({ include_mine: filters.include_mine })} />
      {(cards.length > 0 || away) && (
        <div role="feed" aria-busy={stream.list.loadingMore || stream.list.stale} aria-labelledby="activity-h" className="flex flex-col gap-3" data-testid="feed">
          {plan.rows.map((row, at) => (
            <Fragment key={row.section.key}>
              {away && at === awayAt && row.room && <HomeFeedAway away={away} now={now} onOpen={open} />}
              {row.label !== null && <p aria-hidden="true" className="section-label px-0.5 pb-0 pt-1" data-testid="feed-day">{row.label}</p>}
              {row.cards.map(({ card, pos }) => (
                <HomeFeedCard key={card.key} card={card} pos={pos} now={now} unread={unread(card.head, read, horizon)} fresh={stream.fresh(card.head)} actions={cardActions}
                  onLeaned={(key) => setPinned((was) => new Set([...was, key]))}
                  onSettled={(key) => setPinned((was) => new Set([...was].filter((one) => one !== key)))} />
              ))}
              {row.shown > 0 && <Folded cards={row.section.folded} shown={unfolded.has(row.section.key)} onShow={() => setUnfolded((was) => new Set([...was, row.section.key]))} />}
            </Fragment>
          ))}
          {away && awayAt === sections.length && plan.left > 0 && <HomeFeedAway away={away} now={now} onOpen={open} />}
        </div>
      )}
      <Tail stream={stream} stacked={stacked} more={more} hidden={plan.hidden} onMore={() => setMoreFor(filters)} />
      {cards.length > 0 && !stacked && (
        <div className="flex flex-col items-center gap-1 pb-2 text-label text-faint" data-testid="feed-foot">
          {stream.list.hasMore && <p>Older posts load as you scroll</p>}
          <p className="self-start">
            <kbd className="rounded bg-surface-3 px-1.5 text-ink-2">j</kbd> <kbd className="rounded bg-surface-3 px-1.5 text-ink-2">k</kbd> move between posts ·{" "}
            <kbd className="rounded bg-surface-3 px-1.5 text-ink-2">l</kbd> more like this · <kbd className="rounded bg-surface-3 px-1.5 text-ink-2">m</kbd> less like this
          </p>
        </div>
      )}
      <div aria-live="polite" className="sr-only" data-testid="feed-announce">{stream.announced}</div>
    </section>
  );
}

type FeedFiltersKey = ReturnType<typeof useFeedFilters>[0];

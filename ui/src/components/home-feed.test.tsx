/**
 * D-118 (AJ-E3702) vitest — Home's feed as a reader meets it: the W3C
 * feed pattern's roles; the filters (remembered, said when on, never
 * hiding a need); paging (three pages by themselves, then Load more,
 * never more than one page a request; stacked: one page, Show more); the
 * cached first page painting at once; a failed read keeping the cards;
 * seen apart from read; the away divider and digest at the last look.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeFeed } from "./HomeFeed";
import { feedItem } from "../preview/feed";
import { keepFirst, keptFilters, markRead, readHorizon, readIds, unread } from "../lib/feed-state";
import type { FeedItemInfo } from "../lib/ipc/feed";

const NOW = Date.now();
const HOUR = 3_600_000;

/** `count` finished sessions, each its own card (another App each). */
function many(count: number, from = NOW - 60_000): FeedItemInfo[] {
  return Array.from({ length: count }, (_, n) =>
    feedItem({ id: `l:${n}`, at_ms: from - n * 60_000, kind: "finished", category: "finished", level: 1, coalesce: `s:${n}`, rollup: `v:finished:f${n}:-:L1`, rollup_window_ms: 2 * HOUR, scope: `f:f${n}`, subject: `Session ${n}`, target: { session_id: `s${n}` } }),
  );
}

const seed = (items: FeedItemInfo[]) => Reflect.set(window, "__AIJ_FEED__", items);
const calls = (): unknown[] => {
  const kept: unknown = Reflect.get(window, "__AIJ_FEED_CALLS__");
  return Array.isArray(kept) ? kept : [];
};
const limits = () => calls().map((call) => (typeof call === "object" && call !== null ? Reflect.get(call, "limit") : null));

/** IntersectionObserver that sees every node at once (the end in view). */
class SeeAll implements IntersectionObserver {
  readonly root = null;
  readonly rootMargin = "0px";
  readonly scrollMargin = "0px";
  readonly thresholds: readonly number[] = [0];
  constructor(private readonly callback: IntersectionObserverCallback) {}
  observe(node: Element): void {
    const box = node.getBoundingClientRect();
    const entry: IntersectionObserverEntry = { boundingClientRect: box, intersectionRatio: 1, intersectionRect: box, isIntersecting: true, rootBounds: null, target: node, time: 0 };
    queueMicrotask(() => this.callback([entry], this));
  }
  disconnect(): void {}
  unobserve(): void {}
  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }
}

function feed({ stacked = false, scroller = () => null as HTMLElement | null, open = vi.fn(() => true) } = {}) {
  return render(
    <HomeFeed stacked={stacked} now={NOW} nameOf={() => null} quiet onNewSession={vi.fn()} onOpenApp={vi.fn()} toReview={0} onReviewAll={vi.fn()} scroller={scroller}
      facets={{ projects: [], accounts: [], subscriptions: [], pools: null }} actions={{ open, review: vi.fn(), railOf: () => null }} />,
  );
}

beforeEach(() => vi.stubGlobal("IntersectionObserver", SeeAll));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(window, "__AIJ_FEED_FAILS__");
  Reflect.deleteProperty(window, "__AIJ_SUMMARY__");
});

describe("D-118 — the feed", () => {
  it("is a role=feed of articles, each with its place, labelled by its author and sentence", async () => {
    seed(many(3));
    feed();
    const list = await screen.findByRole("feed", { name: "Activity" });
    const articles = within(list).getAllByRole("article");
    expect(articles).toHaveLength(3);
    expect(articles.map((one) => one.getAttribute("aria-posinset"))).toEqual(["1", "2", "3"]);
    expect(articles[0]!.getAttribute("aria-setsize")).toBe("-1");
    expect(articles[0]!.getAttribute("aria-labelledby")).toMatch(/-who .*-what$/);
  });

  it("pages three times by itself as its end comes into view, then waits for Load more: one page a request", async () => {
    seed(many(200));
    feed();
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(90));
    expect(await screen.findByTestId("feed-load-more")).toBeTruthy();
    expect(calls()).toHaveLength(3);
    expect(limits().every((limit) => limit === 30)).toBe(true);
    fireEvent.click(screen.getByTestId("feed-load-more"));
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(120));
    expect(calls()).toHaveLength(4);
  });

  it("stacked: one page's first 12 cards, then Show more; never paging by itself", async () => {
    seed(many(200));
    feed({ stacked: true });
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(12));
    expect(screen.queryByTestId("feed-sentinel")).toBeNull();
    expect(calls()).toHaveLength(1);
    fireEvent.click(screen.getByTestId("feed-show-more"));
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(30));
    expect(screen.getByTestId("feed-load-more")).toBeTruthy();
    expect(calls()).toHaveLength(1);
  });

  it("filters by chip, says so in a muted line with ×, and remembers it per viewer", async () => {
    seed([...many(2), feedItem({ id: "w:1", at_ms: NOW - 30_000, kind: "switch", category: "switches", subject: "Move it" })]);
    feed();
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(3));
    fireEvent.click(screen.getByRole("radio", { name: "Switches & limits" }));
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(1));
    expect(screen.getByTestId("feed-filtered").textContent).toContain("Filtered: Switches & limits");
    expect(keptFilters().category).toBe("switches");
    fireEvent.click(screen.getByRole("button", { name: "Show everything" }));
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(3));
    expect(screen.queryByTestId("feed-filtered")).toBeNull();
  });

  it("paints the last visit's first page at once, then the live one", async () => {
    seed(many(2));
    keepFirst(`home-feed:${JSON.stringify(keptFilters())}`, { items: many(1, NOW - 10 * HOUR).map((one) => ({ ...one, id: "cached", subject: "From last time" })), next_cursor: null, total: null });
    feed();
    // The first render, before any read answers: the cached card.
    expect(screen.getAllByText("From last time").length).toBeGreaterThan(0);
    await waitFor(() => expect(screen.queryAllByText("From last time")).toHaveLength(0));
    expect(screen.getAllByTestId("feed-card")).toHaveLength(2);
  });

  it("keeps what was loaded when a read fails, says when, and retries", async () => {
    seed(many(40));
    feed();
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(40));
    Reflect.set(window, "__AIJ_FEED_FAILS__", true);
    const { refreshPaged } = await import("../lib/paged");
    await act(async () => refreshPaged("home-feed:"));
    await waitFor(() => expect(screen.getByTestId("feed-error").textContent).toMatch(/^Showing what was loaded at .+Retry$/));
    expect(screen.getAllByTestId("feed-card")).toHaveLength(40);
    Reflect.deleteProperty(window, "__AIJ_FEED_FAILS__");
    fireEvent.click(within(screen.getByTestId("feed-error")).getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.queryByTestId("feed-error")).toBeNull());
  });

  it("says when it's quiet, with New session and Open an app", async () => {
    seed([]);
    feed();
    expect((await screen.findByTestId("feed-empty")).textContent).toContain("Quiet so far.");
  });

  it("opening an item marks it read (its dot clears); seen is the away boundary, apart", async () => {
    const horizon = readHorizon(NOW - HOUR);
    expect(horizon).toBe(NOW - HOUR);
    seed(many(2));
    const open = vi.fn(() => true);
    feed({ open });
    await waitFor(() => expect(screen.getAllByTestId("feed-unread")).toHaveLength(2));
    fireEvent.click(screen.getAllByTestId("feed-subject")[0]!);
    expect(open).toHaveBeenCalledOnce();
    await waitFor(() => expect(screen.getAllByTestId("feed-unread")).toHaveLength(1));
    expect(readIds().has("l:0")).toBe(true);
    markRead(["l:1"]);
    expect(unread({ id: "l:1", at_ms: NOW }, readIds(), horizon)).toBe(false);
    expect(unread({ id: "old", at_ms: NOW - 2 * HOUR }, readIds(), horizon)).toBe(false);
  });

  it("shows the away divider and digest at the last look, 2 h or more ago", async () => {
    Reflect.set(window, "__AIJ_SUMMARY__", { since_ms: NOW - 5 * HOUR, generated_ms: NOW, default_window: false, rows: [], counts: {}, leftovers: 0 });
    seed([...many(2), ...many(1, NOW - 6 * HOUR).map((one) => ({ ...one, id: "old", coalesce: "s:old" }))]);
    feed();
    const away = await screen.findByTestId("feed-away");
    expect(within(away).getByTestId("feed-away-divider").textContent).toMatch(/^While you were away · since /);
    expect(within(away).getByTestId("feed-digest-words").textContent).toBe("2 finished.");
    // It sits after what happened since, before what was seen.
    const cards = screen.getAllByTestId("feed-card");
    expect(cards[1]!.compareDocumentPosition(away) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(away.compareDocumentPosition(cards[2]!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(away).getByTestId("feed-digest-seen"));
    });
    expect(within(away).getByTestId("feed-digest-seen").textContent).toBe("Seen");
  });

  it("holds new items behind the pill while the reader is scrolled down", async () => {
    const box = document.createElement("div");
    document.body.appendChild(box);
    seed(many(3));
    feed({ scroller: () => box });
    await waitFor(() => expect(screen.getAllByTestId("feed-card")).toHaveLength(3));
    box.scrollTop = 400;
    fireEvent.scroll(box);
    seed([...many(1, NOW).map((one) => ({ ...one, id: "new", coalesce: "s:new", subject: "Brand new" })), ...many(3)]);
    const { refreshPaged } = await import("../lib/paged");
    await act(async () => refreshPaged("home-feed:"));
    const pill = await screen.findByTestId("feed-new-pill");
    expect(pill.textContent).toBe("↑ 1 new update");
    expect(screen.queryByText("Brand new")).toBeNull();
    fireEvent.click(pill);
    expect(await screen.findByText("Brand new")).toBeTruthy();
    box.remove();
  });
});

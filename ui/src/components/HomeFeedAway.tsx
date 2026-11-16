/**
 * D-118 (AJ-E3702 R5, R9) — "While you were away": when the last look
 * (`summary_seen_at_ms`, the key the morning summary kept, read through
 * its own command) is 2 h or more ago, the feed's only rule line marks
 * the boundary, and a digest card under it says what happened since:
 * counts by kind and the three most important things, "Show all" in
 * place and "Mark all seen". It replaces the morning summary card.
 *
 * Seen (not read): the boundary moves on "Mark all seen", or once Home
 * has been in front 10 s with the digest on screen. The card stays for
 * this visit; the next one starts from the new boundary.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { overnightSummary, overnightSummarySeen } from "../lib/ipc/park";
import { sharedRead, useSharedRead } from "../lib/shared-read";
import { clock } from "../lib/time-words";
import { digestWords, exactWhen, feedWhen, said, sentence } from "../lib/feed-words";
import { stateOf, type Away } from "../lib/feed-rollup";
import type { FeedItemInfo } from "../lib/ipc/feed";

/** The last look, kept between Home's openings. */
const summary = sharedRead(() => overnightSummary());

/** In front this long with the digest on screen, the boundary moves. */
const SEEN_AFTER_MS = 10_000;

/** When the user last looked: `null` when none was ever kept (the
 * summary's default window is not a look). Marking it seen doesn't read
 * it again: the card stays for this visit, and the next one reads the new
 * boundary as Home opens. */
export function useLastLook(): number | null {
  const read = useSharedRead(summary);
  return read && !read.default_window ? read.since_ms : null;
}

function markSeen(): void {
  void overnightSummarySeen().catch(() => undefined);
}

/** The divider's words: "While you were away · since 9:12 PM". */
export function awayWords(sinceMs: number, now: number): string {
  const sameDay = new Date(sinceMs).toDateString() === new Date(now).toDateString();
  return `While you were away · since ${sameDay ? clock(sinceMs) : feedWhen(sinceMs, now)}`;
}

export function HomeFeedAway({ away, now, onOpen }: { away: Away; now: number; onOpen: (item: FeedItemInfo) => boolean }): JSX.Element {
  const [all, setAll] = useState(false);
  const [seen, setSeen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const node = box.current;
    if (seen || !node || typeof IntersectionObserver === "undefined") return undefined;
    let timer = 0;
    const observer = new IntersectionObserver(([entry]) => {
      window.clearTimeout(timer);
      if (entry?.isIntersecting && document.visibilityState === "visible") {
        timer = window.setTimeout(() => {
          setSeen(true);
          markSeen();
        }, SEEN_AFTER_MS);
      }
    });
    observer.observe(node);
    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
    };
  }, [seen]);
  const things = all ? away.all : away.top;
  return (
    <article aria-label={`${awayWords(away.sinceMs, now)}: ${digestWords(away.kinds)}`} className="flex flex-col gap-2 py-1" data-testid="feed-away">
      <p aria-hidden="true" className="section-label flex items-center gap-2.5" data-testid="feed-away-divider">
        <span aria-hidden="true" className="h-px w-3.5 shrink-0 bg-hairline" />
        <span aria-hidden="true" className="shrink-0">{awayWords(away.sinceMs, now)}</span>
        <span aria-hidden="true" className="h-px flex-1 bg-hairline" />
      </p>
      <div ref={box} className="flex flex-col gap-2 rounded-xl bg-surface px-4 py-3" data-testid="feed-digest">
        <p className="text-[0.84375rem] text-ink" data-testid="feed-digest-words">{digestWords(away.kinds)}.</p>
        <ul className="flex flex-col gap-1">
          {things.map((thing) => {
            const item = stateOf(thing);
            return (
              <li key={thing.key} className="flex items-baseline gap-3 text-meta text-ink-2">
                <button type="button" onClick={() => void onOpen(item)} className="min-w-0 rounded text-left outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-digest-item">
                  {said(sentence(item))}
                </button>
                <time className="ml-auto shrink-0 text-label text-muted" title={exactWhen(item.at_ms)}>{feedWhen(item.at_ms, now)}</time>
              </li>
            );
          })}
        </ul>
        <div className="flex items-center gap-1">
          {away.all.length > away.top.length && (
            <button type="button" aria-expanded={all} onClick={() => setAll((was) => !was)} className="btn-ghost rounded-md px-2 py-0.5 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-digest-all">
              {all ? "Show fewer" : `Show all ${away.all.length}`}
            </button>
          )}
          <button type="button" disabled={seen} onClick={() => { setSeen(true); markSeen(); }} className="btn-ghost rounded-md px-2 py-0.5 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60 disabled:opacity-60" data-testid="feed-digest-seen">
            {seen ? "Seen" : "Mark all seen"}
          </button>
        </div>
      </div>
    </article>
  );
}

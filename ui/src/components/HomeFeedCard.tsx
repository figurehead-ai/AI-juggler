/**
 * D-118, AJ-E3713 — one post of Home's feed, as the approved story-feed
 * mock draws it (`AIJuggler-design-refs/story-feed`), calm (round 2): a
 * surface on the canvas, no border, no shadow (PAT-SURF-1, -2).
 *
 *   [icon]  Claude app · work-max · 9 min
 *           work-max finished [Release checks] in web-app in 42 min.
 *           ┌ an attachment: a switch's before → after, the agent's
 *           └ reply quoted, the room an Account has left
 *           │ ↳ 2 min later · [Login refactor] finished.
 *           ─────────────────────────────────────────────────
 *           ↑  ↓  ⏰  +  💬                              ↗ Open
 *
 * The author is the App's own icon (read at run time) or AI Juggler's
 * mark for its own posts. The story is `lib/feed-story`'s, its things
 * chips that open them. Follow-ups of the same session or task thread
 * under the post instead of new rows. A rollup tells its count and lists
 * its things in place. The unread dot (6 px, ink-2, never the accent)
 * clears once it's read.
 */
import { useState, type JSX } from "react";
import { HomeFeedDigest } from "./HomeFeedDigest";
import { POST, PostAttachment, PostAvatar, PostThread, Story } from "./HomeFeedParts";
import { GestureBar, WhyPanel } from "./HomeFeedGestures";
import { TurnReview } from "./TurnReview";
import { exactWhen, feedWhen, rollupSentence, said, sentence } from "../lib/feed-words";
import { attachmentOf, changesOf, postAuthor, story, type Attachment, type PostAuthor, type StoryPart } from "../lib/feed-story";
import { stateOf, threadOf, type FeedCard, type FeedThing } from "../lib/feed-rollup";
import { needWord, type RailItem } from "../lib/rail-items";
import type { FeedItemInfo } from "../lib/ipc/feed";
import type { ReviewAsk } from "../lib/ipc/turn-review";
import { useAiJob } from "../lib/ai-availability";

/** What a post's controls do (Home's handlers). */
export interface CardActions {
  /** Opens an item's thing; false when it has none to open. */
  open: (item: FeedItemInfo) => boolean;
  /** Marks a finished session reviewed. */
  review: (sessionId: string) => void;
  /** The rail's row for an item's session, when it is one. */
  railOf: (item: FeedItemInfo) => RailItem | null;
  /** The rail's row for a session it lists, live or ended (a digest's
   * lines name each as the rail does, in its state now: 0.5.2 G21). */
  sessionOf?: (sessionId: string) => RailItem | null;
}

/** The kinds that are a session's ending (Home's e archives one). */
const ENDED = new Set(["finished", "failed", "crashed", "ended", "stopped"]);

/** A rollup shows this many of its things before "Show all". */
const SAMPLES = 3;

/** When a card happened: a rollup from its oldest to its newest. */
export function whenOf(card: FeedCard, now: number): string {
  const newest = card.head.at_ms;
  const oldest = card.things[card.things.length - 1]!.head.at_ms;
  // AJ-E2314 L5: never "2 min–2 min": a range only when its ends read apart.
  const [from, to] = [feedWhen(oldest, now), feedWhen(newest, now)];
  return card.rolled && from !== to ? `${from}–${to}` : to;
}

/** The item's pool, as a thing to open (a pool chip). */
function poolOf(item: FeedItemInfo): FeedItemInfo {
  const pool = item.target.pool_id ?? item.facts.pool_id;
  return { ...item, target: { ...item.target, session_id: null, conversation_id: null, pool_id: pool } };
}

/** A rollup's sentence (`feed-words`) as a story's, its author first
 * when it starts with a verb ("Claude Code CLI finished 3 sessions"). */
function asStory(parts: ReturnType<typeof sentence>, author: PostAuthor): StoryPart[] {
  const told: StoryPart[] = parts.map((part) => (part.strong ? { text: part.text, strong: true } : { text: part.text }));
  const first = told[0]?.text ?? "";
  return /^[a-z]/.test(first) ? [{ text: `${author.name} ` }, ...told] : told;
}

/** The calm line: the author in bold, its profile, when (the exact
 * time in its name). */
function Who({ id, author, at, when, unread }: { id: string; author: PostAuthor; at: number; when: string; unread: boolean }): JSX.Element {
  return (
    <p className="flex min-w-0 items-baseline gap-1.5 text-meta text-muted">
      <span id={`${id}-who`} className="flex min-w-0 items-baseline gap-1.5">
        <b className="shrink-0 font-semibold text-ink">{author.name}</b>
        {author.place && <span className="truncate">· {author.place}</span>}
        {unread && <span className="sr-only">, unread</span>}
      </span>
      <time className="shrink-0" dateTime={new Date(at).toISOString()} title={exactWhen(at)} aria-label={exactWhen(at)}>· {when}</time>
    </p>
  );
}

/** Mark reviewed, on a finished session's post (A runs it). */
function Review({ sessionId, subject, onReview }: { sessionId: string; subject: string; onReview: (id: string) => void }): JSX.Element {
  return (
    <button type="button" data-home-action onClick={() => onReview(sessionId)} className="inline-flex h-7 items-center rounded-[7px] px-2 text-label outline-none hover:bg-surface-3 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-review">
      Mark reviewed<span className="sr-only"> {subject}</span>
    </button>
  );
}

/** The kinds that end a turn a review can be about (AJ-E3820). */
const TURN_ENDS = new Set(["finished", "failed", "crashed", "ended", "stopped"]);

/** AJ-E3820 R3, R5: the post's turn, as its review asks for it, when the
 * review covers it: a turn's end of a command-line App whose tool the
 * review job covers (the AI query says which), with its session or
 * conversation. */
function reviewAskOf(card: FeedCard, post: FeedItemInfo, covers: readonly string[] | undefined): ReviewAsk | null {
  const provider = post.author.provider;
  if (card.rolled || !TURN_ENDS.has(post.kind) || !provider || post.author.desktop || !covers?.includes(provider)) return null;
  const { session_id, account_id, conversation_id } = post.target;
  if (!session_id && !(account_id && conversation_id)) return null;
  return { session_id, account_id, conversation_id, until_ms: post.at_ms, ended: post.kind };
}

/** The post's turn review, when the review covers it (the AI query's
 * `covers`). */
function PostReview({ card, post }: { card: FeedCard; post: FeedItemInfo }): JSX.Element | null {
  const ask = reviewAskOf(card, post, useAiJob("review")?.covers);
  return ask ? <TurnReview ask={ask} quote /> : null;
}

/** A rollup's things: three, then Show all in place. */
function Rolled({ card, now, actions }: { card: FeedCard; now: number; actions: CardActions }): JSX.Element {
  const [all, setAll] = useState(false);
  const shown = all ? card.things : card.things.slice(0, SAMPLES);
  return (
    <div className="mb-2 ml-0.5 border-l-2 border-divider py-0.5 pl-3">
      <ul className="flex flex-col gap-0.5" data-testid="feed-rollup">
        {shown.map((thing: FeedThing) => {
          const item = stateOf(thing);
          return (
            <li key={thing.key} className="flex min-w-0 items-baseline gap-3 text-meta text-ink-2" data-testid="feed-rollup-thing">
              {/* A whole 24 px target (WCAG 2.5.8). */}
              <button type="button" onClick={() => actions.open(item)} className="min-h-6 min-w-0 truncate rounded text-left outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">{item.subject || said(sentence(item))}</button>
              <time className="ml-auto shrink-0 text-label text-muted" dateTime={new Date(item.at_ms).toISOString()} title={exactWhen(item.at_ms)}>{feedWhen(item.at_ms, now)}</time>
            </li>
          );
        })}
      </ul>
      {card.things.length > SAMPLES && (
        <button type="button" aria-expanded={all} onClick={() => setAll((was) => !was)} className="rounded text-label text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-rollup-all">
          {all ? "Show fewer" : `Show all ${card.things.length}`}
        </button>
      )}
    </div>
  );
}

/** What a card shows beside its words: a rollup its things' diff stat
 * (AJ-E3713 R6a), a post its own attachment, its follow-ups' turns
 * counted in. */
function attachmentOfCard(card: FeedCard, post: FeedItemInfo, follow: readonly FeedItemInfo[] = []): Attachment | null {
  return card.rolled ? changesOf(card.things.flatMap((thing) => thing.items)) : attachmentOf(post, follow);
}

export function HomeFeedCard({ card, pos, now, unread, fresh, actions, onLeaned, onSettled }: {
  card: FeedCard;
  pos: number;
  now: number;
  unread: boolean;
  fresh: boolean;
  actions: CardActions;
  /** A lean was kept on this post (it stays in place with its why). */
  onLeaned?: (key: string) => void;
  /** Its why was closed: the lean now acts on it too (0.5.2 G24: a
   * "less" post folds, a "more" one rises). */
  onSettled?: (key: string) => void;
}): JSX.Element {
  const [why, setWhy] = useState(false);
  const [allThread, setAllThread] = useState(false);
  const item = card.head;
  // AJ-E3712: the agent digest, its own post: a line per session.
  if (item.kind === "agent_digest" && item.digest) {
    const live = (sessionId: string) => {
      const row = actions.sessionOf?.(sessionId);
      return row ? { name: row.title, state: needWord(row) } : null;
    };
    return <HomeFeedDigest item={item} digest={item.digest} when={feedWhen(item.at_ms, now)} pos={pos} live={live} onOpen={(sessionId) => void actions.open({ ...item, target: { ...item.target, session_id: sessionId } })} />;
  }
  const id = `feed-${item.id.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
  const thread = card.rolled ? null : threadOf(card.things[0]!);
  const post = thread?.post ?? item;
  const author = postAuthor(post);
  const parts = card.rolled ? asStory(rollupSentence(item.kind, card.things.length, item), author) : story(post);
  const attachment = attachmentOfCard(card, post, thread?.follow);
  const open = () => void actions.open(item);
  const finished = !card.rolled && item.kind === "finished" ? item.target.session_id : null;
  const when = card.rolled ? whenOf(card, now) : feedWhen(post.at_ms, now);
  return (
    <article
      aria-labelledby={`${id}-who ${id}-what`}
      aria-posinset={pos}
      aria-setsize={-1}
      tabIndex={-1}
      data-feed-item
      data-home-item
      data-session-id={card.rolled ? undefined : (item.target.session_id ?? undefined)}
      data-ended={!card.rolled && ENDED.has(item.kind) ? "yes" : undefined}
      data-testid="feed-card"
      // Every item it shows (the tests hold that none is a digest line as well).
      data-feed-ids={card.things.flatMap((thing) => thing.items.map((one) => one.id)).join(" ")}
      data-kind={item.kind}
      data-level={card.level}
      data-rolled={card.rolled || undefined}
      className={POST + (fresh ? " feed-fresh" : "")}
    >
      {unread && <span aria-hidden="true" className="absolute left-1.5 top-5 size-1.5 rounded-full bg-ink-2" data-testid="feed-unread" />}
      <PostAvatar item={post} author={author} />
      <div className="min-w-0">
        <Who id={id} author={author} at={post.at_ms} when={when} unread={unread} />
        <p id={`${id}-what`} className="mb-2 mt-[3px] text-body leading-normal text-ink" data-testid="feed-sentence">
          <Story parts={parts} onOpen={open} onPool={() => void actions.open(poolOf(post))} />
        </p>
        {attachment && <PostAttachment attachment={attachment} />}
        <PostReview card={card} post={post} />
        {card.rolled && <Rolled card={card} now={now} actions={actions} />}
        {thread && thread.follow.length > 0 && (
          <PostThread
            from={post.at_ms}
            follow={thread.follow}
            shownAll={allThread}
            onAll={() => setAllThread(true)}
            onOpen={(one) => void actions.open(one)}
            onPool={(one) => void actions.open(poolOf(one))}
          />
        )}
      </div>
      <GestureBar
        item={post}
        parts={parts}
        why={why}
        onOpen={open}
        onLeaned={() => {
          setWhy(true);
          onLeaned?.(card.key);
        }}
        onWhy={() => {
          if (why) onSettled?.(card.key);
          setWhy(!why);
        }}
        extra={finished ? <Review sessionId={finished} subject={item.subject} onReview={actions.review} /> : null}
      />
      {why && <WhyPanel item={post} />}
    </article>
  );
}

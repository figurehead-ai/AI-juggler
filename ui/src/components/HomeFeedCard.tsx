/**
 * D-118 (AJ-E3702 R3, R4, R11) — one card of Home's activity feed, as the
 * proposal's §2 draws it, calm (round 2): no border, the surface tone on
 * hover and focus only.
 *
 *   ●  [icon]  Claude Code CLI · Work Max  web-app            12 min
 *              finished **Fix the flaky login test** in 42 min
 *              "…the agent's words…"               (a pool message)
 *              facts line (muted)
 *              Open · Mark reviewed · ↗         (on hover, focus, selected)
 *
 * The unread dot (6 px, ink-2, never the accent) clears once it's read.
 * The App's icon is in its tone (muted at rest, D-117). In the stream
 * every item is a quiet line: a need's loud mark is the pinned zone's
 * alone. A rollup opens in place to every thing it holds. Every card opens
 * its thing; ↗ goes to its App where it runs.
 */
import { useState, type JSX } from "react";
import { AccountAvatar } from "./Avatar";
import { CliAppMark } from "./PlaceMarks";
import { IconJuggler } from "./icons";
import { useItemMenu } from "./ItemMenu";
import { authorWords, exactWhen, factsLine, feedWhen, placeWords, rollupSentence, said, sentence, type Part } from "../lib/feed-words";
import { restartOf, stateOf, type FeedCard, type FeedThing } from "../lib/feed-rollup";
import { goToWhere, type RailItem } from "../lib/rail-items";
import type { FeedItemInfo } from "../lib/ipc/feed";

/** What a card's controls do (Home's handlers). */
export interface CardActions {
  /** Opens an item's thing; false when it has none to open. */
  open: (item: FeedItemInfo) => boolean;
  /** Marks a finished session reviewed. */
  review: (sessionId: string) => void;
  /** The rail's row for an item's session, when it is one (its ↗). */
  railOf: (item: FeedItemInfo) => RailItem | null;
}

const GHOST = "btn-ghost rounded-md px-2 py-0.5 text-meta font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60";

function Avatar({ item }: { item: FeedItemInfo }): JSX.Element {
  const { author } = item;
  if (author.kind === "app" && author.provider && author.folder) {
    return <span className="app-tone inline-flex"><CliAppMark tool={author.provider} folderId={author.folder} folder={author.folder_label ?? ""} size={20} ring="var(--color-canvas)" /></span>;
  }
  // D-118's mock: an Account stands alone on its row (no App beside it),
  // so it wears its own colour, a disc, as on the Runway (Avatar's rule).
  if (author.kind === "account" && author.folder) return <AccountAvatar account={{ folder: author.folder }} label={author.name ?? ""} size={20} tone="colour" />;
  if (author.kind === "agent" || author.kind === "you") {
    const letters = (author.name ?? (author.kind === "you" ? "You" : "?")).replace(/[^\p{L}\p{N}]/gu, "").slice(0, 2).toLowerCase();
    return <span aria-hidden="true" className="flex size-5 shrink-0 items-center justify-center rounded-full bg-surface-3 text-[10px] font-semibold text-ink-2">{letters}</span>;
  }
  return <span aria-hidden="true" className="app-tone flex size-5 shrink-0 items-center justify-center text-muted"><IconJuggler width={16} height={16} /></span>;
}

/** A sentence's parts; the object's name opens its thing. */
function Said({ parts, onOpen }: { parts: readonly Part[]; onOpen: (() => void) | null }): JSX.Element {
  return (
    <>
      {parts.map((part, at) =>
        part.strong ? (
          onOpen ? (
            <button key={at} type="button" onClick={onOpen} className="rounded font-semibold text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-subject">{part.text}</button>
          ) : (
            <strong key={at} className="font-semibold text-ink">{part.text}</strong>
          )
        ) : (
          <span key={at}>{part.text}</span>
        ),
      )}
    </>
  );
}

/** ↗: its App forward, where its session runs (the rail's row's go-to). */
function GoTo({ row }: { row: RailItem }): JSX.Element | null {
  if (row.session) return <SessionGoTo row={row} session={row.session} />;
  return row.conversation ? <ConversationGoTo row={row} conversation={row.conversation} /> : null;
}

function SessionGoTo({ row, session }: { row: RailItem; session: NonNullable<RailItem["session"]> }): JSX.Element | null {
  const menu = useItemMenu("session", session, row.title);
  const where = goToWhere(row);
  return where === null ? null : <>{menu.goTo({ where, shown: true })}{menu.element}</>;
}

function ConversationGoTo({ row, conversation }: { row: RailItem; conversation: NonNullable<RailItem["conversation"]> }): JSX.Element | null {
  const menu = useItemMenu("conversation", conversation, row.title);
  const where = goToWhere(row);
  return where === null ? null : <>{menu.goTo({ where, shown: true })}{menu.element}</>;
}

function Thing({ thing, now, actions }: { thing: FeedThing; now: number; actions: CardActions }): JSX.Element {
  const item = stateOf(thing);
  return (
    <li className="flex min-w-0 items-baseline gap-3 text-meta text-ink-2" data-testid="feed-rollup-thing">
      <button type="button" onClick={() => actions.open(item)} className="min-w-0 truncate rounded text-left outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">{item.subject || said(sentence(item))}</button>
      <time className="ml-auto shrink-0 text-label text-muted" dateTime={new Date(item.at_ms).toISOString()} title={exactWhen(item.at_ms)}>{feedWhen(item.at_ms, now)}</time>
    </li>
  );
}

/** The kinds that are a session's ending (Home's e archives one). */
const ENDED = new Set(["finished", "failed", "crashed", "ended", "stopped"]);

/** A rollup shows this many of its things before "Show all". */
const SAMPLES = 3;

/** The author line: who, where, when (the exact time in its name). */
function Who({ id, item, when, unread }: { id: string; item: FeedItemInfo; when: string; unread: boolean }): JSX.Element {
  const place = placeWords(item);
  return (
    <p className="flex min-w-0 items-baseline gap-1.5 text-meta">
      <span id={`${id}-who`} className="flex min-w-0 items-baseline gap-1.5">
        <span className="truncate font-medium text-ink-2">{authorWords(item)}</span>
        {place && <span className="truncate text-muted">{place}</span>}
        {unread && <span className="sr-only">, unread</span>}
      </span>
      <time className="ml-auto shrink-0 pl-2 text-label text-muted" dateTime={new Date(item.at_ms).toISOString()} title={exactWhen(item.at_ms)} aria-label={exactWhen(item.at_ms)}>{when}</time>
    </p>
  );
}

/** A single item's quote, facts and its thread's other messages. */
function Facts({ id, item, thing }: { id: string; item: FeedItemInfo; thing: FeedThing }): JSX.Element {
  const restart = restartOf(thing);
  const facts = factsLine(item, restart === item ? null : restart);
  const others = (item.facts.thread_count ?? 1) - 1;
  return (
    <div id={`${id}-facts`} className="flex flex-col gap-1">
      {item.facts.quote && <blockquote className="mt-1 line-clamp-2 rounded-md bg-fill px-3 py-1.5 text-meta text-ink-2" data-testid="feed-quote">“{item.facts.quote}”</blockquote>}
      {facts && <p className="text-label text-muted" data-testid="feed-facts">{facts}</p>}
      {item.kind === "message" && others > 0 && <p className="text-label text-muted">{others} more {others === 1 ? "message" : "messages"} in this thread</p>}
    </div>
  );
}

/** A rollup's things: three, then Show all in place. */
function Rolled({ card, now, actions }: { card: FeedCard; now: number; actions: CardActions }): JSX.Element {
  const [all, setAll] = useState(false);
  const shown = all ? card.things : card.things.slice(0, SAMPLES);
  return (
    <>
      <ul className="mt-1 flex flex-col gap-0.5" data-testid="feed-rollup">
        {shown.map((one) => <Thing key={one.key} thing={one} now={now} actions={actions} />)}
      </ul>
      {card.things.length > SAMPLES && (
        <button type="button" aria-expanded={all} onClick={() => setAll((was) => !was)} className="self-start rounded text-label text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-rollup-all">
          {all ? "Show fewer" : `Show all ${card.things.length}`}
        </button>
      )}
    </>
  );
}

/** A single item's ghost actions: Open, Mark reviewed, ↗. */
function Actions({ item, actions }: { item: FeedItemInfo; actions: CardActions }): JSX.Element {
  const row = actions.railOf(item);
  const session = item.kind === "finished" ? item.target.session_id : null;
  return (
    // Over the card's lower right, shown on hover and focus: out of the
    // flow, so the list never grows or jumps as the pointer moves down it.
    <div className="absolute bottom-1.5 right-2 flex items-center gap-1 rounded-md bg-surface-2 p-0.5 opacity-0 shadow-card transition-opacity group-hover/card:opacity-100 group-focus-within/card:opacity-100 motion-reduce:transition-none" data-testid="feed-actions">
      <button type="button" className={GHOST} data-home-row onClick={() => void actions.open(item)}>Open<span className="sr-only"> {item.subject}</span></button>
      {session && <button type="button" className={GHOST} data-home-action onClick={() => actions.review(session)}>Mark reviewed<span className="sr-only"> {item.subject}</span></button>}
      {row && <GoTo row={row} />}
    </div>
  );
}

/** When a card happened: a rollup from its oldest to its newest. */
function whenOf(card: FeedCard, now: number): string {
  const newest = card.head.at_ms;
  const oldest = card.things[card.things.length - 1]!.head.at_ms;
  return card.rolled && oldest !== newest ? `${feedWhen(oldest, now)}–${feedWhen(newest, now)}` : feedWhen(newest, now);
}

export function HomeFeedCard({ card, pos, now, unread, fresh, actions }: { card: FeedCard; pos: number; now: number; unread: boolean; fresh: boolean; actions: CardActions }): JSX.Element {
  const item = card.head;
  const id = `feed-${item.id.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
  const parts = card.rolled ? rollupSentence(item.kind, card.things.length, item) : sentence(item);
  return (
    <article
      aria-labelledby={`${id}-who ${id}-what`}
      aria-describedby={card.rolled ? undefined : `${id}-facts`}
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
      className={"group/card tone-host relative flex min-w-0 gap-3 rounded-lg px-3 py-2.5 outline-none hover:bg-surface/70 focus-within:bg-surface/70 focus-visible:ring-1 focus-visible:ring-accent/60 " + (fresh ? "feed-fresh" : "")}
    >
      {unread && <span aria-hidden="true" className="absolute left-0 top-4 size-1.5 rounded-full bg-ink-2" data-testid="feed-unread" />}
      <span className="mt-0.5 shrink-0">
        <Avatar item={item} />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <Who id={id} item={item} when={whenOf(card, now)} unread={unread} />
        <p id={`${id}-what`} className="text-[0.84375rem] leading-snug text-ink" data-testid="feed-sentence">
          <Said parts={parts} onOpen={card.rolled ? null : () => void actions.open(item)} />
        </p>
        {card.rolled ? <Rolled card={card} now={now} actions={actions} /> : (
          <>
            <Facts id={id} item={item} thing={card.things[0]!} />
            <Actions item={item} actions={actions} />
          </>
        )}
      </div>
    </article>
  );
}

/**
 * D-136 point 2 — the one conversation, readable at length: it opens at
 * the newest, "Earlier, since 2 Nov" loads the page above (20 at a time,
 * never all), a divider marks each day and each New topic, and find
 * marks its matches without filtering anything. A finished look-up is
 * announced once, politely.
 */
import { Fragment, useEffect, useLayoutEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { IconChevronDown } from "../icons-controls";
import { BotTurn } from "./BotTurn";
import { BotWatchCard } from "./BotWatchCard";
import { WORDS, dayDivider, dayKey, lead, leadText } from "../../lib/bot-words";
import { loadEarlier, setAnchor, useBot } from "../../lib/bot-store";
import { botCount } from "../../lib/ipc/bot";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";
import type { BotTopicInfo } from "../../types/generated/BotTopicInfo";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

function DayDivider({ ms, now }: { ms: number; now: number }): JSX.Element {
  return (
    <div className="flex items-center gap-2.5 text-label font-medium tracking-[.07em] text-section-label uppercase before:h-px before:flex-1 before:bg-divider after:h-px after:flex-1 after:bg-divider" data-testid="bot-day">
      {dayDivider(ms, now)}
    </div>
  );
}

export function TopicDivider({ topic, withNote }: { topic: BotTopicInfo; withNote: boolean }): JSX.Element {
  return (
    <div className="flex flex-col items-center gap-1" data-testid="bot-topic">
      <span className="flex w-full items-center gap-2.5 text-label font-semibold tracking-[.07em] text-accent-hi uppercase before:h-px before:flex-1 before:bg-[color-mix(in_srgb,var(--color-accent)_40%,var(--color-divider))] after:h-px after:flex-1 after:bg-[color-mix(in_srgb,var(--color-accent)_40%,var(--color-divider))]">
        {WORDS.topicDivider(topic.started_ms)}
      </span>
      {withNote && <span className="text-label text-muted">{WORDS.topicNote}</span>}
    </div>
  );
}

/** What a finished look-up says, for the polite announcement. */
function announcement(turn: BotTurnInfo | undefined): string {
  if (!turn) return "";
  if (turn.outcome === "dont_know") return "Bot: I don’t know.";
  if (turn.outcome === "rows" || turn.outcome === "answered") return WORDS.answered(leadText(lead(turn.rows, turn.lang, turn.scope)));
  return "Bot answered.";
}

export function BotConversation({ now, empty, wide = false }: { now: number; empty: ReactNode; wide?: boolean }): JSX.Element {
  const bot = useBot();
  const scroller = useRef<HTMLDivElement | null>(null);
  const [watching, setWatching] = useState<{ turn: string; row: BotRowInfo } | null>(null);
  const last = bot.turns.at(-1);
  const lastId = last?.id;
  const [since, setSince] = useState<number | null>(null);
  useEffect(() => {
    if (bot.more && since === null) void botCount().then((c) => setSince(c.since_ms)).catch(() => undefined);
  }, [bot.more, since]);
  // Opens at the newest; a new turn scrolls to it.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const anchor = bot.anchor ? el.querySelector<HTMLElement>(`[data-turn="${bot.anchor}"]`) : null;
    if (anchor && !lastId) anchor.scrollIntoView?.({ block: "start" });
    else el.scrollTop = el.scrollHeight;
  }, [lastId, bot.anchor]);
  // Find: the current match brought into view.
  const current = bot.find?.matches[bot.find.at] ?? null;
  const turns = bot.turns;
  useEffect(() => {
    if (!current || !turns.some((t) => t.id === current)) return;
    scroller.current?.querySelector<HTMLElement>(`[data-turn="${current}"]`)?.scrollIntoView?.({ block: "center" });
  }, [current, turns]);
  const keepAnchor = () => {
    const el = scroller.current;
    if (!el) return;
    const top = el.getBoundingClientRect().top;
    const first = [...el.querySelectorAll<HTMLElement>("[data-turn]")].find((t) => t.getBoundingClientRect().bottom > top);
    setAnchor(first?.dataset.turn ?? null);
  };
  const matched = new Set(bot.find?.matches ?? []);
  const items: JSX.Element[] = [];
  let lastDay = "";
  const topics = bot.topics.filter((t) => !t.first);
  const oldestShown = bot.turns[0]?.at_ms ?? Number.POSITIVE_INFINITY;
  bot.turns.forEach((turn, i) => {
    const day = dayKey(turn.at_ms);
    const prev = bot.turns[i - 1];
    const startedHere = topics.filter((t) => t.started_ms > (prev?.at_ms ?? (bot.more ? oldestShown : Number.NEGATIVE_INFINITY)) && t.started_ms <= turn.at_ms);
    if (day !== lastDay && (i > 0 || bot.more || dayDivider(turn.at_ms, now) !== "Today")) items.push(<DayDivider key={`d-${turn.id}`} ms={turn.at_ms} now={now} />);
    lastDay = day;
    for (const topic of startedHere) items.push(<TopicDivider key={topic.id} topic={topic} withNote={false} />);
    items.push(
      <Fragment key={turn.id}>
        <BotTurn turn={turn} now={now} latest={turn === last} find={matched.has(turn.id) ? bot.find?.words : undefined} current={turn.id === current} onWatch={(row) => setWatching({ turn: turn.id, row })} />
        {watching?.turn === turn.id && <BotWatchCard key={watching.row.id} row={watching.row} turnId={turn.id} onClose={() => setWatching(null)} />}
      </Fragment>,
    );
  });
  // A New topic with nothing asked yet: its divider at the end, with its note.
  for (const topic of topics.filter((t) => t.started_ms > (last?.at_ms ?? Number.NEGATIVE_INFINITY))) items.push(<TopicDivider key={topic.id} topic={topic} withNote />);
  return (
    <div ref={scroller} onScroll={keepAnchor} className={"relative flex min-h-0 flex-1 flex-col overflow-y-auto " + (wide ? "gap-5 px-7 pt-5 pb-2.5" : "gap-[18px] px-4 pt-[18px] pb-2.5")} data-testid="bot-conversation">
      {bot.more && (
        // The pill floats over the oldest turn shown, the turns fading under it (the mock's top fade).
        <div className={"pointer-events-none sticky top-0 z-[4] -mb-12 flex shrink-0 justify-center bg-linear-to-b from-surface from-40% to-transparent pt-0 pb-8 " + (wide ? "-mx-7 -mt-5" : "-mx-4 -mt-[18px]")}>
          <button type="button" className="pointer-events-auto mt-2.5 inline-flex h-7 items-center gap-1.5 rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink shadow-[0_18px_48px_-12px_rgba(0,0,0,.45)] inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void loadEarlier()} data-testid="bot-earlier">
            <IconChevronDown width={13} height={13} aria-hidden="true" className="rotate-180" />
            {bot.loading ? "Loading…" : WORDS.earlier(since)}
          </button>
        </div>
      )}
      {bot.turns.length === 0 && !bot.loading ? empty : items}
      {bot.asking && (
        <output className="text-meta text-muted">
          {WORDS.asking}
        </output>
      )}
      {bot.error && (
        <p className="text-meta text-danger" role="alert">
          {bot.error}
        </p>
      )}
      <span className="sr-only" aria-live="polite" data-testid="bot-announce">
        {announcement(last)}
      </span>
    </div>
  );
}

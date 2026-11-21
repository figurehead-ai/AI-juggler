/**
 * D-118's mock (Home round 3) — one need, one card, in Needs you now: its
 * mark, then who and where ("Claude app · Work Max  payments-api", the
 * wait at the right), the sentence (the thing's name, what it needs in
 * its level's colour, what it asks), then its actions. The same anatomy
 * as the feed's cards (HomeFeedCard), on the surface; what broke (L3)
 * tinted. Session needs (HomeNeeds) and agents' messages (MessageNeeds)
 * both draw it.
 */
import type { JSX, ReactNode } from "react";

/** A card's box: the surface, or the danger tint for what broke (L3). */
export function needCardClass(level: number): string {
  return "flex min-w-0 gap-3 rounded-xl px-3.5 py-3 " + (level >= 3 ? "bg-danger-soft" : "bg-surface");
}

/** Who acted and where, then the time at the right (muted). */
export function NeedByLine({ author, place, time, timeTestId }: { author: string; place?: string | null; time?: string | null; timeTestId?: string }): JSX.Element {
  return (
    <span className="flex min-w-0 items-baseline gap-1.5 whitespace-nowrap text-meta">
      <span className="min-w-0 shrink truncate font-medium text-ink-2" title={author}>{author}</span>
      {place && <span className="min-w-0 flex-1 truncate text-muted" title={place}>{place}</span>}
      {time && <span className="ml-auto shrink-0 pl-2 text-label text-muted" data-testid={timeTestId}>{time}</span>}
    </span>
  );
}

/** An agent's letters for its disc: its first letter and its number when
 * it has one ("builder-1" → "b1", told from "builder-2"), else its first
 * two ("reviewer" → "re"). */
function agentLetters(name: string): string {
  const plain = name.replace(/[^\p{L}\p{N}]/gu, "").toLowerCase();
  const number = /(\d+)$/u.exec(plain)?.[1];
  if (number && plain.length > number.length) return (plain[0]! + number).slice(0, 3);
  return plain.slice(0, 2) || "?";
}

/** An agent's disc (the feed's): its letters on a neutral round mark. */
export function AgentDisc({ name, size = 22 }: { name: string; size?: number }): JSX.Element {
  return (
    <span aria-hidden="true" style={{ width: size, height: size }} className="flex shrink-0 items-center justify-center rounded-full bg-surface-3 text-[10px] font-semibold text-ink-2">
      {agentLetters(name)}
    </span>
  );
}

/** The card's actions, flush with the words above (ghost buttons start
 * at the text; the one filled primary stands out a little, the mock's). */
export function NeedActions({ children }: { children: ReactNode }): JSX.Element {
  return <div className="-ml-2 mt-1.5 flex flex-wrap items-center gap-1">{children}</div>;
}

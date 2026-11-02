/**
 * One turn of a conversation on its page (AJ-E2108; 0.3.5 keyed by id
 * and memoized, ADR-0024): your words on the right, the agent's on the
 * left with its mark. Its parts are drawn the way the tool's own app
 * draws them (`layout`, which the adapter says): every text with each
 * run of work as one titled line (the Claude app), or the turn's work
 * folded into one row before its reply (ChatGPT's Codex). One element
 * tree whether it is working or done: nothing the reader opened closes
 * when it finishes.
 */
import { memo, type JSX } from "react";
import { ToolMark } from "../../components/ToolMark";
import type { TurnInfo } from "../../lib/ipc/conversations";
import { agoWords } from "../../lib/time-words";
import { Markdown } from "./Markdown";
import { foldOf, Work, workWords } from "./TurnWork";
import { Stretches } from "./TurnStretches";

function When({ ms }: { ms: number | null }): JSX.Element | null {
  if (ms === null) return null;
  const date = new Date(ms);
  return (
    <time dateTime={date.toISOString()} title={date.toLocaleString()} className="text-label tabular-nums text-muted">
      {agoWords(ms, Date.now())}
    </time>
  );
}

type TurnProps = {
  turn: TurnInfo;
  tool: string;
  speaker: string;
  /** `stretches` or `folded`. */
  layout: string;
  asks: boolean;
  asker?: string;
  /** When the turn before it was written: its work's start. */
  startedMs: number | null;
  /** The session's latest turn while it works: shown as it goes. */
  working: boolean;
};

function TurnView({ turn, tool, speaker, layout, asks, asker, startedMs, working }: TurnProps): JSX.Element {
  if (turn.speaker === "you") {
    const words = turn.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n\n");
    return (
      <li className="flex flex-col items-end gap-1.5" data-testid="last-turn" data-speaker="you" data-turn-id={turn.id}>
        {asker ? <span className="text-label font-medium text-muted">{asker}</span> : <span className="sr-only">You</span>}
        <p className="max-w-[85%] whitespace-pre-wrap break-words rounded-2xl rounded-br-md border border-accent/20 bg-accent-soft px-4 py-2.5 text-body leading-[1.5] text-ink">{words}</p>
        <When ms={turn.at_ms} />
      </li>
    );
  }
  return (
    <li className="flex flex-col gap-2" data-testid="last-turn" data-speaker="assistant" data-asks={asks ? "true" : undefined} data-turn-id={turn.id}>
      <div className="flex items-center gap-2">
        <span aria-hidden="true" className="inline-flex size-7 items-center justify-center rounded-full border border-hairline bg-surface-2 text-ink">
          <ToolMark tool={tool} size={15} />
        </span>
        <span className="text-body font-semibold text-ink">{speaker}</span>
        {asks && (
          <span className="rounded-full bg-warning-soft px-2 py-0.5 text-label font-semibold text-warning" data-testid="last-turn-asks">
            asks you
          </span>
        )}
        <When ms={turn.at_ms} />
      </div>
      <div className="flex min-w-0 flex-col gap-3 pl-9">{layout === "folded" ? <Folded turn={turn} startedMs={startedMs} working={working} /> : <Stretches turn={turn} working={working} />}</div>
    </li>
  );
}

function Folded({ turn, startedMs, working }: { turn: TurnInfo; startedMs: number | null; working: boolean }): JSX.Element {
  const { work, reply } = foldOf(turn);
  const took = startedMs !== null && turn.at_ms !== null ? turn.at_ms - startedMs : null;
  return (
    <>
      {work.length > 0 && <Work work={work} words={workWords(work, took)} live={working} />}
      {reply && <Markdown text={reply.text} />}
    </>
  );
}

/** A turn re-renders only when it (by object), or how it's shown, changed. */
export const Turn = memo(TurnView);

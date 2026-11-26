/**
 * AJ-E3825 R8/R9 — the topic's running totals above the composer, calm
 * and muted: tokens this topic, then two indicators, each opening a short
 * breakdown on hover or click:
 * - **context** "context 3.1k / 4k": what the latest answer's request
 *   filled of the model's window, by part (instructions, Bot's memory,
 *   records and look-ups, conversation history, your question, what Bot
 *   linked from your work) and the room left for the answer; below them,
 *   what Bot linked and why ("agent11 · named in your question"), from
 *   its world model (AJ-E3832, ADR-0038 §5);
 * - **memory**: which memory went with it, by kind, with their tokens,
 *   and where to manage it.
 * Numbers are the engine's own counts where it gave one; an estimate is
 * labelled "≈ … est.".
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { botTopicUsage, type BotTopicUsageInfo } from "../../lib/ipc/bot";
import { useBot } from "../../lib/bot-store";
import { useBotNav } from "./bot-nav";
import { CONTEXT_PART_WORDS, MEMORY_KIND_WORDS, countWords, windowWords } from "../../lib/bot-engine-words";

/** A breakdown's width. */
const BREAKDOWN_WIDTH = 288;

function Indicator({ label, testId, children }: { label: string; testId: string; children: ReactNode }): JSX.Element {
  const [open, setOpen] = useState(false);
  // AJ-E3832 R6: a breakdown that would run past the window's right edge
  // (the docked panel) opens leftwards from its indicator instead.
  const [leftwards, setLeftwards] = useState(false);
  const anchor = useRef<HTMLSpanElement>(null);
  const show = (next: boolean) => {
    const box = anchor.current?.getBoundingClientRect();
    if (next && box) setLeftwards(box.left + BREAKDOWN_WIDTH > window.innerWidth - 8);
    setOpen(next);
  };
  return (
    <span ref={anchor} className="relative" onMouseEnter={() => show(true)} onMouseLeave={() => show(false)}>
      <button type="button" className="rounded-md px-1 text-label text-muted tabular-nums outline-none hover:text-ink-2 focus-visible:ring-2 focus-visible:ring-accent/60" aria-expanded={open} onClick={() => show(!open)} onKeyDown={(event) => event.key === "Escape" && show(false)} data-testid={testId}>
        {label}
      </button>
      {open && (
        <fieldset aria-label={label} className={`absolute bottom-[calc(100%+6px)] ${leftwards ? "right-0" : "left-0"} z-20 flex w-[288px] flex-col gap-1 rounded-xl bg-surface p-3 text-label text-ink-2 shadow-[0_18px_48px_-12px_rgba(0,0,0,.45),0_2px_6px_rgba(0,0,0,.2)] inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2`} data-testid={`${testId}-breakdown`}>
          {children}
        </fieldset>
      )}
    </span>
  );
}

/** The most linked things the context breakdown lists. */
const LINKED_SHOWN = 6;

function Row({ label, value }: { label: string; value: string }): JSX.Element {
  return (
    <span className="flex justify-between gap-3">
      <span className="text-muted">{label}</span>
      <span className="text-ink-2 tabular-nums">{value}</span>
    </span>
  );
}

export function BotUsageBar({ onMemories }: { onMemories: () => void }): JSX.Element | null {
  const bot = useBot();
  const nav = useBotNav();
  const [usage, setUsage] = useState<BotTopicUsageInfo | null>(null);
  const turns = bot.turns.length;
  const topics = bot.topics.length;
  useEffect(() => {
    // Nothing asked and no topic yet: nothing to count.
    if (turns === 0 && topics === 0) return () => undefined;
    let live = true;
    void botTopicUsage().then((next) => live && setUsage(next), () => undefined);
    return () => {
      live = false;
    };
  }, [turns, topics]);
  const latest = usage?.latest;
  if (!usage || usage.answers === 0 || !latest) return null;
  const total = { n: usage.tokens_in.n + usage.tokens_out.n, estimated: usage.tokens_in.estimated || usage.tokens_out.estimated };
  const context = latest.context;
  const memoryItems = latest.memory.reduce((sum, kind) => sum + kind.n, 0);
  return (
    <div className="flex shrink-0 items-center gap-1 px-4 pt-1 text-label text-faint" data-testid="bot-usage">
      <span className="tabular-nums" data-testid="bot-topic-tokens">This topic {countWords(total)} tokens</span>
      <span aria-hidden="true">·</span>
      <Indicator label={`context ${countWords(context.used)} / ${windowWords(context.window)}`} testId="bot-context">
        {context.parts.map((part) => (
          <Row key={part.part} label={CONTEXT_PART_WORDS[part.part]} value={countWords(part.tokens)} />
        ))}
        <Row label="Room for the answer" value={windowWords(context.answer_room)} />
        {latest.world.length > 0 && (
          <span className="mt-1 flex flex-col gap-0.5" data-testid="bot-context-linked">
            {latest.world.slice(0, LINKED_SHOWN).map((item) => (
              <span key={`${item.kind}:${item.name}`} className="flex min-w-0 gap-1" title={`${item.name} · ${item.why}`}>
                <span className="max-w-[55%] shrink-0 truncate text-ink-2">{item.name}</span>
                <span className="min-w-0 truncate text-faint">· {item.why}</span>
              </span>
            ))}
            {latest.world.length > LINKED_SHOWN && <span className="text-faint">and {latest.world.length - LINKED_SHOWN} more</span>}
          </span>
        )}
        <hr className="my-1 border-divider" />
        <Row label="Used of the window" value={`${countWords(context.used)} / ${windowWords(context.window)}`} />
      </Indicator>
      <span aria-hidden="true">·</span>
      <Indicator label={`memory ${memoryItems}`} testId="bot-memory">
        {latest.memory.map((kind) => (
          <Row key={kind.kind} label={MEMORY_KIND_WORDS[kind.kind] ?? kind.kind} value={`${kind.n} · ${countWords(kind.tokens)}`} />
        ))}
        <hr className="my-1 border-divider" />
        <button type="button" className="self-start rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onMemories}>
          Manage what Bot remembers
        </button>
        <button type="button" className="self-start rounded-md text-muted outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.openSettings("ai")}>
          Settings › AI
        </button>
      </Indicator>
    </div>
  );
}

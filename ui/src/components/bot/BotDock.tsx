/**
 * D-136 point 4 — the docked panel, the default: beside the page at 400
 * px, its left edge dragged between 340 and 560 (remembered; a double
 * click resets it), a sheet over the page under 1100 px of window. A
 * labelled complementary region; Esc closes it and focus returns to the
 * icon; ⌘F finds in the conversation. One conversation with the Bot page.
 */
import { useEffect, useRef, useState, type JSX, type PointerEvent as ReactPointerEvent } from "react";
import { BotConversation } from "./BotConversation";
import { BotComposer } from "./BotComposer";
import { BotEmpty, BotSetup } from "./BotEmpty";
import { BotFindBar, BotHead } from "./BotHead";
import { BotClearSheet, BotForgetHost } from "./BotSheets";
import { WORDS } from "../../lib/bot-words";
import { SHEET_UNDER, WIDTH_DEFAULT, closeBot, expandBot, openFind, setWidth, useBot } from "../../lib/bot-store";
import { isShown, useAiJob } from "../../lib/ai-availability";
import { BotUsageBar } from "./BotUsageBar";
import { refreshEngine, useBotEngine } from "../../lib/bot-engine";
import { startingLine } from "../../lib/bot-engine-words";
import type { AiEngineStatusInfo } from "../../lib/ipc/ai-engine";

/** A status with only the model's name (the start's line before the
 * backend answered). */
const EMPTY_STATUS: AiEngineStatusInfo = {
  state: "starting",
  failure: null,
  model_id: null,
  model_name: null,
  engine: null,
  version: null,
  context_tokens: null,
  started_ms: null,
  answered_ms: null,
  today: { answers: 0, tokens_in: { n: 0, estimated: false }, tokens_out: { n: 0, estimated: false } },
};

/** Under 1100 px of window the panel slides over the page. */
export function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => typeof window !== "undefined" && window.innerWidth < SHEET_UNDER);
  useEffect(() => {
    const on = () => setNarrow(window.innerWidth < SHEET_UNDER);
    window.addEventListener("resize", on);
    return () => window.removeEventListener("resize", on);
  }, []);
  return narrow;
}

function Grip({ width }: { width: number }): JSX.Element {
  const start = useRef<{ x: number; w: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    start.current = { x: event.clientX, w: width };
    setDragging(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!start.current) return;
    setWidth(start.current.w + (start.current.x - event.clientX));
  };
  const up = () => {
    start.current = null;
    setDragging(false);
  };
  return (
    <div
      // eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- a window splitter (WAI-ARIA) is a focusable separator with a grip inside; <hr> holds neither
      role="separator"
      aria-orientation="vertical"
      aria-label="Bot’s width"
      aria-valuemin={340}
      aria-valuemax={560}
      aria-valuenow={width}
      tabIndex={0}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onDoubleClick={() => setWidth(WIDTH_DEFAULT)}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") setWidth(width + 20);
        if (event.key === "ArrowRight") setWidth(width - 20);
      }}
      className="group absolute top-0 bottom-0 -left-[3px] z-[6] w-1.5 cursor-col-resize outline-none"
      data-testid="bot-grip"
    >
      <i className={"block h-full w-[3px] rounded-sm bg-accent transition-opacity " + (dragging ? "opacity-85" : "opacity-0 group-hover:opacity-85 group-focus-visible:opacity-85")} />
    </div>
  );
}

export function BotPanelBody({ onClear, onMemories }: { onClear: () => void; onMemories: () => void }): JSX.Element {
  const bot = useBot();
  const job = useAiJob("chat");
  const ready = isShown(job);
  const [now] = useState(() => Date.now());
  // AJ-E3825: the engine starts lazily when Bot opens with an AI chosen,
  // and is looked at again after each answer.
  const engine = useBotEngine(ready);
  const turns = bot.turns.length;
  useEffect(() => {
    if (ready && turns > 0) void refreshEngine();
  }, [ready, turns]);
  const starting = engine.starting || engine.status?.state === "starting";
  return (
    <>
      <BotHead job={job} onClear={onClear} onMemories={onMemories} setup={!ready} />
      {ready && bot.find && <BotFindBar />}
      {ready ? (
        <>
          {starting && (
            <output className="block shrink-0 px-4 pt-2 text-meta text-muted" data-testid="bot-starting">
              {startingLine(engine.status ?? (job?.model_name ? { ...EMPTY_STATUS, model_name: job.model_name } : null))}
            </output>
          )}
          <BotConversation now={now} empty={<BotEmpty />} />
          <BotUsageBar onMemories={onMemories} />
          <BotComposer footnote={bot.turns.length === 0 ? WORDS.footnote : undefined} />
        </>
      ) : (
        <BotSetup />
      )}
    </>
  );
}

export function BotDock(): JSX.Element | null {
  const bot = useBot();
  const narrow = useNarrow();
  const [clearing, setClearing] = useState(false);
  const panel = useRef<HTMLElement | null>(null);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (bot.mode !== "panel" || document.querySelector('[aria-modal="true"]')) return;
      if (event.key === "Escape" && !bot.find) {
        closeBot();
      } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f" && panel.current?.contains(document.activeElement)) {
        event.preventDefault();
        openFind();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bot.mode, bot.find]);
  if (bot.mode !== "panel") return null;
  return (
    <>
    {/* Under 1100 px the page can't keep its width beside the panel, so
        Bot slides over it as a sheet (the approved narrow frame), the page
        dimmed under it: nothing is cut half-visible, and a click on the
        page (or Esc) closes Bot. */}
    {narrow && <button type="button" tabIndex={-1} aria-label="Close Bot" className="absolute inset-0 z-[15] cursor-default bg-[rgba(14,18,25,.30)] [[data-theme=dark]_&]:bg-[rgba(4,6,10,.62)]" onClick={closeBot} data-testid="bot-scrim" />}
    <aside
      ref={panel}
      aria-label={WORDS.title}
      className={
        "flex min-h-0 shrink-0 flex-col border-l border-divider bg-surface " +
        (narrow ? "absolute top-0 right-0 bottom-0 z-20 shadow-[-24px_0_48px_-24px_rgba(14,18,25,.28),-1px_0_0_var(--color-divider)] [[data-theme=dark]_&]:shadow-[-24px_0_48px_-24px_rgba(0,0,0,.55),-1px_0_0_var(--color-divider)]" : "relative")
      }
      style={{ width: bot.width }}
      data-testid="bot-panel"
      data-sheet={narrow ? "1" : "0"}
    >
      {!narrow && <Grip width={bot.width} />}
      <BotPanelBody onClear={() => setClearing(true)} onMemories={() => { expandBot(); window.setTimeout(() => window.dispatchEvent(new Event("aij:bot-memories")), 0); }} />
      <BotForgetHost />
      {clearing && <BotClearSheet onClose={() => setClearing(false)} onMemories={() => { expandBot(); window.setTimeout(() => window.dispatchEvent(new Event("aij:bot-memories")), 0); }} />}
    </aside>
    </>
  );
}

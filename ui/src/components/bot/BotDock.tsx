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
import { BotEmpty, BotNoModel } from "./BotEmpty";
import { BotFindBar, BotHead } from "./BotHead";
import { BotClearSheet, BotForgetHost } from "./BotSheets";
import { WORDS } from "../../lib/bot-words";
import { SHEET_UNDER, WIDTH_DEFAULT, closeBot, expandBot, openFind, setWidth, useBot } from "../../lib/bot-store";
import { useAiJob } from "../../lib/ai-availability";

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
  const noModel = job?.state === "no_engine";
  const [now] = useState(() => Date.now());
  return (
    <>
      <BotHead job={job} onClear={onClear} onMemories={onMemories} />
      {bot.find && <BotFindBar />}
      <BotConversation now={now} empty={noModel ? <BotNoModel /> : <BotEmpty job={job} />} />
      <BotComposer footnote={bot.turns.length === 0 ? (noModel ? WORDS.footnoteNoModel : WORDS.footnote) : undefined} />
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
  );
}

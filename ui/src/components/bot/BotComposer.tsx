/**
 * D-136 — the question box: the scope chip ("About: api ×", one click
 * widens to everything), the question, + (Remember this…, Watch a
 * session…, New topic) and Ask. ↩ asks; ⇧↩ is a new line. The box stays
 * even with no model on this Mac (ADR-0034 §6.8 as amended): look-ups
 * need none.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { IconClose } from "../icons-controls";
import { IconPlus } from "../icons-actions";
import { IconBookmark, IconEye, IconLock, IconUp } from "./bot-icons";
import { WORDS } from "../../lib/bot-words";
import { ask, newTopic, setDraft, useBot, widenScope } from "../../lib/bot-store";

function PlusMenu({ onClose }: { onClose: () => void }): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    const onDown = (event: MouseEvent) => {
      if (!(event.target instanceof Node) || !ref.current?.contains(event.target)) onClose();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [onClose]);
  const item = "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-meta text-ink outline-none hover:bg-accent-soft focus-visible:bg-accent-soft";
  const pick = (run: () => void) => () => {
    onClose();
    run();
  };
  return (
    <div ref={ref} role="menu" aria-label={WORDS.plusLabel} className="absolute bottom-[calc(100%+6px)] left-0 z-20 flex w-[268px] flex-col gap-px rounded-xl bg-surface p-1.5 shadow-[0_18px_48px_-12px_rgba(0,0,0,.45),0_2px_6px_rgba(0,0,0,.2)] inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-plus-menu">
      <button type="button" role="menuitem" className={item} onClick={pick(() => setDraft("Remember: "))}>
        <IconBookmark width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="flex flex-col">
          <span>{WORDS.remember}</span>
          <span className="text-label text-muted">{WORDS.rememberHint}</span>
        </span>
      </button>
      <button type="button" role="menuitem" className={item} onClick={pick(() => void ask("What’s running?"))}>
        <IconEye width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="flex flex-col">
          <span>{WORDS.watchSession}</span>
          <span className="text-label text-muted">{WORDS.watchHint}</span>
        </span>
      </button>
      <hr className="mx-1.5 my-1 border-divider" />
      <button type="button" role="menuitem" className={item} onClick={pick(() => void newTopic())}>
        <IconPlus width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
        <span>{WORDS.newTopic}</span>
      </button>
    </div>
  );
}

export function BotComposer({ footnote }: { footnote?: string }): JSX.Element {
  const bot = useBot();
  const box = useRef<HTMLTextAreaElement | null>(null);
  const [plus, setPlus] = useState(false);
  useEffect(() => {
    if (bot.focusKey > 0) box.current?.focus();
  }, [bot.focusKey]);
  const typed = bot.draft.trim().length > 0;
  const placeholder = bot.scope ? WORDS.placeholderAbout(bot.scope.name) : WORDS.placeholder;
  return (
    <div className="relative shrink-0 px-3 pt-2 pb-3">
      <div className="relative flex flex-col gap-2 rounded-[14px] bg-fill py-[11px] pr-2.5 pl-3.5 inset-ring-1 inset-ring-hairline/75 focus-within:inset-ring-accent/70" data-testid="bot-composer">
        {bot.scope && (
          <div>
            <span className="inline-flex h-[26px] max-w-full items-center gap-1.5 rounded-full bg-accent-soft pr-1 pl-2.5 text-meta font-medium whitespace-nowrap text-accent-hi inset-ring-1 inset-ring-accent/30" data-testid="bot-scope-chip">
              <span className="truncate">{WORDS.aboutChip} {bot.scope.name}</span>
              <button type="button" className="grid size-[18px] shrink-0 place-items-center rounded-full outline-none hover:bg-accent/20 focus-visible:ring-2 focus-visible:ring-accent/60" title={WORDS.widen} aria-label={`${WORDS.widen} (remove “${bot.scope.name}”)`} onClick={widenScope} data-testid="bot-scope-x">
                <IconClose width={12} height={12} aria-hidden="true" />
              </button>
            </span>
          </div>
        )}
        <textarea
          ref={box}
          rows={1}
          value={bot.draft}
          placeholder={placeholder}
          aria-label={placeholder}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              void ask();
            }
          }}
          className="max-h-32 min-h-5 resize-none bg-transparent text-body leading-snug text-ink outline-none placeholder:text-faint field-sizing-content focus-visible:outline-none!"
          data-testid="bot-input"
        />
        <div className="flex items-center gap-1.5">
          <span className="relative">
            <button type="button" className={"grid size-7 place-items-center rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (plus ? "bg-surface-3 text-ink" : "text-muted hover:bg-surface-2 hover:text-ink")} title={WORDS.plusLabel} aria-label={WORDS.plusLabel} aria-haspopup="menu" aria-expanded={plus} onClick={() => setPlus((p) => !p)} data-testid="bot-plus">
              <IconPlus width={15} height={15} aria-hidden="true" />
            </button>
            {plus && <PlusMenu onClose={() => setPlus(false)} />}
          </span>
          <span className="grow" />
          <button type="button" disabled={!typed || bot.asking} className={"grid size-[30px] place-items-center rounded-[9px] outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (typed ? "bg-accent-fill text-white" : "bg-surface-3 text-faint")} aria-label={WORDS.askLabel} onClick={() => void ask()} data-testid="bot-send">
            <IconUp width={15} height={15} aria-hidden="true" />
          </button>
        </div>
      </div>
      {footnote && (
        <div className="mt-2 flex items-center justify-center gap-1.5 text-label text-faint">
          <IconLock width={12} height={12} aria-hidden="true" />
          {footnote}
        </div>
      )}
    </div>
  );
}

/**
 * D-136 — the panel's head: Bot (and its Beta pill while written answers
 * are on), New topic (its tip says what it does: a choice whose effect is
 * easy to misread), Search (⌘F), Expand to the Bot page, ⋯ and Close
 * (Esc). The find bar opens under it; the ⋯ menu holds Search, What Bot
 * remembers, the Bot page, the Beta switch (only where a model passes,
 * D-127), Clear the conversation… and Settings › AI.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { IconChevronDown, IconClose } from "../icons-controls";
import { IconMore } from "../icons-canvas";
import { IconPlus, IconTrash } from "../icons-actions";
import { IconBookmark, IconBot, IconExpand, IconEye, IconGrid, IconSearch, IconSparkle } from "./bot-icons";
import { BetaPill } from "./BotWritten";
import { useBotNav } from "./bot-nav";
import { WORDS } from "../../lib/bot-words";
import { closeBot, closeFind, expandBot, findWords, newTopic, openFind, stepMatch, useBot } from "../../lib/bot-store";
import { aiBetaSet, type AiJobInfo } from "../../lib/ipc/ai";
import { botCount } from "../../lib/ipc/bot";
import { useMemories } from "../../lib/bot-memory";
import { useMenuCloses } from "../../lib/use-menu-closes";

const ICON_BUTTON = "grid size-[30px] place-items-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60";

/** Whether the Beta switch is offered: a model passes Bot's tests here. */
function betaOffered(job: AiJobInfo | null): boolean {
  return job?.state === "ready" || job?.state === "waiting" || job?.state === "beta_off";
}

export function BotMenu({ job, onClose, onClear, onMemories, at = "right-2.5 top-[46px]" }: { job: AiJobInfo | null; onClose: () => void; onClear: () => void; onMemories: () => void; at?: string }): JSX.Element {
  const nav = useBotNav();
  const memories = useMemories();
  const ref = useRef<HTMLDivElement | null>(null);
  useMenuCloses(ref, onClose, "[role^=menuitem]");
  const item = "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-meta text-ink outline-none hover:bg-surface-3/70 focus-visible:bg-accent-soft";
  const pick = (run: () => void) => () => {
    onClose();
    run();
  };
  const betaOn = job?.beta_on ?? false;
  return (
    <div ref={ref} role="menu" aria-label={WORDS.moreLabel} className={`absolute ${at} z-30 flex w-[300px] flex-col gap-px rounded-xl bg-surface p-1.5 shadow-[0_18px_48px_-12px_rgba(0,0,0,.45),0_2px_6px_rgba(0,0,0,.2)] inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2`} data-testid="bot-menu">
      <button type="button" role="menuitem" className={item} onClick={pick(openFind)}>
        <IconSearch width={15} height={15} aria-hidden="true" className="text-muted" />
        <span className="grow">{WORDS.searchLabel}</span>
        <span className="text-label text-faint">⌘F</span>
      </button>
      <button type="button" role="menuitem" className={item} onClick={pick(onMemories)}>
        <IconBookmark width={15} height={15} aria-hidden="true" className="text-muted" />
        <span className="grow">{WORDS.whatBotRemembers}</span>
        {memories.info && memories.info.items.length > 0 && <span className="text-label text-faint tabular-nums">{memories.info.items.length}</span>}
      </button>
      <button type="button" role="menuitem" className={item} onClick={pick(expandBot)}>
        <IconExpand width={15} height={15} aria-hidden="true" className="text-muted" />
        <span className="grow">{WORDS.openPage}</span>
      </button>
      {betaOffered(job) && (
        <>
          <hr className="mx-1.5 my-1 border-divider" />
          <button type="button" role="menuitemcheckbox" aria-checked={betaOn} className={item} onClick={() => void aiBetaSet("chat", !betaOn)} data-testid="bot-beta-switch">
            <IconSparkle width={15} height={15} aria-hidden="true" className="text-muted" />
            <span className="flex grow flex-col">
              <span>{WORDS.betaSwitch}</span>
              <span className="text-label text-muted">{betaOn ? WORDS.betaOn : WORDS.betaOff}</span>
            </span>
            <span aria-hidden="true" className={"relative h-5 w-[34px] shrink-0 rounded-full " + (betaOn ? "bg-accent-fill" : "bg-surface-3 inset-ring-1 inset-ring-hairline")}>
              <span className={"absolute top-[3px] size-3.5 rounded-full " + (betaOn ? "left-[17px] bg-white" : "left-[3px] bg-faint")} />
            </span>
          </button>
        </>
      )}
      <hr className="mx-1.5 my-1 border-divider" />
      <button type="button" role="menuitem" className={item + " text-danger"} onClick={pick(onClear)} data-testid="bot-menu-clear">
        <IconTrash width={15} height={15} aria-hidden="true" />
        <span className="flex grow flex-col">
          <span>{WORDS.clear}</span>
          <span className="text-label text-muted">{WORDS.clearHint}</span>
        </span>
      </button>
      <button type="button" role="menuitem" className={item} onClick={pick(() => nav.openSettings("ai"))}>
        <IconGrid width={15} height={15} aria-hidden="true" className="text-muted" />
        <span className="grow">{WORDS.settingsAi}</span>
      </button>
    </div>
  );
}

/** Find in the conversation: "login", "1 of 5", ↑ ↓, Done. */
export function BotFindBar(): JSX.Element | null {
  const bot = useBot();
  const [words, setWords] = useState(bot.find?.words ?? "");
  const box = useRef<HTMLInputElement | null>(null);
  useEffect(() => box.current?.focus(), []);
  useEffect(() => {
    const t = window.setTimeout(() => void findWords(words), 180);
    return () => window.clearTimeout(t);
  }, [words]);
  const find = bot.find;
  const n = find?.matches.length ?? 0;
  return (
    <search className="flex shrink-0 items-center gap-1.5 border-b border-divider py-2 pr-2.5 pl-3" data-testid="bot-find">
      <label className="flex h-8 grow items-center gap-2 rounded-[10px] bg-fill px-2.5 inset-ring-[1.5px] inset-ring-accent ring-4 ring-accent-soft">
        <IconSearch width={14} height={14} aria-hidden="true" className="text-faint" />
        <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
          ref={box}
          value={words}
          onChange={(event) => setWords(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void stepMatch(event.shiftKey ? -1 : 1);
            if (event.key === "Escape") {
              event.stopPropagation();
              closeFind();
            }
          }}
          placeholder={WORDS.findPlaceholder}
          aria-label={WORDS.findPlaceholder}
          className="min-w-0 grow bg-transparent text-meta text-ink outline-none placeholder:text-faint focus-visible:outline-none!"
          data-testid="bot-find-input"
        />
      </label>
      <span className="px-1 text-meta whitespace-nowrap text-muted tabular-nums" aria-live="polite">
        {words.trim() ? WORDS.findCount(n === 0 ? 0 : (find?.at ?? 0) + 1, n) : ""}
      </span>
      <button type="button" className={ICON_BUTTON} aria-label={WORDS.findUp} disabled={n === 0} onClick={() => void stepMatch(-1)}>
        <IconChevronDown width={15} height={15} aria-hidden="true" className="rotate-180" />
      </button>
      <button type="button" className={ICON_BUTTON} aria-label={WORDS.findDown} disabled={n === 0} onClick={() => void stepMatch(1)}>
        <IconChevronDown width={15} height={15} aria-hidden="true" />
      </button>
      <button type="button" className="inline-flex h-7 items-center rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={closeFind} data-testid="bot-find-done">
        {WORDS.done}
      </button>
    </search>
  );
}

function TopicButton({ compact }: { compact: boolean }): JSX.Element {
  const [tip, setTip] = useState(false);
  return (
    <span className="relative" onMouseEnter={() => setTip(true)} onMouseLeave={() => setTip(false)}>
      <button type="button" className="mr-0.5 inline-flex h-7 items-center gap-1.5 rounded-lg px-2 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" aria-describedby="bot-topic-tip" onFocus={() => setTip(true)} onBlur={() => setTip(false)} onClick={() => void newTopic()} data-testid="bot-new-topic">
        <IconPlus width={13} height={13} aria-hidden="true" />
        <span className={compact ? "sr-only" : "whitespace-nowrap"}>{WORDS.newTopic}</span>
      </button>
      <span id="bot-topic-tip" role="tooltip" className={"absolute top-[calc(100%+6px)] right-0 z-30 w-[250px] flex-col gap-0.5 rounded-[9px] bg-ink px-2.5 py-2 text-meta text-canvas shadow-[0_18px_48px_-12px_rgba(0,0,0,.45)] [[data-theme=dark]_&]:bg-surface-3 [[data-theme=dark]_&]:text-ink " + (tip ? "flex" : "hidden")} data-testid="bot-topic-tip">
        <b className="font-semibold">{WORDS.newTopic}</b>
        <span className="opacity-80">{WORDS.newTopicTip}</span>
      </span>
    </span>
  );
}

/** The watch count (the maintainer's call on the Watches view: here, not
 * in the top bar). */
function WatchCount(): JSX.Element | null {
  const [n, setN] = useState<number | null>(null);
  useEffect(() => {
    void botCount().then((c) => setN(c.watches)).catch(() => setN(null));
  }, []);
  if (!n) return null;
  const words = `${n} ${n === 1 ? "watch" : "watches"}`;
  return (
    <span className="mr-1 inline-flex h-7 items-center gap-1 rounded-full px-2 text-meta font-medium text-ink-2 inset-ring-1 inset-ring-hairline" title={`Watches · ${words}`} aria-label={words} data-testid="bot-watch-count">
      <IconEye width={15} height={15} aria-hidden="true" className="text-faint" />
      <span className="tabular-nums">{n}</span>
    </span>
  );
}

export function BotHead({ job, onClear, onMemories }: { job: AiJobInfo | null; onClear: () => void; onMemories: () => void }): JSX.Element {
  const bot = useBot();
  const [menu, setMenu] = useState(false);
  return (
    <div className="relative flex h-12 shrink-0 items-center gap-0.5 border-b border-divider pr-2 pl-4 whitespace-nowrap">
      <h2 className="flex items-center gap-2 text-section font-semibold text-ink">
        <IconBot width={18} height={18} aria-hidden="true" className="text-accent-hi" />
        {WORDS.title}
      </h2>
      {job?.state === "ready" && job.beta && (
        <span className="ml-2" title="Written answers are on (Beta)">
          <BetaPill>Beta</BetaPill>
        </span>
      )}
      <span className="grow" />
      <WatchCount />
      <TopicButton compact={job?.state === "ready" && job.beta} />
      <button type="button" className={ICON_BUTTON + (bot.find ? " bg-surface-3 text-ink" : "")} title={`${WORDS.searchLabel} (⌘F)`} aria-label={WORDS.searchLabel} aria-pressed={bot.find !== null} onClick={() => (bot.find ? closeFind() : openFind())} data-testid="bot-search">
        <IconSearch width={16} height={16} aria-hidden="true" />
      </button>
      <button type="button" className={ICON_BUTTON} title={WORDS.expandLabel} aria-label={WORDS.expandLabel} onClick={expandBot} data-testid="bot-expand">
        <IconExpand width={15} height={15} aria-hidden="true" />
      </button>
      <button type="button" className={ICON_BUTTON + (menu ? " bg-surface-3 text-ink" : "")} title={WORDS.moreLabel} aria-label={WORDS.moreLabel} aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu((m) => !m)} data-testid="bot-more">
        <IconMore width={16} height={16} aria-hidden="true" />
      </button>
      <button type="button" className={ICON_BUTTON} title={`${WORDS.closeLabel} (Esc)`} aria-label={WORDS.closeLabel} onClick={closeBot} data-testid="bot-close">
        <IconClose width={16} height={16} aria-hidden="true" />
      </button>
      {menu && <BotMenu job={job} onClose={() => setMenu(false)} onClear={onClear} onMemories={onMemories} />}
    </div>
  );
}

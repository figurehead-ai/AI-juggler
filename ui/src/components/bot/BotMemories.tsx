/**
 * ADR-0037 §9 (as amended by D-136) — What Bot remembers, a tab of the Bot
 * page: only what the user told Bot, grouped (Names you use, What matters
 * to you, Your notes), each with where it came from, its age, how often
 * it was used, Edit and Forget (the sheet shows FORGET_AND_BACKUPS word
 * for word). Facts about sessions and usage are never kept here: Bot reads
 * them fresh each time. Watches are counted and kept while they run.
 */
import { useState, type JSX, type ReactNode } from "react";
import { LogoTile } from "../ToolLogo";
import { IconPool } from "../icons-actions";
import { IconBookmark, IconEdit, IconEye, IconFolder, IconLock, IconQuestion } from "./bot-icons";
import { memoryWords } from "./BotMemoryParts";
import { askForget, editMemory, useMemories } from "../../lib/bot-memory";
import { agoWords } from "../../lib/time-words";
import type { BotMemoryInfo } from "../../types/generated/BotMemoryInfo";

const KIND_WORDS: Record<string, string> = { session: "Session", project: "Project", pool: "Pool", app: "App" };

function Row({ memory, now }: { memory: BotMemoryInfo; now: number }): JSX.Element {
  const icon = memory.about_kind === "pool" ? IconPool : memory.slot === "focus" ? IconFolder : IconBookmark;
  const meta = [memory.about_kind ? KIND_WORDS[memory.about_kind] : null, `you said, ${agoWords(memory.at_ms, now)}`, memory.used > 0 ? (memory.used === 1 ? "used once" : `used ${memory.used} times`) : null].filter((m): m is string => typeof m === "string" && m.length > 0);
  return (
    <li className="grid grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-x-3.5 px-3.5 py-3 [&+&]:border-t [&+&]:border-divider" data-testid="bot-memory-row">
      <LogoTile icon={icon} size={32} />
      <div className="min-w-0">
        <div className="text-body leading-snug text-ink-2">{memory.slot === "note" ? memory.text : <span>{memoryWords(memory, true)}</span>}</div>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-meta text-muted">
          {meta.map((m, i) => (
            <span key={m} className="flex items-center gap-2.5">
              {i > 0 && <span aria-hidden="true" className="size-[3px] rounded-full bg-faint" />}
              {m}
            </span>
          ))}
        </div>
      </div>
      <div className="flex gap-0.5">
        <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => editMemory(memory)} aria-label={`Edit: ${memory.text}`}>
          <IconEdit width={13} height={13} aria-hidden="true" />
          Edit
        </button>
        <button type="button" className="inline-flex h-7 items-center rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => askForget(memory)} aria-label={`Forget: ${memory.text}`} data-testid="bot-memory-forget">
          Forget
        </button>
      </div>
    </li>
  );
}

function Group({ title, items, now }: { title: string; items: BotMemoryInfo[]; now: number }): JSX.Element | null {
  if (items.length === 0) return null;
  return (
    <section className="flex flex-col gap-2.5" aria-label={title}>
      <div className="flex items-baseline gap-2.5">
        <h3 className="text-section font-semibold text-ink">{title}</h3>
        <span className="text-meta text-muted">{items.length}</span>
      </div>
      <ul className="rounded-[14px] bg-surface inset-ring-1 inset-ring-hairline/75">
        {items.map((m) => (
          <Row key={m.key} memory={m} now={now} />
        ))}
      </ul>
    </section>
  );
}

function Tile({ icon, title, help, aside }: { icon: typeof IconEye; title: string; help: string; aside?: ReactNode }): JSX.Element {
  return (
    <div className="flex grow basis-0 items-center gap-3 rounded-[14px] bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75">
      <LogoTile icon={icon} size={32} />
      <div className="flex grow flex-col">
        <span className="text-body font-medium text-ink">{title}</span>
        <span className="text-meta text-muted">{help}</span>
      </div>
      {aside}
    </div>
  );
}

export function BotMemories(): JSX.Element {
  const { info, error } = useMemories();
  const [now] = useState(() => Date.now());
  const items = info?.items ?? [];
  const names = items.filter((m) => m.slot === "alias");
  const focus = items.filter((m) => m.slot === "focus");
  const notes = items.filter((m) => m.slot === "note");
  return (
    <div className="min-h-0 flex-1 overflow-y-auto pb-6" data-testid="bot-memories">
      <p className="mb-3 max-w-[90ch] text-meta text-muted">
        Only what you told Bot. Facts about your sessions and usage are never stored here: Bot reads them fresh each time. All of it stays on this Mac.
      </p>
      <div className="mb-[18px] flex items-center gap-3 rounded-xl bg-surface-2 px-3.5 py-2.5 text-meta text-ink-2">
        <IconLock width={15} height={15} aria-hidden="true" className="text-faint" />
        <span>On this Mac only · never synced, never sent</span>
      </div>
      {error && <p className="text-meta text-danger" role="alert">{error}</p>}
      {info && items.length === 0 && (
        <div className="rounded-[14px] bg-surface px-4 py-5 inset-ring-1 inset-ring-hairline/75" data-testid="bot-memories-empty">
          <p className="text-body font-medium text-ink">Nothing yet</p>
          <p className="text-meta text-muted">Say “Remember: call web the api agent” or “Remember: shop matters most” in the conversation, and it shows here.</p>
        </div>
      )}
      <div className="flex flex-col gap-5">
        <Group title="Names you use" items={names} now={now} />
        <div className="flex flex-wrap items-start gap-4">
          <div className="min-w-[280px] grow basis-0">
            <Group title="What matters to you" items={focus} now={now} />
          </div>
          <div className="min-w-[280px] grow basis-0">
            <Group title="Your notes" items={notes} now={now} />
          </div>
        </div>
        {info && (
          <div className="flex flex-wrap items-stretch gap-4">
            <Tile icon={IconEye} title={`Watches · ${info.watches}`} help="Kept while they run. They stay when you clear the conversation." />
            <Tile icon={IconQuestion} title={`What you asked · ${info.questions}`} help="Your questions, on this Mac. Clear them from the conversation’s ⋯ menu." />
          </div>
        )}
      </div>
    </div>
  );
}

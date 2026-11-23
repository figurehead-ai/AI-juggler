/**
 * D-136 point 1, D-144 — Bot's two empty states.
 * - **AI ready, nothing asked yet**: what Bot is (your AI assistant for
 *   everything AI Juggler sees), a short can and can't list (it doesn't
 *   talk to your agents), and three questions to try.
 * - **No AI ready**: only "Set up Bot", the engine-setup guide
 *   (AJ-E3823): no question box and no results. Bot is AI and
 *   conversation only; finding records is Search's (⌘K, AJ-E3824).
 */
import type { JSX } from "react";
import { IconCheck, IconClose } from "../icons-controls";
import { IconArrowRight, IconCheckCircle, IconHourglass, IconMessage, IconSparkle } from "./bot-icons";
import { EngineSetup } from "../ai/EngineSetup";
import { WORDS } from "../../lib/bot-words";
import { ask } from "../../lib/bot-store";

const EXAMPLE_ICONS = [IconMessage, IconCheckCircle, IconHourglass];

export function BotEmpty(): JSX.Element {
  return (
    <div className="flex flex-col gap-[18px]" data-testid="bot-empty">
      <div className="flex flex-col gap-2 px-0.5 pt-1.5">
        <span className="text-label font-semibold tracking-[.07em] text-accent-hi uppercase">{WORDS.eyebrow}</span>
        <h3 className="text-xl leading-tight font-semibold tracking-[-.01em] text-ink">{WORDS.emptyTitle}</h3>
        <p className="text-body text-muted">{WORDS.emptyLead}</p>
      </div>
      <ul className="flex flex-col gap-0.5 rounded-xl bg-surface-2 px-1.5 py-2" aria-label="What Bot does today">
        {WORDS.can.map((line) => (
          <li key={line.text} className="flex items-start gap-2.5 px-2 py-[5px] text-[13.5px] leading-snug text-ink-2">
            {line.mark === "yes" && <IconCheck width={15} height={15} aria-label="Does" className="mt-px shrink-0 text-success" />}
            {line.mark === "no" && <IconClose width={15} height={15} aria-label="Doesn’t" className="mt-px shrink-0 text-faint" />}
            {line.mark === "later" && <IconSparkle width={15} height={15} aria-hidden="true" className="mt-px shrink-0 text-accent-hi" />}
            <span className={line.mark === "later" ? "text-muted" : ""}>{line.text}</span>
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-2">
        <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{WORDS.tryOne}</span>
        <div className="flex flex-col gap-1.5">
          {WORDS.examples.map((example, i) => {
            const Icon = EXAMPLE_ICONS[i] ?? IconMessage;
            return (
              <button key={example} type="button" className="flex items-center gap-2.5 rounded-xl bg-surface-2 px-3 py-2.5 text-left text-body text-ink outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void ask(example)} data-testid="bot-example">
                <Icon width={16} height={16} aria-hidden="true" className="shrink-0 text-faint" />
                <span className="grow">{example}</span>
                <IconArrowRight width={14} height={14} aria-hidden="true" className="text-faint" />
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/** D-144: with no AI ready, Bot shows only "Set up Bot". */
export function BotSetup(): JSX.Element {
  return (
    <div className="scroll-edge flex min-h-0 flex-1 flex-col overflow-y-auto px-4 pt-[18px] pb-5" data-testid="bot-setup">
      <EngineSetup place="bot" />
    </div>
  );
}

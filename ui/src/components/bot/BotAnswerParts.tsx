/**
 * AJ-E3825 — the parts of an answer that aren't records:
 * - **General** (D-148): the model's own short answer, labelled "Bot's
 *   own answer · not from your records", never with record chips; for a
 *   question outside AI Juggler, one line saying what Bot is for.
 * - **Failed**: one plain line with its fix, and Try again; never a
 *   records-only answer in its place.
 * - **Details**: a small, calm line under each answer: the model, tokens
 *   in and out (the engine's own counts; an estimate says "≈ … est."),
 *   how long it took, and the records it rested on when grounded.
 */
import type { JSX } from "react";
import { ask } from "../../lib/bot-store";
import { useBotNav } from "./bot-nav";
import { GENERAL_LABEL, SCOPE_LINE, countWords, failureWords, modelName, secondsWords } from "../../lib/bot-engine-words";
import { useBotEngine } from "../../lib/bot-engine";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

export function BotGeneralAnswer({ turn }: { turn: BotTurnInfo }): JSX.Element | null {
  const general = turn.general;
  if (!general) return null;
  return (
    <section className="flex flex-col gap-1.5" aria-label={GENERAL_LABEL} data-testid="bot-general">
      <span className="text-label text-muted" data-testid="bot-general-label">{GENERAL_LABEL}</span>
      <p className="text-body leading-normal text-ink">{general.text}</p>
      {general.off_topic && <p className="text-meta text-muted" data-testid="bot-scope-line">{SCOPE_LINE}</p>}
    </section>
  );
}

export function BotFailedAnswer({ turn, model }: { turn: BotTurnInfo; model?: string }): JSX.Element {
  const nav = useBotNav();
  const engine = useBotEngine(false);
  const words = failureWords(turn.failure ?? "unknown", turn.details?.model_name ?? model ?? modelName(engine.status));
  return (
    <output className="flex flex-col gap-1.5" data-testid="bot-failed" data-failure={turn.failure ?? "unknown"}>
      <p className="text-body leading-normal text-ink-2">
        <b className="font-semibold text-ink">{words.line}</b> {words.fix}
      </p>
      <span className="flex gap-3 text-meta">
        <button type="button" className="rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void ask(turn.question)} data-testid="bot-try-again">
          Try again
        </button>
        <button type="button" className="rounded-md text-muted outline-none hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.openSettings("ai")}>
          Details
        </button>
      </span>
    </output>
  );
}

export function BotDetailsLine({ turn }: { turn: BotTurnInfo }): JSX.Element | null {
  const details = turn.details;
  if (!details) return null;
  const parts = [
    details.model_name,
    `${countWords(details.tokens_in)} in · ${countWords(details.tokens_out)} out`,
    secondsWords(details.ms),
  ];
  if (turn.outcome === "answered" && details.records > 0) parts.push(`${details.records} ${details.records === 1 ? "record" : "records"}`);
  return (
    <p className="text-label text-faint tabular-nums" data-testid="bot-details">
      {parts.join(" · ")}
    </p>
  );
}

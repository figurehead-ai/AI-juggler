/**
 * AJ-E3826 — Bot's AI, the Models page's first card once a model is in
 * use: which AI writes Bot's answers (each model that can, on this Mac,
 * as a choice with its own tile; Automatic first when there are two or
 * more), the current choice in the accent's soft fill (PAT-SURF), its
 * live state from the started engine ("Running · answered 2 min ago",
 * "Idle · starts when you ask Bot", "Not running: why. fix"), and
 * today's answers and tokens, an estimate labelled "≈ … est.".
 *
 * One source: the engine status AJ-E3825 built (`ai_engine_status`, read
 * here and in Bot's header from one store; it starts nothing). One
 * setting: a choice is the chat job's pick (`ai_set_pick`), the same the
 * router reads.
 */
import type { JSX } from "react";
import { Radio } from "../../../components/controls/Toggle";
import { IconWarn } from "../../../components/icons-menu";
import { countWords, failureWords, modelName } from "../../../lib/bot-engine-words";
import { useEngineStatus } from "../../../lib/bot-engine";
import type { AiEngineStatusInfo } from "../../../lib/ipc/ai-engine";
import type { AiJobInfo } from "../../../lib/ipc/ai";
import type { CatalogModel, ModelsCatalog, ModelsInfo } from "../../../lib/ipc/models";
import { useClock } from "../../../lib/clock";
import { agoWords, knownMs } from "../../../lib/time-words";
import { IconAuto, IconPulse } from "./model-icons";
import { CARD, ModelTile, ModelsSection } from "./ModelParts";
import { nameOf, pickable, verdictOf } from "./models-view";
import { BOT_AI } from "./models-words";

const AUTOMATIC = "automatic";

interface Choice {
  id: string;
  name: string;
  hint: string | null;
  model: CatalogModel | null;
}

/** The choices: Automatic first when there are two or more models. */
function choicesOf(catalog: ModelsCatalog | null, installed: ModelsInfo | null): Choice[] {
  const models = pickable(catalog, installed, "chat");
  const own = models.map((model) => ({ id: model.id, name: model.name, hint: verdictOf(model, "chat").note, model }));
  if (models.length < 2) return own;
  const automatic = nameOf(catalog?.recommendation?.model_id, catalog, installed);
  return [{ id: AUTOMATIC, name: BOT_AI.automatic, hint: BOT_AI.automaticHint(automatic), model: null }, ...own];
}

/** The live state in words, and whether it is a failure. */
export function stateLine(status: AiEngineStatusInfo | null, starting: boolean, now: number): { words: string; tone: "live" | "warn" | "quiet" } | null {
  if (!status) return null;
  const at = status.answered_ms;
  const answered = knownMs(at) ? BOT_AI.answered(agoWords(at, now)) : null;
  const join = (...parts: Array<string | null>) => parts.filter(Boolean).join(" · ");
  if (starting || status.state === "starting") return { words: BOT_AI.starting, tone: "quiet" };
  switch (status.state) {
    case "running":
      return { words: join(BOT_AI.running, answered), tone: "live" };
    case "answering":
      return { words: BOT_AI.answering, tone: "live" };
    case "idle":
      return { words: join(BOT_AI.idle, answered ?? BOT_AI.idleHow), tone: "quiet" };
    case "failed": {
      const failure = failureWords(status.failure ?? "unknown", modelName(status));
      return { words: `${BOT_AI.notRunning}: ${failure.line} ${failure.fix}`, tone: "warn" };
    }
    default:
      return { words: join(BOT_AI.off, BOT_AI.offHow), tone: "quiet" };
  }
}

/** Today's answers and tokens: "Today · 12 answers · 8.1k tokens in · ≈ 2.3k est. out". */
export function todayLine(status: AiEngineStatusInfo | null): string | null {
  if (!status) return null;
  const { answers, tokens_in, tokens_out } = status.today;
  if (answers === 0) return `${BOT_AI.today} · ${BOT_AI.noAnswers}`;
  return [BOT_AI.today, BOT_AI.answers(answers), BOT_AI.tokensIn(countWords(tokens_in)), BOT_AI.tokensOut(countWords(tokens_out))].join(" · ");
}

function StateLine({ status, starting, prefix }: { status: AiEngineStatusInfo | null; starting: boolean; prefix: string | null }): JSX.Element | null {
  const state = stateLine(status, starting, useClock());
  if (!state) return null;
  return (
    <span className="flex min-w-0 items-start gap-1.5 text-meta text-ink-2" aria-live="polite" data-testid="models-bot-state" data-tone={state.tone}>
      {state.tone === "live" && <span aria-hidden="true" className="mt-[7px] size-1.5 shrink-0 rounded-full bg-live" />}
      {state.tone === "warn" && <IconWarn width={13} height={13} aria-hidden="true" className="mt-[3px] shrink-0 text-warning" />}
      <span className="min-w-0">{prefix ? `${prefix} · ${state.words}` : state.words}</span>
    </span>
  );
}

function ChoiceTile({ choice, on }: { choice: Choice; on: boolean }): JSX.Element {
  if (choice.model) return <ModelTile model={choice.model} on={on} />;
  return (
    <span aria-hidden="true" className={`flex size-9 shrink-0 items-center justify-center rounded-[10px] ${on ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2"}`}>
      <IconAuto width={19} height={19} />
    </span>
  );
}

function ChoiceRow({
  choice,
  chosen,
  several,
  status,
  starting,
  onChoose,
}: {
  choice: Choice;
  chosen: boolean;
  several: boolean;
  status: AiEngineStatusInfo | null;
  starting: boolean;
  onChoose: () => void;
}): JSX.Element {
  const look = chosen ? "bg-accent-soft" : several ? "cursor-pointer hover:bg-surface-2" : "";
  const body = (
    <>
      <ChoiceTile choice={choice} on={chosen} />
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className={`text-body text-ink ${chosen ? "font-semibold" : "font-medium"}`}>{choice.name}</span>
        {chosen ? (
          <StateLine status={status} starting={starting} prefix={choice.model ? null : modelName(status)} />
        ) : (
          choice.hint && <span className="text-meta text-muted">{choice.hint}</span>
        )}
      </span>
    </>
  );
  const row = `flex min-w-0 items-center gap-3 rounded-[10px] px-3 py-2.5 transition-colors ${look}`;
  if (!several) {
    return (
      <div className={row} data-testid={`models-bot-choice-${choice.id}`} data-chosen={chosen || undefined}>
        {body}
      </div>
    );
  }
  return (
    <label className={`${row} has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-accent/60`} data-testid={`models-bot-choice-${choice.id}`} data-chosen={chosen || undefined}>
      <Radio name="models-bot-ai" className="sr-only" checked={chosen} onChange={onChoose} aria-label={choice.name} />
      {body}
    </label>
  );
}

export function BotAiSection({
  jobs,
  catalog,
  installed,
  onPick,
}: {
  jobs: readonly AiJobInfo[] | null;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  onPick: (job: AiJobInfo["job"], modelId: string | null) => void;
}): JSX.Element {
  const chat = jobs?.find((info) => info.job === "chat");
  const { status, starting } = useEngineStatus(`${chat?.state}:${chat?.model_id}:${chat?.pick}:${chat?.beta_on}`);
  const choices = choicesOf(catalog, installed);
  const several = choices.length > 1;
  const chosenId = several ? (chat?.pick ?? AUTOMATIC) : choices[0]?.id;
  const today = todayLine(status);
  return (
    // Not a search landing: it shows only once a model is in use (search
    // lands on the pane, whose first section it then is).
    <ModelsSection id="ai-bot" title={BOT_AI.title} help={BOT_AI.help} searchable={false}>
      <div className={`flex flex-col gap-0.5 p-1.5 ${CARD}`} data-testid="models-bot">
        <div role={several ? "radiogroup" : undefined} aria-label={several ? BOT_AI.label : undefined} className="flex flex-col gap-0.5">
          {choices.length === 0 ? (
            <div className="flex items-center gap-3 px-3 py-2.5">
              <span className="flex min-w-0 flex-col gap-0.5">
                <span className="text-body font-semibold text-ink">{modelName(status)}</span>
                <StateLine status={status} starting={starting} prefix={null} />
              </span>
            </div>
          ) : (
            choices.map((choice) => (
              <ChoiceRow
                key={choice.id}
                choice={choice}
                chosen={choice.id === chosenId}
                several={several}
                status={status}
                starting={starting}
                onChoose={() => onPick("chat", choice.id === AUTOMATIC ? null : choice.id)}
              />
            ))
          )}
        </div>
        {today && (
          <p className="flex items-center gap-2 px-3 pt-1.5 pb-1 text-meta text-muted" data-testid="models-bot-today">
            <IconPulse width={14} height={14} aria-hidden="true" className="shrink-0 text-faint" />
            <span className="tabular-nums">{today}</span>
          </p>
        )}
      </div>
    </ModelsSection>
  );
}

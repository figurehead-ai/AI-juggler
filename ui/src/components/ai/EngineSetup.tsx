/**
 * AJ-E3823 (D-144, rich mode) — the one engine-setup guide for Bot: what
 * this Mac can do, Apple's built-in AI first, the downloadable AI named
 * and explained. One component, used as "Set up Bot" (Bot's panel and
 * page, with no AI ready), in the wizard's Meet Bot step, and at the top
 * of Settings › AI › Models while Apple's AI is off, getting ready or
 * couldn't be checked.
 *
 * "Checking what this Mac can do…" first; then the recommendation line,
 * the two choices side by side (stacked in a narrow panel), each with its
 * state in plain words and a button that says what it does, and "What is
 * this?". A download starts only on its button, through the Models
 * screen's approval and disk rules; nothing else here raises a system
 * prompt (opening a System Settings pane asks nothing).
 */
import { useId, type JSX } from "react";
import { Alert } from "../Alert";
import { Button } from "../primitives";
import { useAiJob, useAiJobs } from "../../lib/ai-availability";
import { useModelsCatalog, useModelsInstalled } from "../../lib/ai-models";
import { useHelperSetUp } from "../../lib/helper-setup";
import type { AiJob } from "../../lib/ipc/ai";
import type { ModelsCatalog } from "../../lib/ipc/models";
import { ApprovalDialog } from "../../screens/settings/ai/ModelsApproval";
import { ModelFacts, ModelTile } from "../../screens/settings/ai/ModelParts";
import { Licence } from "../../screens/settings/ai/ModelsOthers";
import { useModelActions, type ModelActions } from "../../screens/settings/ai/models-actions";
import { memoryWords } from "../../screens/settings/ai/models-words";
import { setupView, type SetupView } from "./engine-setup-view";
import { APPLE, DOWNLOADED, NO_AI, SETUP, originOf } from "./engine-setup-words";
import { AppleAction, Choice, Consequence, DownloadAction, Recommendation, What } from "./EngineSetupParts";

/** The job the guide sets up: Bot's answers. */
const BOT: readonly AiJob[] = ["chat"];

export interface EngineSetupProps {
  /**
   * Where it sits:
   * - `bot`: Bot's panel or page with no AI ready ("Set up Bot", its own
   *   heading and lead);
   * - `wizard`: the wizard's Meet Bot step (the step has its heading;
   *   the lead and Skip show);
   * - `models`: the top of Settings › AI › Models (no heading or lead:
   *   the screen's section has them).
   */
  place: "bot" | "wizard" | "models";
  /** Wizard: "Skip for now" (shown only when given). */
  onSkip?: () => void;
  /** Called once Bot has its AI chosen: Use clicked, or a download
   * started (the wizard moves on; a download goes on by itself). */
  onChosen?: () => void;
}


/** Nothing read yet. */
function Checking(): JSX.Element {
  return (
    <output className="flex items-center gap-2.5 text-body text-ink-2" data-testid="setup-checking">
      <span aria-hidden="true" className="size-4 shrink-0 animate-spin rounded-full border-2 border-surface-3 border-t-accent motion-reduce:animate-none" />
      {SETUP.checking}
    </output>
  );
}

/** Neither AI can run here: one line on what it needs. */
function NoAi({ why, catalog }: { why: NonNullable<SetupView["noAi"]>; catalog: ModelsCatalog | null }): JSX.Element {
  const memory = catalog?.mac ? memoryWords(catalog.mac.memory_bytes, catalog.mac.memory_gb) : "";
  let line = NO_AI.other;
  if (why === "not_apple_silicon") line = NO_AI.not_apple_silicon;
  else if (why === "not_enough_memory") line = NO_AI.not_enough_memory(memory);
  return (
    <Alert level="info" title={NO_AI.title} testId="setup-no-ai">
      {line}
    </Alert>
  );
}

type Chosen = (run: () => void) => () => void;

function AppleCard({ id, view, act, chosen }: { id: string; view: SetupView; act: ModelActions; chosen: Chosen }): JSX.Element {
  const apple = view.apple ?? "unchecked";
  // AJ-E1547: its helper hasn't started yet (not set up), so it wasn't checked.
  const helperSetUp = useHelperSetUp();
  const notSetUp = apple === "unchecked" && helperSetUp === false;
  let tone: "ok" | "wait" | "warn" = "warn";
  if (apple === "ready") tone = "ok";
  else if (apple === "preparing" || apple === "off") tone = "wait";
  return (
    <Choice
      titleId={id}
      tile={<ModelTile model={view.appleModel} size="lg" on={view.recommended === "apple"} />}
      name={APPLE.name}
      sub={APPLE.sub}
      recommended={view.recommended === "apple"}
      state={notSetUp ? APPLE.notSetUp : APPLE.state[apple]}
      stateTone={tone}
      facts={<ModelFacts model={view.appleModel} inCard />}
      points={APPLE.points}
      need={{ met: apple === "ready" || apple === "preparing" }}
      testId="setup-apple"
    >
      <AppleAction view={view} act={act} notSetUp={notSetUp} onAdopt={chosen(() => act.use(view.appleModel, BOT))} />
    </Choice>
  );
}

function DownloadCard({ id, view, act, chosen }: { id: string; view: SetupView; act: ModelActions; chosen: Chosen }): JSX.Element | null {
  const model = view.download;
  if (!model) return null;
  const origin = originOf(model.licence_url);
  const here = view.downloadAction === "use" || view.downloadAction === "in-use";
  return (
    <Choice
      titleId={id}
      tile={<ModelTile model={model} size="lg" on={view.recommended === "download"} />}
      name={model.name}
      sub={
        <>
          {DOWNLOADED.sub(origin.maker, model.size_bytes ?? 0)}
          {model.licence_name && (
            <>
              {" · "}
              <Licence model={model} />
            </>
          )}
        </>
      }
      recommended={view.recommended === "download"}
      state={here ? DOWNLOADED.here : DOWNLOADED.notHere(view.free)}
      stateTone={here ? "ok" : "wait"}
      facts={<ModelFacts model={model} inCard />}
      points={DOWNLOADED.points}
      testId="setup-download-card"
    >
      <DownloadAction view={view} act={act} onAdopt={chosen(() => act.use(model, BOT))} onDownload={chosen(() => act.download(model, true, BOT))} />
    </Choice>
  );
}

/** The recommendation, Apple's state when it can't run here, the two
 * choices and "What is this?". */
function Choices({ view, act, chosen }: { view: SetupView; act: ModelActions; chosen: Chosen }): JSX.Element {
  const ids = useId();
  return (
    <>
      <Recommendation view={view} />
      {view.apple === "cant_run" && (
        <p className="flex items-start gap-2 px-1 text-meta text-ink-2" data-testid="setup-apple-state">
          <span aria-hidden="true" className="mt-[5px] size-2 shrink-0 rounded-full bg-faint" />
          <span>
            <span className="font-medium text-ink">{APPLE.name}:</span> {APPLE.state.cant_run}. {APPLE.cantRunLine}
          </span>
        </p>
      )}
      <section aria-labelledby={`${ids}-choose`} className="flex flex-col gap-2.5">
        <span id={`${ids}-choose`} className="text-label font-medium tracking-[.07em] text-section-label uppercase">
          {SETUP.choose}
        </span>
        <div className="@container">
          <div className="grid grid-cols-1 gap-3 @min-[680px]:grid-cols-2">
            {view.apple !== "cant_run" && <AppleCard id={`${ids}-apple`} view={view} act={act} chosen={chosen} />}
            <DownloadCard id={`${ids}-download`} view={view} act={act} chosen={chosen} />
          </div>
        </div>
      </section>
      <What view={view} />
    </>
  );
}

/** "Set up Bot"'s own heading (Bot's panel and page only). */
function Head(): JSX.Element {
  return (
    <div className="flex flex-col gap-2 px-0.5 pt-1.5">
      <span className="text-label font-semibold tracking-[.07em] text-accent-hi uppercase">{SETUP.eyebrow}</span>
      <h3 className="text-xl leading-tight font-semibold tracking-[-.01em] text-ink">{SETUP.title}</h3>
    </div>
  );
}

function Skip({ onSkip, step }: { onSkip: () => void; step: boolean }): JSX.Element {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="ghost" onClick={onSkip} data-testid="setup-skip">
        {step ? SETUP.skipStep : SETUP.skip}
      </Button>
      <Consequence>{SETUP.skipLine}</Consequence>
    </div>
  );
}

export function EngineSetup({ place, onSkip, onChosen }: EngineSetupProps): JSX.Element {
  const catalog = useModelsCatalog();
  const installed = useModelsInstalled();
  const jobs = useAiJobs();
  const job = useAiJob("chat");
  const act = useModelActions(jobs);
  const view = setupView(catalog, installed, job);
  const chosen: Chosen = (run) => () => {
    run();
    onChosen?.();
  };
  let body: JSX.Element;
  if (view.checking) body = <Checking />;
  else if (view.noAi) body = <NoAi why={view.noAi} catalog={catalog} />;
  else body = <Choices view={view} act={act} chosen={chosen} />;
  return (
    <div className="flex flex-col gap-4" data-testid="engine-setup" data-place={place} data-apple={view.apple ?? "checking"}>
      {place === "bot" && <Head />}
      {place !== "models" && <p className="text-body text-muted">{SETUP.lead}</p>}
      {act.note && (
        <Alert level="needs-you" testId="setup-note">
          {act.note}
        </Alert>
      )}
      {body}
      {/* In the setup wizard it skips the step, worded as the wizard's (0.5.1 F50). */}
      {onSkip && <Skip onSkip={onSkip} step={place === "wizard"} />}
      {act.asking && (
        <ApprovalDialog
          model={act.asking}
          have={installed?.installed.map((m) => m.name) ?? []}
          totalNow={installed?.total_bytes ?? 0}
          free={view.free}
          onDownload={act.approve}
          onNotNow={act.notNow}
          onClose={act.closeAsk}
        />
      )}
    </div>
  );
}

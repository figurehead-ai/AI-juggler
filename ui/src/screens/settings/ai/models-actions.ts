/**
 * AJ-E3812 — what the Models screen's clicks do, through the backend's
 * commands only (no byte is fetched before a Download click):
 * - Use it turns on the jobs the model passed as Beta (D-130's opt-in);
 * - Download starts it, or opens the approval card for a second model
 *   (D-121, asked every time), or says why not; the recommended model's
 *   Download turns its Beta jobs on too, as its line says;
 * - the approval's Download carries its one-time token; Not now is
 *   remembered; Cancel, Remove, a Beta switch and a job's pick.
 */
import { useState } from "react";
import { errorText } from "../../../lib/bridge";
import { aiBetaSet, aiJobPickSet, type AiJob, type AiJobInfo } from "../../../lib/ipc/ai";
import {
  aiModelApproveExtra,
  aiModelDeclineExtra,
  aiModelDownloadCancel,
  aiModelDownloadStart,
  aiModelRemove,
  aiEnginesCheck,
  aiOpenSystemPage,
  type CatalogModel,
  type StartInfo,
} from "../../../lib/ipc/models";
import { refreshModels } from "../../../lib/ai-models";
import { BOT_HELPER, osPromptRequest } from "../../../lib/ipc/os-prompts";
import { betaJobs } from "./models-view";
import { REFUSALS } from "./models-words";

export interface ModelActions {
  /** The model whose approval card is open. */
  asking: CatalogModel | null;
  /** A refusal or failure to say, in words. */
  note: string | null;
  /** Turns on the jobs `model` passed as Beta (`only`: just these). */
  use: (model: CatalogModel, only?: readonly AiJob[]) => void;
  /** Starts a download; `recommended` turns its Beta jobs on once it
   * started (`only`: just these). */
  download: (model: CatalogModel, recommended: boolean, only?: readonly AiJob[]) => void;
  approve: () => void;
  notNow: () => void;
  closeAsk: () => void;
  cancel: (id: string) => void;
  remove: (id: string) => void;
  beta: (job: AiJob, on: boolean) => void;
  pick: (job: AiJob, modelId: string | null) => void;
  /** Asks the engines again now, then reads the screen again. */
  checkAgain: () => void;
  /** A Check again is under way. */
  checking: boolean;
  openStorage: () => void;
  openAppleIntelligence: () => void;
}

/** Turns on each of `model`'s Beta jobs that is off (`only`: of these). */
async function turnOnBetas(model: CatalogModel, jobs: readonly AiJobInfo[] | null, only?: readonly AiJob[]): Promise<void> {
  for (const job of betaJobs(model, jobs).filter((one) => !only || only.includes(one))) {
    if (!jobs?.find((info) => info.job === job)?.beta_on) await aiBetaSet(job, true);
  }
}

export function useModelActions(jobs: readonly AiJobInfo[] | null): ModelActions {
  const [asking, setAsking] = useState<{ model: CatalogModel; recommended: boolean; only?: readonly AiJob[] } | null>(null);
  const [checking, setChecking] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const run = (work: () => Promise<unknown>) => {
    setNote(null);
    void work().catch((cause: unknown) => setNote(errorText(cause)));
  };
  /** After a click: started, the approval card, or why not. */
  const answered = async (answer: StartInfo, model: CatalogModel, recommended: boolean, only?: readonly AiJob[]) => {
    if (answer.started) {
      if (recommended) await turnOnBetas(model, jobs, only);
      return;
    }
    if (answer.refusal === "needs_approval") setAsking({ model, recommended, only });
    else if (answer.refusal === "not_enough_disk") refreshModels();
    else setNote((answer.refusal && REFUSALS[answer.refusal]) ?? null);
  };
  return {
    asking: asking?.model ?? null,
    note,
    use: (model, only) => run(() => turnOnBetas(model, jobs, only)),
    download: (model, recommended, only) => run(async () => answered(await aiModelDownloadStart(model.id), model, recommended, only)),
    approve: () => {
      const ask = asking;
      setAsking(null);
      if (!ask) return;
      run(async () => {
        const approval = await aiModelApproveExtra(ask.model.id);
        await answered(await aiModelDownloadStart(ask.model.id, approval.token), ask.model, ask.recommended, ask.only);
      });
    },
    notNow: () => {
      const ask = asking;
      setAsking(null);
      if (ask) run(() => aiModelDeclineExtra(ask.model.id));
    },
    closeAsk: () => setAsking(null),
    cancel: (id) => run(() => aiModelDownloadCancel(id)),
    remove: (id) => run(() => aiModelRemove(id)),
    beta: (job, on) => run(() => aiBetaSet(job, on)),
    pick: (job, modelId) => run(() => aiJobPickSet(job, modelId)),
    checkAgain: () => {
      setChecking(true);
      run(async () => {
        try {
          // AJ-E1547: the check may start Bot's AI helper the first time:
          // the gate's sheet first (nothing when it's set up already).
          // Not now checks nothing; the next click asks again.
          if ((await osPromptRequest(BOT_HELPER)).result === "not_now") return;
          await aiEnginesCheck();
        } finally {
          setChecking(false);
          refreshModels();
        }
      });
    },
    checking,
    openStorage: () => run(() => aiOpenSystemPage("storage")),
    openAppleIntelligence: () => run(() => aiOpenSystemPage("apple_intelligence")),
  };
}

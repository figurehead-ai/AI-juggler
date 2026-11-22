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
  aiOpenSystemPage,
  type CatalogModel,
  type StartInfo,
} from "../../../lib/ipc/models";
import { refreshModels } from "../../../lib/ai-models";
import { betaJobs } from "./models-view";
import { REFUSALS } from "./models-words";

export interface ModelActions {
  /** The model whose approval card is open. */
  asking: CatalogModel | null;
  /** A refusal or failure to say, in words. */
  note: string | null;
  use: (model: CatalogModel) => void;
  download: (model: CatalogModel, recommended: boolean) => void;
  approve: () => void;
  notNow: () => void;
  closeAsk: () => void;
  cancel: (id: string) => void;
  remove: (id: string) => void;
  beta: (job: AiJob, on: boolean) => void;
  pick: (job: AiJob, modelId: string | null) => void;
  checkAgain: () => void;
  openStorage: () => void;
  openAppleIntelligence: () => void;
}

/** Turns on each of `model`'s Beta jobs that is off. */
async function turnOnBetas(model: CatalogModel, jobs: readonly AiJobInfo[] | null): Promise<void> {
  for (const job of betaJobs(model)) {
    if (!jobs?.find((info) => info.job === job)?.beta_on) await aiBetaSet(job, true);
  }
}

export function useModelActions(jobs: readonly AiJobInfo[] | null): ModelActions {
  const [asking, setAsking] = useState<{ model: CatalogModel; recommended: boolean } | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const run = (work: () => Promise<unknown>) => {
    setNote(null);
    void work().catch((cause: unknown) => setNote(errorText(cause)));
  };
  /** After a click: started, the approval card, or why not. */
  const answered = async (answer: StartInfo, model: CatalogModel, recommended: boolean) => {
    if (answer.started) {
      if (recommended) await turnOnBetas(model, jobs);
      return;
    }
    if (answer.refusal === "needs_approval") setAsking({ model, recommended });
    else if (answer.refusal === "not_enough_disk") refreshModels();
    else setNote((answer.refusal && REFUSALS[answer.refusal]) ?? null);
  };
  return {
    asking: asking?.model ?? null,
    note,
    use: (model) => run(() => turnOnBetas(model, jobs)),
    download: (model, recommended) => run(async () => answered(await aiModelDownloadStart(model.id), model, recommended)),
    approve: () => {
      const ask = asking;
      setAsking(null);
      if (!ask) return;
      run(async () => {
        const approval = await aiModelApproveExtra(ask.model.id);
        await answered(await aiModelDownloadStart(ask.model.id, approval.token), ask.model, ask.recommended);
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
    checkAgain: () => refreshModels(),
    openStorage: () => run(() => aiOpenSystemPage("storage")),
    openAppleIntelligence: () => run(() => aiOpenSystemPage("apple_intelligence")),
  };
}

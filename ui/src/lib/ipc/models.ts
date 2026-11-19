/**
 * AJ-E3812 (with AJ-E3803, AJ-E3813) — Settings › AI › Models, as the
 * window reads it: the screen's list (`ai_models_catalog`: this Mac, the
 * recommendation, the models it can use), the models on this Mac and the
 * download (`ai_models_installed`), and the user's clicks: Download,
 * Cancel, Remove, the approval card's Download and Not now, and two fixed
 * System Settings pages. The backend says when something changed
 * (`ai-models://changed`) and how a download goes (`ai-models://progress`).
 *
 * The preview and tests set `window.__AIJ_AI_CATALOG__` and
 * `__AIJ_AI_MODELS__`; the clicks change them as the backend would, and
 * no byte is ever fetched. With none, nothing is listed.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import { listenWhileShown } from "../shown-window";
import type { ApprovalInfo } from "../../types/generated/ApprovalInfo";
import type { CatalogModel } from "../../types/generated/CatalogModel";
import type { DownloadInfo } from "../../types/generated/DownloadInfo";
import type { InstalledModelInfo } from "../../types/generated/InstalledModelInfo";
import type { ModelsCatalog } from "../../types/generated/ModelsCatalog";
import type { ModelsInfo } from "../../types/generated/ModelsInfo";
import type { StartInfo } from "../../types/generated/StartInfo";
import type { SystemPage } from "../../types/generated/SystemPage";

export type { ApprovalInfo, CatalogModel, DownloadInfo, InstalledModelInfo, ModelsCatalog, ModelsInfo, StartInfo, SystemPage };

/** The backend's events. */
export const AI_MODELS_CHANGED = "ai-models://changed";
export const AI_MODELS_PROGRESS = "ai-models://progress";

declare global {
  interface Window {
    __AIJ_AI_CATALOG__?: ModelsCatalog;
    __AIJ_AI_MODELS__?: ModelsInfo;
    /** The answer the preview's next Download click gets (a refusal). */
    __AIJ_AI_START__?: StartInfo;
    __AIJ_OPENED__?: Array<{ kind: string; id: string }>;
  }
}

const NO_CATALOG: ModelsCatalog = { listed: false, mac: null, recommendation: null, models: [] };
const NO_MODELS: ModelsInfo = { installed: [], total_bytes: 0, download: null, declined: [] };

const seamModels = (): ModelsInfo => window.__AIJ_AI_MODELS__ ?? NO_MODELS;

/** The preview's backend: change the models, say so. */
function changeModels(next: ModelsInfo): ModelsInfo {
  window.__AIJ_AI_MODELS__ = next;
  window.dispatchEvent(new Event(AI_MODELS_CHANGED));
  return next;
}

/** The screen's list: this Mac, the recommendation, the models it can use. */
export async function aiModelsCatalog(): Promise<ModelsCatalog> {
  if (!hasLiveBackend()) return window.__AIJ_AI_CATALOG__ ?? NO_CATALOG;
  return call("ai_models_catalog");
}

/** The models on this Mac, their total, and the download. */
export async function aiModelsInstalled(): Promise<ModelsInfo> {
  if (!hasLiveBackend()) return seamModels();
  return call("ai_models_installed");
}

/** A Download click (`approval`: the approval card's token). */
export async function aiModelDownloadStart(modelId: string, approval: string | null = null): Promise<StartInfo> {
  if (hasLiveBackend()) return call("ai_model_download_start", { modelId, approval });
  const refused = window.__AIJ_AI_START__;
  if (refused && !(approval && refused.refusal === "needs_approval")) return refused;
  const size = window.__AIJ_AI_CATALOG__?.models.find((model) => model.id === modelId)?.size_bytes ?? 0;
  const download: DownloadInfo = { id: modelId, done_bytes: 0, size_bytes: size, state: "running", wait: null, failure: null };
  changeModels({ ...seamModels(), download });
  return { started: true, refusal: null, size_bytes: null, total_after_bytes: null, needed_bytes: null, free_bytes: null };
}

/** Cancels the download; nothing of it is kept. */
export async function aiModelDownloadCancel(modelId: string): Promise<ModelsInfo> {
  if (hasLiveBackend()) return call("ai_model_download_cancel", { modelId });
  const models = seamModels();
  const download = models.download && { ...models.download, state: "cancelled" as const, done_bytes: 0 };
  return changeModels({ ...models, download });
}

/** Removes a model from this Mac. */
export async function aiModelRemove(modelId: string): Promise<ModelsInfo> {
  if (hasLiveBackend()) return call("ai_model_remove", { modelId });
  const models = seamModels();
  const gone = models.installed.find((model) => model.id === modelId);
  return changeModels({
    ...models,
    installed: models.installed.filter((model) => model.id !== modelId),
    total_bytes: models.total_bytes - (gone?.size_bytes ?? 0),
  });
}

/** The approval card's Download: a one-time token for this model. */
export async function aiModelApproveExtra(modelId: string): Promise<ApprovalInfo> {
  if (hasLiveBackend()) return call("ai_model_approve_extra", { modelId });
  const size = window.__AIJ_AI_CATALOG__?.models.find((model) => model.id === modelId)?.size_bytes ?? 0;
  return { id: modelId, token: "preview", size_bytes: size, total_after_bytes: seamModels().total_bytes + size };
}

/** The approval card's Not now: remembered. */
export async function aiModelDeclineExtra(modelId: string): Promise<ModelsInfo> {
  if (hasLiveBackend()) return call("ai_model_decline_extra", { modelId });
  const models = seamModels();
  return changeModels({ ...models, declined: [...models.declined.filter((id) => id !== modelId), modelId] });
}

/** Opens Storage or Apple Intelligence in System Settings. */
export async function aiOpenSystemPage(page: SystemPage): Promise<void> {
  if (hasLiveBackend()) return call("ai_open_system_page", { page });
  (window.__AIJ_OPENED__ ??= []).push({ kind: "system", id: page });
}

/** Calls `on` when the models changed, and once when the window is shown
 * again. Returns the stop. */
export function onAiModelsChanged(on: () => void): () => void {
  const heard = () => on();
  window.addEventListener(AI_MODELS_CHANGED, heard);
  const stop = hasLiveBackend() ? listenWhileShown<null>(AI_MODELS_CHANGED, heard, heard) : null;
  return () => {
    window.removeEventListener(AI_MODELS_CHANGED, heard);
    stop?.();
  };
}

/** Calls `on` with each step of a running download (the preview's
 * downloads take no steps). Returns the stop. */
export function onAiModelsProgress(on: (info: DownloadInfo) => void): () => void {
  if (!hasLiveBackend()) return () => undefined;
  return listenWhileShown<DownloadInfo>(AI_MODELS_PROGRESS, on, () => undefined);
}

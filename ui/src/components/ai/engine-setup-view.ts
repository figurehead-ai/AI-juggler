/**
 * AJ-E3823 — what the engine-setup guide shows, worked out from the
 * Models screen's reads (this Mac, the recommendation, the models it can
 * use), the models on this Mac and Bot's job, with no drawing. Pure, so
 * each state is a test of data.
 *
 * - **Apple's built-in AI** is ready, turned off, still being downloaded
 *   by the system, can't run here, or couldn't be checked (no answer, or
 *   its helper blocked): an unknown is never "can't run it".
 * - **The downloadable AI** is the one the recommendation names, else
 *   the one on this Mac, else the smallest offered that passed Bot's
 *   answers: named, sized, with this Mac's free disk.
 * - **The recommended badge**: Apple's when it is ready, off or getting
 *   ready (turning it on is free, nothing to download); the download only
 *   when Apple's can't run here; none while Apple's couldn't be checked.
 */
import type { AiJobInfo } from "../../lib/ipc/ai";
import type { CatalogModel, ModelsCatalog, ModelsInfo } from "../../lib/ipc/models";
import type { AppleAiState } from "../../types/generated/AppleAiState";
import type { NothingWhy } from "../../types/generated/NothingWhy";

/** Apple's built-in AI, as the guide says it. */
export type AppleSetup = "ready" | "off" | "preparing" | "cant_run" | "unchecked" | "no_answer" | "blocked";

const APPLE_OF: Record<AppleAiState, AppleSetup> = {
  on: "ready",
  off: "off",
  preparing: "preparing",
  unavailable: "cant_run",
  unchecked: "unchecked",
  no_answer: "no_answer",
  blocked: "blocked",
};

/** Apple's state couldn't be read. */
export function unknown(apple: AppleSetup | null): boolean {
  return apple === "unchecked" || apple === "no_answer" || apple === "blocked";
}

/** What the downloadable AI's card does now. */
type DownloadAction = "in-use" | "use" | "download" | "downloading" | "short";

export interface SetupView {
  /** Nothing read yet: "Checking what this Mac can do…". */
  checking: boolean;
  apple: AppleSetup | null;
  /** Apple's entry (a stand-in when the list doesn't show it: it shows
   * only while Apple Intelligence is on). */
  appleModel: CatalogModel;
  /** Bot's answers are written by Apple's AI now. */
  appleInUse: boolean;
  /** The downloadable AI, when one fits this Mac and passed Bot's answers. */
  download: CatalogModel | null;
  downloadAction: DownloadAction | null;
  /** Which card carries the badge. */
  recommended: "apple" | "download" | null;
  /** Neither can run here: why, in the recommendation's words. */
  noAi: NothingWhy | "other" | null;
  /** Free disk where AI files live, bytes. */
  free: number;
}

/** Drawn only (its tile and facts): no id, so nothing routes to it. */
const APPLE_STAND_IN: CatalogModel = {
  id: "",
  name: "Apple's on-device model",
  built_in: true,
  offered: false,
  size_bytes: null,
  licence_name: null,
  licence_url: null,
  min_chip: 1,
  min_memory_gb: 8,
  free_disk_needed_bytes: 0,
  speed: null,
  verdicts: [],
};

function passesBot(model: CatalogModel): boolean {
  const chat = model.verdicts.find((v) => v.job === "chat")?.verdict;
  return chat === "beta" || chat === "ships";
}

/** Whether Bot's answers run on `id` now. */
function inUse(job: AiJobInfo | null, id: string): boolean {
  return (job?.state === "ready" || job?.state === "waiting") && job.model_id === id;
}

const UNDER_WAY = new Set(["running", "waiting", "verifying"]);

/** The downloadable AI the guide names. */
function downloadOf(catalog: ModelsCatalog, installed: ModelsInfo | null): CatalogModel | null {
  const candidates = catalog.models.filter((m) => !m.built_in && passesBot(m));
  const here = new Set(installed?.installed.map((m) => m.id) ?? []);
  const named = candidates.find((m) => m.id === catalog.recommendation?.model_id);
  const onThisMac = candidates.find((m) => here.has(m.id));
  const smallest = [...candidates].filter((m) => m.offered).sort((a, b) => (a.size_bytes ?? 0) - (b.size_bytes ?? 0))[0];
  return named ?? onThisMac ?? smallest ?? null;
}

/** What the downloadable AI's card does now. */
function actionOf(download: CatalogModel, installed: ModelsInfo | null, job: AiJobInfo | null, free: number): DownloadAction {
  if (installed?.installed.some((m) => m.id === download.id)) return inUse(job, download.id) ? "in-use" : "use";
  if (installed?.download?.id === download.id && UNDER_WAY.has(installed.download.state)) return "downloading";
  return free < download.free_disk_needed_bytes ? "short" : "download";
}

/** The guide's view, from the Models screen's reads and Bot's job. */
export function setupView(catalog: ModelsCatalog | null, installed: ModelsInfo | null, job: AiJobInfo | null): SetupView {
  const mac = catalog?.mac ?? null;
  if (!catalog || !mac) {
    return { checking: true, apple: null, appleModel: APPLE_STAND_IN, appleInUse: false, download: null, downloadAction: null, recommended: null, noAi: null, free: 0 };
  }
  const apple: AppleSetup = mac.apple_silicon ? APPLE_OF[mac.apple_ai] : "cant_run";
  const appleModel = catalog.models.find((m) => m.built_in) ?? APPLE_STAND_IN;
  const download = downloadOf(catalog, installed);
  const free = mac.free_disk_bytes;
  const downloadAction = download ? actionOf(download, installed, job, free) : null;
  let recommended: SetupView["recommended"] = null;
  if (apple === "ready" || apple === "off" || apple === "preparing") recommended = "apple";
  else if (apple === "cant_run" && download) recommended = "download";
  const noAi = apple === "cant_run" && !download ? (catalog.recommendation?.nothing ?? "other") : null;
  return { checking: false, apple, appleModel, appleInUse: inUse(job, appleModel.id), download, downloadAction, recommended, noAi, free };
}

/** The reasons nothing is recommended that the guide answers: Apple's AI
 * can run here but isn't on yet, or couldn't be checked. */
const SETUP_REASONS: ReadonlySet<NothingWhy> = new Set(["turn_on_apple_ai", "apple_ai_getting_ready", "apple_ai_unknown"]);

/** Whether Settings › AI › Models leads with the guide (in the
 * Recommended card's place). */
export function showsSetup(catalog: ModelsCatalog | null): boolean {
  const why = catalog?.recommendation?.nothing;
  return !!why && SETUP_REASONS.has(why);
}

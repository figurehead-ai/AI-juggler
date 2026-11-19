/**
 * AJ-E3812 — the Models screen's preview data: the tested models as the
 * nightly list carries them (config/remote/nightly.json), three Macs, and
 * each state of the approved mock-up (`?s=`): Apple recommended, a
 * download recommended, downloading, the second-model approval, not
 * enough free disk, the list unread (offline), and installed with a
 * withdrawn model. Used by the component tests and the gallery; nothing
 * here is fetched.
 */
import type { AiJobInfo } from "../lib/ipc/ai";
import type { CatalogModel, ModelsCatalog, ModelsInfo, StartInfo } from "../lib/ipc/models";
import type { MacInfo } from "../types/generated/MacInfo";
import type { ModelJobVerdict } from "../types/generated/ModelJobVerdict";
import type { RecommendationInfo } from "../types/generated/RecommendationInfo";

const GB = 1_000_000_000;
const GIB = 2 ** 30;
const MARGIN = 10 * GB;
/** 12 and 14 Nov 2026, noon UTC. */
const NOV_12 = Date.UTC(2026, 10, 12, 12);
const NOV_14 = Date.UTC(2026, 10, 14, 12);

const v = (job: ModelJobVerdict["job"], verdict: ModelJobVerdict["verdict"], note: string | null): ModelJobVerdict => ({ job, verdict, note });

function model(spec: {
  id: string;
  name: string;
  size: number | null;
  licence: { name: string; url: string } | null;
  memory: number;
  speed: CatalogModel["speed"];
  verdicts: ModelJobVerdict[];
}): CatalogModel {
  return {
    id: spec.id,
    name: spec.name,
    built_in: spec.size === null,
    offered: true,
    size_bytes: spec.size,
    licence_name: spec.licence?.name ?? null,
    licence_url: spec.licence?.url ?? null,
    min_chip: 1,
    min_memory_gb: spec.memory,
    free_disk_needed_bytes: spec.size === null ? 0 : spec.size + MARGIN,
    speed: spec.speed,
    verdicts: spec.verdicts,
  };
}

export const APPLE: CatalogModel = model({
  id: "apple-on-device",
  name: "Apple's on-device model",
  size: null,
  licence: null,
  memory: 8,
  speed: null,
  verdicts: [
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 81 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Preferred over the plain answer in 75 % of blind checks."),
    v("digest", "not_yet", "The plain roundup reads better for now."),
    v("watches", "not_yet", "Drafts matched the request 70 % of the time; we want 95 %."),
  ],
});

export const QWEN: CatalogModel = model({
  id: "qwen3-1-7b-q8",
  name: "Qwen3 1.7B",
  size: 1834426016,
  licence: { name: "Apache-2.0", url: "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/blob/90862c4b9d2787eaed51d12237eafdfe7c5f6077/LICENSE" },
  memory: 8,
  speed: "quick",
  verdicts: [
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 89 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Answers in about half a second; preferred in 76 % of blind checks."),
    v("digest", "not_yet", "The plain roundup reads better for now."),
    v("watches", "not_yet", "Drafts matched the request 81 % of the time; we want 95 %."),
  ],
});

export const NEMOTRON: CatalogModel = model({
  id: "nemotron-3-nano-4b-q4km",
  name: "Nemotron 3 Nano 4B",
  size: 2837072864,
  licence: { name: "NVIDIA Nemotron Open Model License", url: "https://huggingface.co/nvidia/NVIDIA-Nemotron-3-Nano-4B-GGUF/blob/ba223d14e45525f7fae81db77ea8cabeb2fc6c25/README.md" },
  memory: 8,
  speed: "moderate",
  verdicts: [
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 71 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Preferred over the plain answer in 80 % of blind checks."),
    v("digest", "not_yet", "The plain roundup reads better for now."),
    v("watches", "not_yet", "Drafts matched the request 85 % of the time; we want 95 %."),
  ],
});

export const GRANITE: CatalogModel = model({
  id: "granite-4-1-3b-q4km",
  name: "Granite 4.1 3B",
  size: 2099501664,
  licence: { name: "Apache-2.0", url: "https://huggingface.co/ibm-granite/granite-4.1-3b-GGUF/blob/ab4701481089b58a082ef63cc1cee738887293ff/README.md" },
  memory: 8,
  speed: "quick",
  verdicts: [
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 77 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Preferred over the plain answer in 79 % of blind checks."),
    v("digest", "not_yet", "The plain roundup reads better for now."),
    v("watches", "not_yet", "Drafts matched the request 85 % of the time; we want 95 %."),
  ],
});

export const GEMMA: CatalogModel = model({
  id: "gemma-4-e2b-q4-0",
  name: "Gemma 4 E2B",
  size: 3349516256,
  licence: { name: "Apache-2.0", url: "https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf/blob/675cff42a74c774d6cb76f76d8eacb49b48c9b93/README.md" },
  memory: 16,
  speed: "moderate",
  verdicts: [
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 74 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Preferred over the plain answer in 89 % of blind checks."),
    v("digest", "not_yet", "The plain roundup reads better for now."),
    v("watches", "not_yet", "Drafts matched the request 70 % of the time; we want 95 %."),
  ],
});

/** The downloads, smallest first. */
export const DOWNLOADS = [QWEN, GRANITE, NEMOTRON, GEMMA];

/** A Mac with Apple Intelligence on (M3, 18 GB, 212 GB free). */
export const MAC_AI: MacInfo = { apple_silicon: true, chip: 3, memory_bytes: 18 * GIB, memory_gb: 19, free_disk_bytes: 212 * GB, apple_ai: "on" };
/** A Mac without it (M2, 16 GB, 48 GB free). */
export const MAC_PLAIN: MacInfo = { apple_silicon: true, chip: 2, memory_bytes: 16 * GIB, memory_gb: 17, free_disk_bytes: 48 * GB, apple_ai: "off" };

const rec = (over: Partial<RecommendationInfo>): RecommendationInfo => ({ model_id: null, why: null, nothing: null, needed_bytes: null, apple_ai_off: false, ...over });

export function catalog(mac: MacInfo, recommendation: RecommendationInfo, models: CatalogModel[], listed = true): ModelsCatalog {
  return { listed, mac, recommendation, models };
}

export const NO_MODELS: ModelsInfo = { installed: [], total_bytes: 0, download: null, declined: [] };

function installed(...rows: Array<[CatalogModel, number, "listed" | "withdrawn" | "unknown"]>): ModelsInfo {
  const list = rows.map(([m, at, standing]) => ({ id: m.id, name: m.name, size_bytes: m.size_bytes ?? 0, installed_at_ms: at, licence_name: m.licence_name, standing }));
  return { installed: list, total_bytes: list.reduce((sum, row) => sum + row.size_bytes, 0), download: null, declined: [] };
}

/** The AI jobs' answer: review and Chat with their Beta switch, the rest off. */
export function jobs(over: Partial<Record<"review" | "chat", Partial<AiJobInfo>>> = {}): AiJobInfo[] {
  const base = (job: AiJobInfo["job"], state: AiJobInfo["state"]): AiJobInfo => ({ job, state, beta: false, beta_on: false, model_id: null, model_name: null, why: null, pick: null });
  return [
    { ...base("feed", "off") },
    { ...base("review", "beta_off"), ...over.review },
    { ...base("chat", "beta_off"), ...over.chat },
    { ...base("digest", "off") },
    { ...base("watches", "off") },
  ];
}

const on = (id: string, name: string, pick: string | null = null): Partial<AiJobInfo> => ({ state: "ready", beta: true, beta_on: true, model_id: id, model_name: name, why: pick ? "picked" : "recommended", pick });

export interface ModelsState {
  catalog: ModelsCatalog;
  models: ModelsInfo;
  jobs: AiJobInfo[];
  /** The next Download click's answer. */
  start?: StartInfo;
}

const NEEDS_APPROVAL: StartInfo = { started: false, refusal: "needs_approval", size_bytes: GRANITE.size_bytes, total_after_bytes: (QWEN.size_bytes ?? 0) + (GRANITE.size_bytes ?? 0), needed_bytes: null, free_bytes: null };

/** Each state of the mock-up, by its `?s=` name. */
export const STATES: Record<string, ModelsState> = {
  models: { catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, ...DOWNLOADS]), models: NO_MODELS, jobs: jobs() },
  noapple: { catalog: catalog(MAC_PLAIN, rec({ model_id: QWEN.id, why: "fits" }), DOWNLOADS), models: NO_MODELS, jobs: jobs({ review: { state: "no_engine" }, chat: { state: "no_engine" } }) },
  downloading: {
    catalog: catalog(MAC_PLAIN, rec({ model_id: QWEN.id, why: "fits" }), DOWNLOADS),
    models: { ...NO_MODELS, download: { id: QWEN.id, done_bytes: 0.7 * GB, size_bytes: QWEN.size_bytes ?? 0, state: "running", wait: null, failure: null } },
    jobs: jobs({ review: { state: "no_engine", beta_on: true }, chat: { state: "no_engine", beta_on: true } }),
  },
  approval: {
    catalog: catalog({ ...MAC_PLAIN, free_disk_bytes: 46 * GB }, rec({ model_id: QWEN.id, why: "installed" }), DOWNLOADS),
    models: installed([QWEN, NOV_12, "listed"]),
    jobs: jobs({ review: on(QWEN.id, QWEN.name), chat: on(QWEN.id, QWEN.name) }),
    start: NEEDS_APPROVAL,
  },
  nodisk: {
    catalog: catalog({ ...MAC_PLAIN, free_disk_bytes: 11.2 * GB }, rec({ nothing: "not_enough_disk", needed_bytes: QWEN.free_disk_needed_bytes, apple_ai_off: true }), DOWNLOADS),
    models: NO_MODELS,
    jobs: jobs({ review: { state: "no_engine" }, chat: { state: "no_engine" } }),
  },
  offline: {
    catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, { ...QWEN, offered: false }], false),
    models: installed([QWEN, NOV_12, "unknown"]),
    jobs: jobs({ review: on(APPLE.id, APPLE.name), chat: on(APPLE.id, APPLE.name) }),
  },
  installed: {
    catalog: catalog({ ...MAC_AI, free_disk_bytes: 207 * GB }, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, QWEN, GRANITE, GEMMA, { ...NEMOTRON, offered: false }]),
    models: installed([QWEN, NOV_12, "listed"], [NEMOTRON, NOV_14, "withdrawn"]),
    jobs: jobs({ review: on(APPLE.id, APPLE.name), chat: on(NEMOTRON.id, NEMOTRON.name, NEMOTRON.id) }),
  },
  nolist: {
    catalog: catalog({ ...MAC_PLAIN, apple_ai: "unavailable" }, rec({ nothing: "nothing_listed" }), [], false),
    models: NO_MODELS,
    jobs: jobs({ review: { state: "no_engine" }, chat: { state: "no_engine" } }),
  },
};

/** Sets a state's seams on `window` (the preview and the tests). */
export function seedModels(state: ModelsState): void {
  window.__AIJ_AI_CATALOG__ = state.catalog;
  window.__AIJ_AI_MODELS__ = state.models;
  window.__AIJ_AI_JOBS__ = state.jobs;
  if (state.start) window.__AIJ_AI_START__ = state.start;
  else delete window.__AIJ_AI_START__;
}

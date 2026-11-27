/**
 * AJ-E3812 — the Models screen's preview data: the tested models as the
 * nightly list carries them (config/remote/nightly.json), three Macs, and
 * each state of the approved mock-up (`?s=`): Apple recommended, a
 * download recommended, downloading, the second-model approval, not
 * enough free disk, the list unread (offline), installed with a
 * withdrawn model, and (AJ-E3823) Apple Intelligence off, its helper
 * unanswered, and its helper blocked: the engine-setup guide on top. Used by the component tests and the gallery; nothing
 * here is fetched.
 */
import type { AiJobInfo } from "../lib/ipc/ai";
import type { AiEngineStatusInfo } from "../types/generated/AiEngineStatusInfo";
import { ENGINE_OFF } from "../lib/ai-engine-off";
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

const APPLE: CatalogModel = model({
  id: "apple-on-device",
  name: "Apple's on-device model",
  size: null,
  licence: null,
  memory: 8,
  speed: null,
  verdicts: [
    // AJ-E3826: as builtin.json has them (D-149 and its amendments).
    v("feed", "not_yet", "The plain line reads better for now."),
    v("review", "beta", "Preferred over the plain review in 81 % of blind checks."),
    v("attention", "ships", "Built from the facts; no misses in testing."),
    v("chat", "beta", "Preferred over the plain answer in 75 % of blind checks."),
    v("digest", "beta", "Preferred over the plain roundup in 64 % of blind checks."),
    v("watches", "not_yet", "Fills in what it understands from your words; you check the form and save."),
  ],
});

const QWEN: CatalogModel = model({
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
    v("watches", "not_yet", "Fills in what it understands from your words; you check the form and save."),
  ],
});

const NEMOTRON: CatalogModel = model({
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
    v("watches", "not_yet", "Fills in what it understands from your words; you check the form and save."),
  ],
});

const GRANITE: CatalogModel = model({
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
    v("watches", "not_yet", "Fills in what it understands from your words; you check the form and save."),
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
    v("watches", "not_yet", "Fills in what it understands from your words; you check the form and save."),
  ],
});

/** The downloads, smallest first. */
const DOWNLOADS = [QWEN, GRANITE, NEMOTRON, GEMMA];

/** A Mac with Apple Intelligence on (M3, 18 GB, 212 GB free). */
const MAC_AI: MacInfo = { apple_silicon: true, chip: 3, memory_bytes: 18 * GIB, memory_gb: 19, free_disk_bytes: 212 * GB, apple_ai: "on" };
/** A Mac that can't run it (M2, 16 GB, 48 GB free). */
const MAC_PLAIN: MacInfo = { apple_silicon: true, chip: 2, memory_bytes: 16 * GIB, memory_gb: 17, free_disk_bytes: 48 * GB, apple_ai: "unavailable" };
/** The first Mac with Apple Intelligence turned off, then two where it
 * couldn't be checked (AJ-E3823): its helper never answered, or was
 * blocked by the system. */
const MAC_OFF: MacInfo = { ...MAC_AI, apple_ai: "off" };
const MAC_NO_ANSWER: MacInfo = { ...MAC_AI, apple_ai: "no_answer" };
const MAC_BLOCKED: MacInfo = { ...MAC_AI, apple_ai: "blocked" };

const rec = (over: Partial<RecommendationInfo>): RecommendationInfo => ({ model_id: null, why: null, nothing: null, needed_bytes: null, apple_ai_off: false, ...over });

export function catalog(mac: MacInfo, recommendation: RecommendationInfo, models: CatalogModel[], listed = true): ModelsCatalog {
  return { listed, mac, recommendation, models };
}

const NO_MODELS: ModelsInfo = { installed: [], total_bytes: 0, download: null, declined: [] };

function installed(...rows: Array<[CatalogModel, number, "listed" | "withdrawn" | "unknown"]>): ModelsInfo {
  const list = rows.map(([m, at, standing]) => ({ id: m.id, name: m.name, size_bytes: m.size_bytes ?? 0, installed_at_ms: at, licence_name: m.licence_name, standing }));
  return { installed: list, total_bytes: list.reduce((sum, row) => sum + row.size_bytes, 0), download: null, declined: [] };
}

/** The AI jobs' answer as this version gives it (AJ-E3826, D-149): Bot,
 * the digest and the turn review (AJ-E3820, for the tools whose turn
 * ends AI Juggler sees) with their Beta switch; the feed line on its
 * plain path by its verdict; watches on Bot's AI (AJ-E3830). */
export function jobs(over: Partial<Record<"review" | "chat" | "digest", Partial<AiJobInfo>>> = {}): AiJobInfo[] {
  const base = (job: AiJobInfo["job"], state: AiJobInfo["state"]): AiJobInfo => ({ job, state, beta: false, beta_on: false, model_id: null, model_name: null, why: null, pick: null, covers: [] });
  return [
    { ...base("feed", "no_engine") },
    { ...base("review", "beta_off"), covers: ["claude-code", "codex"], ...over.review },
    { ...base("chat", "beta_off"), ...over.chat },
    { ...base("digest", "beta_off"), ...over.digest },
    watchesOf({ ...base("chat", "beta_off"), ...over.chat }),
  ];
}

/** AJ-E3830 (0.5.2): Watches as the backend says it, on Bot's own state
 * and model; its Beta (Bot's confirm card) offered while Bot's AI runs. */
function watchesOf(chat: AiJobInfo): AiJobInfo {
  const on = chat.state === "ready" || chat.state === "waiting";
  return { job: "watches", state: chat.state, beta: on, beta_on: false, model_id: chat.model_id, model_name: chat.model_name, why: chat.why, pick: null, covers: [] };
}

/** Bot's engine as `ai_engine_status` gives it (AJ-E3825), for the
 * Models card (AJ-E3826). */
export function engine(over: Partial<AiEngineStatusInfo> = {}): AiEngineStatusInfo {
  return { ...structuredClone(ENGINE_OFF), ...over };
}

/** The gallery's clock (`models-gallery.spec.ts`): 2026-11-19 12:00 UTC. */
const NOW = Date.UTC(2026, 10, 19, 12);

const running = (id: string, name: string, short: string, over: Partial<AiEngineStatusInfo> = {}): AiEngineStatusInfo =>
  engine({
    state: "running",
    model_id: id,
    model_name: name,
    short_name: short,
    engine: id === "apple-on-device" ? "apple_builtin" : "gguf_local",
    context_tokens: 4096,
    started_ms: NOW - 40 * 60_000,
    answered_ms: NOW - 2 * 60_000,
    today: { answers: 12, tokens_in: { n: 8_140, estimated: false }, tokens_out: { n: 2_310, estimated: true } },
    ...over,
  });

const on = (id: string, name: string, pick: string | null = null): Partial<AiJobInfo> => ({ state: "ready", beta: true, beta_on: true, model_id: id, model_name: name, why: pick ? "picked" : "recommended", pick });

export interface ModelsState {
  catalog: ModelsCatalog;
  models: ModelsInfo;
  jobs: AiJobInfo[];
  /** Bot's engine now (AJ-E3826's card); none: off. */
  engine?: AiEngineStatusInfo;
  /** The next Download click's answer. */
  start?: StartInfo;
}

const NEEDS_APPROVAL: StartInfo = { started: false, refusal: "needs_approval", size_bytes: GRANITE.size_bytes, total_after_bytes: (QWEN.size_bytes ?? 0) + (GRANITE.size_bytes ?? 0), needed_bytes: null, free_bytes: null };

/** Each state of the mock-up, by its `?s=` name. AJ-E3826: a model in
 * use opens on Bot's AI (`inuse`, `failed`, `approval`, `offline`,
 * `installed`); none in use, Recommended leads. */
const NONE = { chat: { state: "no_engine" }, digest: { state: "no_engine" }, review: { state: "no_engine" } } as const;
export const STATES: Record<string, ModelsState> = {
  models: { catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, ...DOWNLOADS]), models: NO_MODELS, jobs: jobs() },
  inuse: {
    catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, ...DOWNLOADS]),
    models: NO_MODELS,
    jobs: jobs({ chat: on(APPLE.id, APPLE.name), digest: on(APPLE.id, APPLE.name) }),
    engine: running(APPLE.id, APPLE.name, "Apple on-device"),
  },
  failed: {
    catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, ...DOWNLOADS]),
    models: NO_MODELS,
    jobs: jobs({ chat: on(APPLE.id, APPLE.name) }),
    engine: running(APPLE.id, APPLE.name, "Apple on-device", { state: "failed", failure: "timed_out", today: { answers: 3, tokens_in: { n: 2_480, estimated: false }, tokens_out: { n: 640, estimated: true } } }),
  },
  noapple: { catalog: catalog(MAC_PLAIN, rec({ model_id: QWEN.id, why: "fits" }), DOWNLOADS), models: NO_MODELS, jobs: jobs(NONE) },
  downloading: {
    catalog: catalog(MAC_PLAIN, rec({ model_id: QWEN.id, why: "fits" }), DOWNLOADS),
    models: { ...NO_MODELS, download: { id: QWEN.id, done_bytes: 0.7 * GB, size_bytes: QWEN.size_bytes ?? 0, state: "running", wait: null, failure: null } },
    jobs: jobs({ chat: { state: "no_engine", beta_on: true }, digest: { state: "no_engine" } }),
  },
  approval: {
    catalog: catalog({ ...MAC_PLAIN, free_disk_bytes: 46 * GB }, rec({ model_id: QWEN.id, why: "installed" }), DOWNLOADS),
    models: installed([QWEN, NOV_12, "listed"]),
    jobs: jobs({ chat: on(QWEN.id, QWEN.name), digest: { state: "no_engine" } }),
    engine: running(QWEN.id, QWEN.name, "Qwen3 1.7B", { state: "idle", answered_ms: NOW - 3 * 3_600_000, today: { answers: 4, tokens_in: { n: 3_120, estimated: false }, tokens_out: { n: 860, estimated: false } } }),
    start: NEEDS_APPROVAL,
  },
  nodisk: {
    catalog: catalog({ ...MAC_PLAIN, free_disk_bytes: 11.2 * GB }, rec({ nothing: "not_enough_disk", needed_bytes: QWEN.free_disk_needed_bytes }), DOWNLOADS),
    models: NO_MODELS,
    jobs: jobs(NONE),
  },
  offline: {
    catalog: catalog(MAC_AI, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, { ...QWEN, offered: false }], false),
    models: installed([QWEN, NOV_12, "unknown"]),
    jobs: jobs({ chat: on(APPLE.id, APPLE.name), digest: on(APPLE.id, APPLE.name) }),
    engine: running(APPLE.id, APPLE.name, "Apple on-device", { state: "idle", answered_ms: null, today: { answers: 0, tokens_in: { n: 0, estimated: false }, tokens_out: { n: 0, estimated: false } } }),
  },
  installed: {
    catalog: catalog({ ...MAC_AI, free_disk_bytes: 207 * GB }, rec({ model_id: APPLE.id, why: "built_in" }), [APPLE, QWEN, GRANITE, GEMMA, { ...NEMOTRON, offered: false }]),
    models: installed([QWEN, NOV_12, "listed"], [NEMOTRON, NOV_14, "withdrawn"]),
    jobs: jobs({ chat: on(NEMOTRON.id, NEMOTRON.name, NEMOTRON.id), digest: on(APPLE.id, APPLE.name) }),
    engine: running(NEMOTRON.id, NEMOTRON.name, "Nemotron 3 Nano 4B"),
  },
  appleoff: { catalog: catalog(MAC_OFF, rec({ nothing: "turn_on_apple_ai" }), DOWNLOADS), models: NO_MODELS, jobs: jobs(NONE) },
  noanswer: { catalog: catalog(MAC_NO_ANSWER, rec({ nothing: "apple_ai_unknown" }), DOWNLOADS), models: NO_MODELS, jobs: jobs(NONE) },
  blocked: { catalog: catalog(MAC_BLOCKED, rec({ nothing: "apple_ai_unknown" }), DOWNLOADS), models: NO_MODELS, jobs: jobs(NONE) },
  nolist: {
    catalog: catalog({ ...MAC_PLAIN, apple_ai: "unavailable" }, rec({ nothing: "nothing_listed" }), [], false),
    models: NO_MODELS,
    jobs: jobs(NONE),
  },
};

/** Sets a state's seams on `window` (the preview and the tests). */
export function seedModels(state: ModelsState): void {
  window.__AIJ_AI_CATALOG__ = state.catalog;
  window.__AIJ_AI_MODELS__ = state.models;
  window.__AIJ_AI_JOBS__ = state.jobs;
  if (state.engine) window.__AIJ_AI_ENGINE__ = state.engine;
  else delete window.__AIJ_AI_ENGINE__;
  if (state.start) window.__AIJ_AI_START__ = state.start;
  else delete window.__AIJ_AI_START__;
}

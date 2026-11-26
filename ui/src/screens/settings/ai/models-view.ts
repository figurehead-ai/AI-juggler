/**
 * AJ-E3812 — what Settings › AI › Models shows, worked out from its three
 * reads (the list, the models on this Mac, the AI jobs' answer), with no
 * drawing: which model is recommended and what its button does, the
 * other tested models smallest first with at most one tag each, and each
 * job's row. Pure, so each state is a test of data.
 */
import type { AiJob, AiJobInfo } from "../../../lib/ipc/ai";
import type { CatalogModel, ModelsCatalog, ModelsInfo } from "../../../lib/ipc/models";
import type { ModelJob } from "../../../types/generated/ModelJob";
import type { ModelSpeed } from "../../../types/generated/ModelSpeed";
import { FIT, JOBS, chipWords, memoryWords } from "./models-words";

/** The jobs a model card lists (attention is code, never the model's:
 * it shows only in Each job uses): the routed jobs. */
const MODEL_JOBS: readonly AiJob[] = ["review", "chat", "feed", "digest", "watches"];

/** Every job's row in Each job uses, in the mock's order: AJ-E3826
 * (D-149 #4) shows each job's real state, attention hints included (on,
 * built from the facts), never only the jobs a model passes. */
export const JOB_ROWS: readonly ModelJob[] = ["review", "chat", "attention", "feed", "digest", "watches"];

export function verdictOf(model: CatalogModel, job: ModelJob) {
  return model.verdicts.find((verdict) => verdict.job === job) ?? { job, verdict: "not_tested" as const, note: null };
}

function passed(model: CatalogModel, job: ModelJob): boolean {
  const { verdict } = verdictOf(model, job);
  return verdict === "beta" || verdict === "ships";
}

/** The jobs it passed, and the ones it didn't yet (attention aside).
 * AJ-E3826: a job this version doesn't have (`absent`, off) is in
 * neither: a card never offers what Use it can't turn on. */
export function jobsOf(model: CatalogModel, absent: readonly AiJob[] = []): { passed: AiJob[]; notYet: AiJob[] } {
  const here = MODEL_JOBS.filter((job) => !absent.includes(job));
  return {
    passed: here.filter((job) => passed(model, job)),
    notYet: here.filter((job) => !passed(model, job)),
  };
}

/** The jobs this version doesn't have now (off: not built yet, or
 * paused by an update). */
export function absentJobs(jobs: readonly AiJobInfo[] | null): AiJob[] {
  return (jobs ?? []).filter((info) => info.state === "off").map((info) => info.job);
}

/** The jobs it passed as a Beta: what Use it and Download turn on.
 * AJ-E3826: with the jobs' answer, only the jobs this version has (a job
 * that isn't built yet is never promised). */
export function betaJobs(model: CatalogModel, jobs?: readonly AiJobInfo[] | null): AiJob[] {
  return MODEL_JOBS.filter((job) => verdictOf(model, job).verdict === "beta" && jobs?.find((info) => info.job === job)?.state !== "off");
}

/** What the Recommended card's action is. */
type RecommendedAction = "use" | "in-use" | "download" | "downloading" | "no-disk";

export interface Recommended {
  model: CatalogModel;
  /** The Beta jobs its Use it or Download turns on (this version's). */
  turnsOn: AiJob[];
  /** The jobs this version doesn't have now (its card leaves them out). */
  absent: AiJob[];
  action: RecommendedAction;
  /** The reason is "the smallest that passed" (else "passed the most"). */
  smallest: boolean;
}

const SPEED_RANK: Record<ModelSpeed, number> = { quick: 0, moderate: 1, slow: 2 };

/** The downloadable models offered here, smallest first. */
function downloads(catalog: ModelsCatalog): CatalogModel[] {
  return catalog.models.filter((model) => model.offered && !model.built_in).sort((a, b) => (a.size_bytes ?? 0) - (b.size_bytes ?? 0));
}

function installedIds(installed: ModelsInfo | null): Set<string> {
  return new Set(installed?.installed.map((model) => model.id) ?? []);
}

/** Whether the jobs a model passed as Beta are on (Use it was said). */
function betaOn(model: CatalogModel, jobs: readonly AiJobInfo[] | null): boolean {
  const wanted = betaJobs(model, jobs);
  return wanted.length > 0 && wanted.every((job) => jobs?.find((info) => info.job === job)?.beta_on);
}

/** The recommended model and its action; with nothing recommended for
 * want of free disk, the model needing the least of it, its Download
 * held (the card says what it needs). `null`: nothing to show. */
export function recommended(catalog: ModelsCatalog, installed: ModelsInfo | null, jobs: readonly AiJobInfo[] | null): Recommended | null {
  const pick = catalog.recommendation;
  const offered = downloads(catalog);
  const smallestId = offered[0]?.id;
  if (pick?.nothing === "not_enough_disk") {
    const short = [...offered].sort((a, b) => a.free_disk_needed_bytes - b.free_disk_needed_bytes)[0];
    return short ? { model: short, turnsOn: betaJobs(short, jobs), absent: absentJobs(jobs), action: "no-disk", smallest: short.id === smallestId } : null;
  }
  const model = catalog.models.find((known) => known.id === pick?.model_id);
  if (!model) return null;
  const here = model.built_in || installedIds(installed).has(model.id);
  const download = installed?.download;
  const running = download?.id === model.id && ["running", "waiting", "verifying"].includes(download.state);
  let action: RecommendedAction = "download";
  if (here) action = betaOn(model, jobs) ? "in-use" : "use";
  else if (running) action = "downloading";
  return { model, turnsOn: betaJobs(model, jobs), absent: absentJobs(jobs), action, smallest: model.id === smallestId };
}

/** The other tested models: offered downloads not on this Mac and not
 * the recommended one, smallest first. */
export function others(catalog: ModelsCatalog, installed: ModelsInfo | null, recommendedId: string | null): CatalogModel[] {
  const here = installedIds(installed);
  return downloads(catalog).filter((model) => !here.has(model.id) && model.id !== recommendedId);
}

export type ModelTag = "smallest" | "fastest";

/** At most one tag per model, from the data: the smallest download, then
 * the one quicker than every other left (none when two tie). */
export function tags(catalog: ModelsCatalog): Map<string, ModelTag> {
  const offered = downloads(catalog);
  const out = new Map<string, ModelTag>();
  if (offered.length < 2) return out;
  out.set(offered[0]!.id, "smallest");
  const rest = offered.slice(1).filter((model) => model.speed);
  const best = Math.min(...rest.map((model) => SPEED_RANK[model.speed!]));
  const quickest = rest.filter((model) => SPEED_RANK[model.speed!] === best);
  if (quickest.length === 1) out.set(quickest[0]!.id, "fastest");
  return out;
}

/** One line of the "Why it fits" checklist: met, its name, its value. */
export type FitRow = [ok: boolean, key: string, value: string];

/** The checklist for `model` on this Mac. */
export function fitRows(catalog: ModelsCatalog, model: CatalogModel, installed: ModelsInfo | null): FitRow[] {
  const mac = catalog.mac;
  if (!mac) return [];
  const memory = memoryWords(mac.memory_bytes, mac.memory_gb);
  const memoryRow: FitRow = [true, FIT.memory, FIT.needsMemory(memory, model.min_memory_gb)];
  if (model.built_in) return [[true, FIT.appleAi, FIT.on], memoryRow, [true, FIT.download, FIT.noDownload]];
  const chip: FitRow = [true, FIT.chip, FIT.needsChip(chipWords(mac.apple_silicon, mac.chip), model.min_chip)];
  const copy = installed?.installed.find((known) => known.id === model.id);
  if (copy) return [chip, memoryRow, [true, FIT.installed, FIT.installedOn(copy.size_bytes, dayOf(copy.installed_at_ms))]];
  const disk: FitRow = [mac.free_disk_bytes >= model.free_disk_needed_bytes, FIT.freeDisk, FIT.needsDisk(mac.free_disk_bytes, model.free_disk_needed_bytes)];
  return [chip, memoryRow, disk];
}

/** A day as "Nov 12" (the reader's own order). */
export function dayOf(ms: number): string {
  return new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" });
}

/** The models a job may be picked for: on this Mac (Apple's, when it's
 * shown) and passed for that job (D-127). */
export function pickable(catalog: ModelsCatalog | null, installed: ModelsInfo | null, job: ModelJob): CatalogModel[] {
  const here = installedIds(installed);
  return (catalog?.models ?? []).filter((model) => (model.built_in || here.has(model.id)) && passed(model, job));
}

/** The model's display name by id, from the list or the installed rows. */
export function nameOf(id: string | null | undefined, catalog: ModelsCatalog | null, installed: ModelsInfo | null): string | null {
  if (!id) return null;
  return catalog?.models.find((model) => model.id === id)?.name ?? installed?.installed.find((model) => model.id === id)?.name ?? null;
}

/** Two letters for a model's tile ("Qw" for Qwen3 1.7B). */
export function initials(name: string): string {
  const letters = name.replace(/[^A-Za-z]/g, "");
  return `${letters.charAt(0).toUpperCase()}${letters.charAt(1).toLowerCase()}`;
}

/** AJ-E3826: whether a model is in use: some job runs on one (or is
 * getting it ready). Until then Recommended for this Mac leads; after,
 * Bot's AI does, and Recommended steps aside. */
export function modelInUse(jobs: readonly AiJobInfo[] | null): boolean {
  return (jobs ?? []).some((info) => info.state === "ready" || info.state === "waiting");
}

/** The model whose test notes explain a job's state: the one it runs on
 * or was picked for, else Automatic's (the recommendation), else the one
 * Bot uses, else Apple's, else the first listed. */
export function referenceModel(catalog: ModelsCatalog | null, jobs: readonly AiJobInfo[] | null, job?: ModelJob): CatalogModel | null {
  const models = catalog?.models ?? [];
  const byId = (id: string | null | undefined) => (id ? models.find((model) => model.id === id) : undefined);
  const own = jobs?.find((info) => info.job === job);
  const chat = jobs?.find((info) => info.job === "chat");
  return byId(own?.model_id) ?? byId(own?.pick) ?? byId(catalog?.recommendation?.model_id) ?? byId(chat?.model_id) ?? models.find((model) => model.built_in) ?? models[0] ?? null;
}

/** A job's row, worked out: its chip lit, its Beta tag and switch, its
 * picker, and its one line (D-149 #4: on, Beta, or off and why; never a
 * bare "not yet"). */
export interface JobRowView {
  /** The job runs (or is getting its model ready): the chip is lit. */
  on: boolean;
  /** A Beta job (some model passed it as Beta): its tag and switch. */
  beta: boolean;
  /** Its model picker (the routed jobs other than Bot's, which the card
   * above chooses). */
  picker: boolean;
  line: string;
}

/** The note a model's tests give for `job`, from the `models` section. */
function noteFor(models: readonly CatalogModel[], reference: CatalogModel | null, job: ModelJob, want: (verdict: string) => boolean): string | null {
  const own = reference ? verdictOf(reference, job) : null;
  if (own && want(own.verdict) && own.note) return own.note;
  for (const model of models) {
    const verdict = verdictOf(model, job);
    if (want(verdict.verdict) && verdict.note) return verdict.note;
  }
  return null;
}

const line = (state: string, why: string | null | undefined): string => (why ? `${state} · ${why}` : state);

/** Why a job with no engine here is off: its reason, else the tests'
 * note for it, else that it needs a model. */
function noEngineWhy(job: ModelJob, info: AiJobInfo, models: readonly CatalogModel[], reference: CatalogModel | null): string | null {
  const reason = info.why ? JOBS.why[info.why] : undefined;
  if (reason) return reason;
  const own = reference ? verdictOf(reference, job) : null;
  if (reference && own?.note && !passed(reference, job)) return own.note;
  if (models.some((known) => passed(known, job))) return JOBS.needsModel;
  return noteFor(models, reference, job, (verdict) => verdict !== "beta" && verdict !== "ships");
}

/** A running job's model in words: "Beta with …", "Beta, your pick: …". */
function runsWith(info: AiJobInfo, model: string | null): string | null {
  if (!model) return null;
  if (!info.beta) return JOBS.onShips(model);
  return info.pick ? JOBS.onPicked(model) : JOBS.onWith(model);
}

export function jobRowView(job: ModelJob, info: AiJobInfo | undefined, catalog: ModelsCatalog | null, installed: ModelsInfo | null, reference: CatalogModel | null): JobRowView {
  if (job === "attention") return { on: true, beta: false, picker: false, line: line(JOBS.on, JOBS.attention) };
  if (!info) return { on: false, beta: false, picker: false, line: "" };
  const models = catalog?.models ?? [];
  const beta = info.state !== "off" && models.some((model) => verdictOf(model, job).verdict === "beta");
  const picker = job !== "chat" && ["ready", "waiting", "beta_off"].includes(info.state) && pickable(catalog, installed, job).length > 0;
  const model = info.model_name ?? nameOf(info.pick ?? info.model_id, catalog, installed);
  const row = (on: boolean, state: string, why: string | null) => ({ on, beta, picker, line: line(state, why) });
  switch (info.state) {
    case "off":
      return row(false, JOBS.offLine, info.why === "not_built" ? JOBS.notBuilt : JOBS.paused);
    case "ready":
      return row(true, JOBS.on, runsWith(info, model));
    case "waiting":
      return row(true, JOBS.on, model ? JOBS.gettingReady(model) : null);
    case "beta_off":
      return row(false, JOBS.offLine, noteFor(models, reference, job, (verdict) => verdict === "beta"));
    default:
      return row(false, JOBS.offLine, noEngineWhy(job, info, models, reference));
  }
}

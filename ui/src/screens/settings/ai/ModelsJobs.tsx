/**
 * AJ-E3812 — Each job uses: one row per AI job, its Beta switch where it
 * has one, and its model through the router (AJ-E3802): Automatic first
 * (the recommendation), then only the models on this Mac that passed
 * that job (D-127, pickers suggested then all). A job no model passes
 * says "Not yet" with no picker; attention hints need no model at all —
 * the one place attention appears (the maintainer's fix 1). Settings ›
 * AI › Jobs shows the same rows on their own.
 */
import type { JSX } from "react";
import { Picker, type PickerOption } from "../../../components/controls";
import type { AiJobInfo } from "../../../lib/ipc/ai";
import type { ModelsCatalog, ModelsInfo } from "../../../lib/ipc/models";
import type { ModelJob } from "../../../types/generated/ModelJob";
import { Switch } from "../parts";
import { JobIcon } from "./model-icons";
import { CARD, ModelsSection } from "./ModelParts";
import { JOB_ROWS, nameOf, pickable, verdictOf } from "./models-view";
import { JOBS, JOB_NAMES, VERDICT_WORDS } from "./models-words";

const AUTOMATIC = "automatic";

export interface JobsActions {
  onBeta: (job: AiJobInfo["job"], on: boolean) => void;
  onPick: (job: AiJobInfo["job"], modelId: string | null) => void;
}

/** The row's line under its name. */
function lineOf(job: ModelJob, info: AiJobInfo | undefined, model: string | null): string {
  const what = JOBS.descriptions[job];
  if (job === "attention") return `${JOBS.on}. ${what}`;
  if (!info || info.state === "off") return what;
  if (!info.beta_on) return JOBS.off(what);
  if (!model) return `${JOBS.on} · ${JOBS.needsModel}`;
  return info.pick ? JOBS.onPicked(model) : JOBS.onWith(model);
}

function PlainEnd({ words }: { words: string }): JSX.Element {
  return <span className="justify-self-end pr-1 text-meta text-muted">{words}</span>;
}

function ModelPicker({
  job,
  info,
  catalog,
  installed,
  onPick,
}: {
  job: AiJobInfo["job"];
  info: AiJobInfo;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  onPick: JobsActions["onPick"];
}): JSX.Element {
  const models = pickable(catalog, installed, job);
  if (models.length === 0) return <PlainEnd words={JOBS.noModel} />;
  const automatic = nameOf(catalog?.recommendation?.model_id, catalog, installed);
  const options: PickerOption[] = [
    { value: AUTOMATIC, label: JOBS.automatic, hint: JOBS.automaticHint(automatic), pinned: true },
    ...models.map((model) => {
      const verdict = verdictOf(model, job);
      const gone = !model.offered && !model.built_in;
      return { value: model.id, label: model.name, hint: JOBS.pickHint(VERDICT_WORDS[verdict.verdict], gone ? JOBS.pickGone : verdict.note) };
    }),
  ];
  return (
    <Picker
      value={info.pick ?? AUTOMATIC}
      options={options}
      groups={[]}
      size="sm"
      heading={JOBS.pickerHeading(JOB_NAMES[job])}
      ariaLabel={JOBS.pickerLabel(JOB_NAMES[job])}
      onChange={(value) => onPick(job, value === AUTOMATIC ? null : value)}
      className="w-full max-w-[236px] justify-self-end"
      testId={`models-pick-${job}`}
    />
  );
}

function JobRow({
  job,
  info,
  catalog,
  installed,
  actions,
}: {
  job: ModelJob;
  info: AiJobInfo | undefined;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  actions: JobsActions;
}): JSX.Element {
  const routed = job !== "attention" && info && info.state !== "off";
  const hasBeta = routed && catalog?.models.some((model) => verdictOf(model, job).verdict === "beta");
  const model = info?.model_name ?? nameOf(info?.pick, catalog, installed);
  const titleId = `job-${job}`;
  let end: JSX.Element = <PlainEnd words={job === "attention" ? JOBS.noModelNeeded : JOBS.notYet} />;
  if (routed && info) end = <ModelPicker job={info.job} info={info} catalog={catalog} installed={installed} onPick={actions.onPick} />;
  return (
    <li className="grid grid-cols-[28px_minmax(0,1fr)_auto] items-center gap-x-3 gap-y-2 px-2.5 py-2.5 @min-[560px]:grid-cols-[28px_minmax(0,1fr)_44px_minmax(0,236px)]" data-testid={`models-job-${job}`}>
      <span aria-hidden="true" className="flex size-7 items-center justify-center rounded-lg bg-surface-3 text-ink-2">
        <JobIcon job={job} />
      </span>
      <span className="flex min-w-0 flex-col gap-px">
        <span id={titleId} className="flex items-center gap-1.5 text-body font-medium text-ink">
          {JOB_NAMES[job]}
          {hasBeta && <span className="rounded-full bg-accent-soft px-1.5 text-[11px] leading-[18px] font-medium text-accent-hi">{JOBS.beta}</span>}
        </span>
        <span className="text-[12.5px] text-muted">{lineOf(job, info, model)}</span>
      </span>
      <span className="flex justify-end">
        {hasBeta && info && <Switch checked={info.beta_on} onChange={(on) => actions.onBeta(info.job, on)} label={JOBS.switchLabel(JOB_NAMES[job])} testId={`models-beta-${job}`} />}
      </span>
      <span className="col-span-2 col-start-2 flex justify-end @min-[560px]:col-span-1 @min-[560px]:col-start-auto">{end}</span>
    </li>
  );
}

export function JobsTable({
  jobs,
  catalog,
  installed,
  actions,
  title = true,
}: {
  jobs: readonly AiJobInfo[] | null;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  actions: JobsActions;
  /** Its own heading (on Models; search lands on the Jobs pane's rows
   * instead); the Jobs pane has the page's. */
  title?: boolean;
}): JSX.Element {
  const rows = (
    <ul className={`@container flex flex-col divide-y divide-divider px-1.5 py-1 ${CARD}`} data-testid="models-jobs">
      {JOB_ROWS.map((job) => (
        <JobRow key={job} job={job} info={jobs?.find((info) => info.job === job)} catalog={catalog} installed={installed} actions={actions} />
      ))}
    </ul>
  );
  if (!title) return rows;
  return (
    <ModelsSection id="ai-jobs-table" title={JOBS.title} help={JOBS.help} searchable={false}>
      {rows}
    </ModelsSection>
  );
}

/**
 * AJ-E3812 — Each job uses: one row per AI job, its Beta switch where it
 * has one, and its model through the router (AJ-E3802): Automatic first
 * (the recommendation), then only the models on this Mac that passed
 * that job (D-127, pickers suggested then all).
 *
 * AJ-E3826 (D-149 #4): every job's row says its real state in one line:
 * on, Beta with its switch, or off and why (the `models` section's own
 * notes; "not built yet" for a job this version doesn't have), never a
 * bare "Not yet". Each row leads with the kit's icon chip, lit in the
 * accent's soft step while its job is on (TGL-2); rows are spaced on
 * tone, no rules (PAT-SURF); each model in a picker carries its icon
 * (PCK, M1). Bot's model is chosen in the Bot's AI card above, so its
 * row keeps only its switch.
 */
import type { JSX } from "react";
import { RowChip } from "../../../components/controls/ControlRow";
import { Picker, type PickerOption } from "../../../components/controls";
import type { AiJobInfo } from "../../../lib/ipc/ai";
import type { ModelsCatalog, ModelsInfo } from "../../../lib/ipc/models";
import type { ModelJob } from "../../../types/generated/ModelJob";
import { Switch } from "../parts";
import { IconAuto, jobIconOf, modelIcon } from "./model-icons";
import { CARD, ModelsSection } from "./ModelParts";
import { JOB_ROWS, jobRowView, nameOf, pickable, referenceModel, verdictOf } from "./models-view";
import { JOBS, JOB_NAMES, VERDICT_WORDS } from "./models-words";

const AUTOMATIC = "automatic";

export interface JobsActions {
  onBeta: (job: AiJobInfo["job"], on: boolean) => void;
  onPick: (job: AiJobInfo["job"], modelId: string | null) => void;
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
  const automatic = nameOf(catalog?.recommendation?.model_id, catalog, installed);
  const options: PickerOption[] = [
    { value: AUTOMATIC, label: JOBS.automatic, hint: JOBS.automaticHint(automatic), icon: IconAuto, pinned: true },
    ...models.map((model) => {
      const verdict = verdictOf(model, job);
      const gone = !model.offered && !model.built_in;
      return { value: model.id, label: model.name, icon: modelIcon(model), hint: JOBS.pickHint(VERDICT_WORDS[verdict.verdict], gone ? JOBS.pickGone : verdict.note) };
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
  jobs,
  actions,
}: {
  job: ModelJob;
  info: AiJobInfo | undefined;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  jobs: readonly AiJobInfo[] | null;
  actions: JobsActions;
}): JSX.Element {
  const view = jobRowView(job, info, catalog, installed, referenceModel(catalog, jobs, job));
  const titleId = `job-${job}`;
  return (
    <li
      className="grid grid-cols-[30px_minmax(0,1fr)_auto] items-center gap-x-3 gap-y-2 rounded-[10px] px-3 py-[9px] @min-[560px]:grid-cols-[30px_minmax(0,1fr)_44px_minmax(0,236px)]"
      data-testid={`models-job-${job}`}
      data-on={view.on || undefined}
    >
      <RowChip icon={jobIconOf(job)} on={view.on} />
      <span className="flex min-w-0 flex-col">
        <span id={titleId} className="flex items-center gap-1.5 text-body font-medium text-ink">
          {JOB_NAMES[job]}
          {view.beta && <span className="rounded-full bg-accent-soft px-1.5 text-label font-medium text-accent-hi">{JOBS.beta}</span>}
        </span>
        <span className="text-meta text-muted" data-testid={`models-job-line-${job}`}>
          {view.line}
        </span>
      </span>
      <span className="flex justify-end">
        {view.beta && info && <Switch checked={info.beta_on} onChange={(on) => actions.onBeta(info.job, on)} label={JOBS.switchLabel(JOB_NAMES[job])} testId={`models-beta-${job}`} />}
      </span>
      <span className="col-span-2 col-start-2 flex justify-end @min-[560px]:col-span-1 @min-[560px]:col-start-auto">
        {view.picker && info && <ModelPicker job={info.job} info={info} catalog={catalog} installed={installed} onPick={actions.onPick} />}
      </span>
    </li>
  );
}

export function JobsTable({
  jobs,
  catalog,
  installed,
  actions,
}: {
  jobs: readonly AiJobInfo[] | null;
  catalog: ModelsCatalog | null;
  installed: ModelsInfo | null;
  actions: JobsActions;
}): JSX.Element {
  return (
    <ModelsSection id="ai-jobs-table" title={JOBS.title} help={JOBS.help} searchable={false}>
      <ul className={`@container flex flex-col gap-0.5 p-1.5 ${CARD}`} data-testid="models-jobs">
        {JOB_ROWS.map((job) => (
          <JobRow key={job} job={job} info={jobs?.find((info) => info.job === job)} catalog={catalog} installed={installed} jobs={jobs} actions={actions} />
        ))}
      </ul>
    </ModelsSection>
  );
}

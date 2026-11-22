/**
 * AJ-E3812 — Settings › AI › Jobs: Each job uses on its own, under the
 * page's title (the mock's AI › Jobs sub-row). The same rows as on
 * Models, from the same reads: each job's Beta switch and its model.
 */
import type { JSX } from "react";
import { Alert } from "../../../components/Alert";
import { useAiJobs } from "../../../lib/ai-availability";
import { useModelsCatalog, useModelsInstalled } from "../../../lib/ai-models";
import { Pane } from "../parts";
import { useModelActions } from "./models-actions";
import { HonestyLine, ModelsSection } from "./ModelParts";
import { JobsTable } from "./ModelsJobs";
import { JOBS } from "./models-words";

export function JobsPane({ anchor }: { anchor?: string }): JSX.Element {
  const catalog = useModelsCatalog();
  const installed = useModelsInstalled();
  const jobs = useAiJobs();
  const act = useModelActions(jobs);
  return (
    <Pane id="ai-jobs" title={JOBS.paneTitle} anchor={anchor}>
      {act.note && (
        <Alert level="needs-you" testId="models-note">
          {act.note}
        </Alert>
      )}
      <ModelsSection id="ai-jobs" title={JOBS.title} help={JOBS.paneLead}>
        <JobsTable jobs={jobs} catalog={catalog} installed={installed} actions={{ onBeta: act.beta, onPick: act.pick }} title={false} />
        <HonestyLine />
      </ModelsSection>
    </Pane>
  );
}

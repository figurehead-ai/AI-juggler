/**
 * AJ-E1533, 0.1.5 — a project's Usage: the usage dashboard scoped to it
 * (its figures, over time by account, profile, tool or model, who did
 * the work, when, and its top sessions), from the same summaries the
 * Usage screen reads: AI Juggler's sessions and the ones it didn't start,
 * each message counted once.
 */
import { useEffect, useState, type JSX } from "react";
import { UsageDashboard } from "../components/usage/Dashboard";
import { Segmented } from "../components/usage/controls";
import { useRange } from "../components/usage/useUsage";
import { accountsList, type AccountInfo, type InstanceInfo } from "../lib/bridge";
import { RANGES } from "../lib/usage/range";

export function ProjectUsage({ projectId }: { projectId: string; sessions?: InstanceInfo[] }): JSX.Element {
  const [range, setRange] = useRange("project");
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  useEffect(() => {
    let alive = true;
    accountsList().then(
      (rows) => alive && setAccounts(rows),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, []);
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="project-usage">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-meta text-ink-2">Measured from each session's own files on this computer, each message counted once.</p>
        <Segmented label="Range" options={RANGES.map(({ id, label }) => ({ id, label }))} value={range} onChange={setRange} testId="project-usage-range" />
      </div>
      <UsageDashboard kind="project" scope={{ project: projectId }} range={range} accounts={accounts} testId="project-usage-dashboard" />
    </div>
  );
}

/**
 * AJ-E1533, 0.1.5 — a project's Usage: the usage dashboard scoped to it
 * (its figures, over time by account, profile, tool or model, who did
 * the work, when, and its top sessions), from the same summaries the
 * Usage screen reads: AI Juggler's sessions and the ones it didn't start,
 * each message counted once. With none at all yet, the tab stays and
 * says so in one line (AJ-E2805 R2), not with empty charts.
 */
import { useEffect, useState, type JSX } from "react";
import { UsageDashboard } from "../components/usage/Dashboard";
import { Segmented } from "../components/usage/controls";
import { useRange } from "../components/usage/useUsage";
import { accountsList } from "../lib/ipc/accounts";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { RANGES } from "../lib/usage/range";
import { useCached } from "../lib/usage/cached";
import { usageHistory, type UsageHistoryInfo } from "../lib/ipc/usageHistory";
import { outside, useConversations } from "../lib/conversations";

/** A project has usage to show when the usage history holds any of it,
 * at any time (0.1.5: its summaries, not the sessions the app holds
 * now); until that answer comes, what the working set says, so the tab
 * doesn't blink. */
function useProjectHasUsage(projectId: string, sessions: InstanceInfo[]): boolean {
  const { all: conversations } = useConversations();
  const used = (t: { input: number; output: number } | null | undefined) => !!t && t.input + t.output > 0;
  const meanwhile = sessions.some((s) => s.project_id === projectId && used(s.tokens)) || outside(conversations).some((c) => c.project_id === projectId && used(c.tokens));
  const result = useCached<UsageHistoryInfo>(
    `project-has-usage:${projectId}`,
    () => usageHistory({ edges_ms: [0, Date.now()], group_by: "profile", profile: null, project: projectId, top: 0 }),
    60_000,
  );
  const summary = result.current ? result.data : null;
  return summary ? summary.totals.messages > 0 || summary.session_count > 0 || meanwhile : meanwhile;
}

export function ProjectUsage({ projectId, sessions = [] }: { projectId: string; sessions?: InstanceInfo[] }): JSX.Element {
  const hasUsage = useProjectHasUsage(projectId, sessions);
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
  if (!hasUsage) {
    return (
      <div className="flex w-full max-w-content flex-col gap-grid" data-testid="project-usage">
        <p className="rounded-lg border border-dashed border-hairline px-3 py-4 text-meta text-ink-2" data-testid="project-usage-empty">
          No usage in this project yet. It shows here once a session in its folder has run.
        </p>
      </div>
    );
  }
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

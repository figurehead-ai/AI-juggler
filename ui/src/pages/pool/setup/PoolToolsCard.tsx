/**
 * 0.4.6 (AJ-E3011 §7, round 2 #1): the team tools (D-30's agent tools)
 * where a pool needs them — a compact card in the pool's inspector, and
 * Review's "Team tools" line. It says what they give, who they cover,
 * what they cost and what happens without them, with an example, then
 * that it's asked once and switched off per project or app in Settings ›
 * Agent access (a link to that place). "Turn on team tools" and, while
 * not asked yet, "Not now". Off for one of this pool's projects or Apps,
 * it says where, and Turn on switches those back on. Gone once on.
 */
import { useState, type JSX } from "react";

import { Button } from "../../../components/primitives";
import { errorText } from "../../../lib/bridge";
import { agentBridgeAnswer, setAgentBridge, useAgentBridge, type AgentBridgeInfo } from "../../../lib/ipc/agent-bridge";
import { AGENT_ACCESS, TEAM_TOOLS, openAgentAccess, teamTools, teamToolsLine, turnOnTeamTools, type TeamTools, type ToolsPlace } from "../../../lib/team-tools";

/** Saves a change to the tools and redraws every view that reads them. */
export function useToolsSave(): { busy: boolean; error: string | null; run: (work: () => Promise<AgentBridgeInfo>) => void } {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = (work: () => Promise<AgentBridgeInfo>) => {
    setBusy(true);
    setError(null);
    work().then(
      (next) => {
        setAgentBridge(next);
        setBusy(false);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  return { busy, error, run };
}

/** "Settings › Agent access" as a link to that place. */
export function AgentAccessLink(): JSX.Element {
  return (
    <button type="button" onClick={openAgentAccess} className="rounded font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="team-tools-settings">
      {AGENT_ACCESS}
    </button>
  );
}

/** Where this pool's tools stand, from its projects and Apps. */
export function useTeamTools(projects: readonly ToolsPlace[], apps: readonly ToolsPlace[]): TeamTools | null {
  const info = useAgentBridge();
  return info === null ? null : teamTools(info, projects, apps);
}

export function PoolToolsCard({ testId = "pool-tools-card", projects = [], apps = [] }: { testId?: string; projects?: readonly ToolsPlace[]; apps?: readonly ToolsPlace[] }): JSX.Element | null {
  const state = useTeamTools(projects, apps);
  const { busy, error, run } = useToolsSave();
  if (state === null || state.kind === "on") return null;
  return (
    <section aria-labelledby={`${testId}-title`} className="flex flex-col gap-1.5 rounded-xl bg-surface-2 px-3.5 py-3" data-testid={testId} data-state={state.kind}>
      <h4 id={`${testId}-title`} className="text-body font-semibold text-ink">
        {TEAM_TOOLS.title}
      </h4>
      {state.kind !== "ask" && (
        <p className="text-meta font-medium text-ink" data-testid={`${testId}-state`}>
          {teamToolsLine(state)}
        </p>
      )}
      <p className="text-meta text-ink-2">
        {TEAM_TOOLS.gives} {TEAM_TOOLS.example}
      </p>
      <ul className="flex flex-col gap-0.5 text-meta text-ink-2" data-testid={`${testId}-facts`}>
        <li>{TEAM_TOOLS.who}</li>
        <li>{TEAM_TOOLS.cost}</li>
        <li className="text-ink">{TEAM_TOOLS.without}</li>
      </ul>
      <p className="text-meta text-muted">
        {TEAM_TOOLS.where} <AgentAccessLink />.
      </p>
      {state.kind !== "missing" && (
        <span className="mt-1 flex flex-wrap gap-2">
          <Button size="sm" variant="accent" disabled={busy} onClick={() => run(() => turnOnTeamTools(state))} data-testid={`${testId}-on`}>
            {TEAM_TOOLS.button}
          </Button>
          {state.kind === "ask" && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => run(() => agentBridgeAnswer(false))} data-testid={`${testId}-not-now`}>
              Not now
            </Button>
          )}
        </span>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

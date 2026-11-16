/**
 * AJ-E3011 round 4 (main's ruling), D-30: the team tools are a question,
 * asked once, the first time pools are used — one component for the pool
 * page (at its top), New pool's own step and Settings › Agent access.
 *
 * - Unanswered: the question, what it gives, what it costs (nothing
 *   extra), where to switch it off, then "Yes, let them work together"
 *   and "Not now".
 * - Answered Yes: nothing (Settings keeps the switches).
 * - Answered Not now, or off for this pool's project or App: one quiet
 *   line with Turn on beside it ({@link TeamToolsFix}); nowhere says
 *   "can't message" without that one-click fix.
 */
import { useState, type JSX } from "react";

import { Button } from "../../../components/primitives";
import { errorText } from "../../../lib/bridge";
import { agentBridgeAnswer, setAgentBridge, useAgentBridge, type AgentBridgeInfo } from "../../../lib/ipc/agent-bridge";
import { AGENT_ACCESS, TEAM_TOOLS, needsTurnOn, openAgentAccess, teamTools, teamToolsLine, turnOnTeamTools, type TeamTools, type ToolsPlace } from "../../../lib/team-tools";

/** Saves an answer or a switch and redraws every view that reads it. */
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

/** Where this pool's tools stand, from its projects and Apps. */
export function useTeamTools(projects: readonly ToolsPlace[] = [], apps: readonly ToolsPlace[] = []): TeamTools | null {
  const info = useAgentBridge();
  return info === null ? null : teamTools(info, projects, apps);
}

function Failed({ error }: { error: string | null }): JSX.Element | null {
  if (!error) return null;
  return (
    <p className="text-meta text-danger" role="alert">
      {error}
    </p>
  );
}

/** The line, and Turn on beside it when they can't message. */
export function TeamToolsFix({ state, testId }: { state: TeamTools; testId: string }): JSX.Element {
  const { busy, error, run } = useToolsSave();
  return (
    <span className="flex flex-col gap-1">
      <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <span data-testid={`${testId}-line`}>{teamToolsLine(state)}</span>
        {needsTurnOn(state) && (
          <Button size="sm" variant="accent" disabled={busy} onClick={() => run(() => turnOnTeamTools(state))} data-testid={`${testId}-on`}>
            {TEAM_TOOLS.turnOn}
          </Button>
        )}
      </span>
      <Failed error={error} />
    </span>
  );
}

/** D-30's question while unanswered; once answered Not now (or off for
 * this pool), its quiet line with Turn on; nothing once on. */
export function TeamToolsQuestion({
  projects = [],
  apps = [],
  onAnswer,
  heading = 3,
  testId = "team-tools",
}: {
  /** Its title's level: 4 inside a section that has its own (Settings). */
  heading?: 3 | 4;
  projects?: readonly ToolsPlace[];
  apps?: readonly ToolsPlace[];
  /** Answered here (Yes or Not now). */
  onAnswer?: () => void;
  testId?: string;
}): JSX.Element | null {
  const state = useTeamTools(projects, apps);
  const { busy, error, run } = useToolsSave();
  const Title = heading === 4 ? "h4" : "h3";
  if (state === null || state.kind === "on" || state.kind === "missing") return null;
  if (state.kind !== "ask")
    return (
      <div className="flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-2 text-meta text-ink-2" data-testid={`${testId}-quiet`}>
        <TeamToolsFix state={state} testId={testId} />
      </div>
    );
  return (
    <section aria-labelledby={`${testId}-title`} className="flex flex-col gap-2 rounded-xl bg-surface-2 px-4 py-3.5 shadow-card" data-testid={testId}>
      <Title id={`${testId}-title`} className="text-body font-semibold text-ink">
        {TEAM_TOOLS.question}
      </Title>
      <p className="text-meta text-ink-2">{TEAM_TOOLS.gives}</p>
      <p className="text-meta text-ink-2">{TEAM_TOOLS.cost}</p>
      <p className="text-meta text-muted">
        {TEAM_TOOLS.where}{" "}
        <button type="button" onClick={openAgentAccess} className="rounded font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid={`${testId}-settings`}>
          {AGENT_ACCESS}
        </button>
        .
      </p>
      <span className="mt-1 flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="primary"
          disabled={busy}
          onClick={() => {
            onAnswer?.();
            run(() => agentBridgeAnswer(true));
          }}
          data-testid={`${testId}-yes`}
        >
          {TEAM_TOOLS.yes}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          disabled={busy}
          onClick={() => {
            onAnswer?.();
            run(() => agentBridgeAnswer(false));
          }}
          data-testid={`${testId}-not-now`}
        >
          {TEAM_TOOLS.notNow}
        </Button>
      </span>
      <Failed error={error} />
    </section>
  );
}

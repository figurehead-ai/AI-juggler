/**
 * 0.4.6 (D-126 §3): D-30's team tools question as the setup checklist's
 * last item, "Let them message each other", where the banner over the
 * pool page was (it pushed the List view down by about 15 rows, X2 §4c).
 * Open, it says what they give with Learn more, and its actions:
 *
 * - not asked yet: Turn on team tools (the view's one primary) and Not now;
 * - answered Not now, or off for this pool's project or App: Turn on team
 *   tools, with the quiet line saying where they're off;
 * - unable to run from AI Juggler's folder: why, and what to do; no
 *   button (the checklist says the pool can't work yet).
 *
 * Done ("On"), its Change opens Settings › Agent access. Not in this
 * build at all: no item.
 */
import type { JSX } from "react";

import { Button } from "../../../components/primitives";
import { trustOpen } from "../../../lib/ipc/trust";
import { agentBridgeAnswer } from "../../../lib/ipc/agent-bridge";
import { TEAM_TOOLS, teamToolsLine, turnOnTeamTools, type TeamTools } from "../../../lib/team-tools";
import { TOOLS_ITEM } from "../setup-words";
import { useToolsSave } from "./TeamTools";

/** Whether the item is on the checklist (a build with team tools, a pool
 * that isn't sealed). */
export const toolsItemShown = (state: TeamTools | null, sealed: boolean): boolean => !sealed && state !== null && state.kind !== "missing";

/** The open item's words and actions. */
export function ToolsItemBody({ state, primary = true, testId }: { state: TeamTools; primary?: boolean; testId: string }): JSX.Element {
  const { busy, error, run } = useToolsSave();
  const unsafe = state.kind === "unsafe";
  return (
    <div className="flex flex-col gap-2.5" data-testid={testId}>
      <p className="text-meta text-ink-2">
        {TOOLS_ITEM.body}{" "}
        <button type="button" onClick={() => void trustOpen({ kind: "guide", id: TOOLS_ITEM.guide })} className="rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid={`${testId}-learn`}>
          {TOOLS_ITEM.learn}
        </button>
      </p>
      {state.kind !== "ask" && (
        <p className={"text-meta " + (unsafe ? "text-ink" : "text-muted")} data-testid={`${testId}-line`}>
          {teamToolsLine(state)}
        </p>
      )}
      {!unsafe && (
        <span className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant={primary ? "primary" : "secondary"} disabled={busy} onClick={() => run(() => (state.kind === "ask" ? agentBridgeAnswer(true) : turnOnTeamTools(state)))} data-testid={`${testId}-on`}>
            {TOOLS_ITEM.turnOn}
          </Button>
          {state.kind === "ask" && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => run(() => agentBridgeAnswer(false))} data-testid={`${testId}-not-now`}>
              {TEAM_TOOLS.notNow}
            </Button>
          )}
        </span>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

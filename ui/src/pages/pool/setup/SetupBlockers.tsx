/**
 * Round 6 (Casey's run: Review and the checklist said all was set while
 * nothing could work): what keeps a draft pool from working, said on
 * Review before it's made — the team tools that can't run here, and each
 * desktop App whose connector isn't in it (or can't go in), with where
 * it's fixed. The diagram's checklist says the same (`setupBlockers`).
 */
import type { JSX } from "react";

import { useApps, type AppModel } from "../../../lib/apps";
import { desktopAppsOf, isAdded, unavailableWords, useConnectorStatus } from "../../../lib/connector";
import { useAgentBridge } from "../../../lib/ipc/agent-bridge";
import { teamToolsLine, type TeamTools } from "../../../lib/team-tools";
import { lowerFirst } from "../../../lib/pool-setup";

/** One desktop App's line, when its connector isn't usable yet. */
function AppBlocker({ app }: { app: AppModel }): JSX.Element | null {
  const status = useConnectorStatus(app.id);
  const bridge = useAgentBridge();
  if (!status || isAdded(status.state)) return null;
  const why =
    status.state === "unavailable"
      ? unavailableWords(status, bridge)
      : status.state === "changed"
        ? "its AI Juggler connector was changed: fix it in Who's in."
        : "it needs AI Juggler's connector first: add it in Who's in.";
  return (
    <li className="text-meta text-warning" data-testid="setup-blocker">
      {app.name} can't talk here yet: {lowerFirst(why)}
    </li>
  );
}

export function SetupBlockers({ refs, tools, testId = "setup-blockers" }: { refs: readonly string[]; tools: TeamTools | null; testId?: string }): JSX.Element | null {
  const apps = useApps();
  const desktop = desktopAppsOf(refs, apps ?? []);
  const toolsDown = tools && (tools.kind === "missing" || tools.kind === "unsafe");
  if (!toolsDown && desktop.length === 0) return null;
  return (
    <ul className="flex flex-col gap-1 empty:hidden" data-testid={testId}>
      {toolsDown && (
        <li className="text-meta text-warning" data-testid="setup-blocker">
          The team tools can't run, so no one here can message: {lowerFirst(teamToolsLine(tools))}
        </li>
      )}
      {desktop.map((app) => (
        <AppBlocker key={app.id} app={app} />
      ))}
    </ul>
  );
}

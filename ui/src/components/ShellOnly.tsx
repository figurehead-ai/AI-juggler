/**
 * D-62 (the P8 walk): a pool member whose tool has no agent tools yet
 * works through its shell only. It can't read the pool, post or take a
 * task, so it says so wherever a member is shown: its card on the canvas,
 * its row in the List view, its inspector. So does one whose App can't
 * take them (AJ-E3002 R3), with its adapter's reason, and one in a
 * project or App where the user switched them off (Settings › Agent
 * access › "Where they're off"; the rc18 walk, E3002-R3 step 12).
 */
import type { JSX } from "react";

import { useAgentToolsWhy } from "../lib/agent-tools";
import { useAgentBridge } from "../lib/ipc/agent-bridge";
import { useTools } from "../lib/tools";

const SHELL_ONLY_WHY = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";
const NO_POOL = "it works through its shell, and can't read the pool, post or take a task.";
const OFF_EVERYWHERE = `The agent tools are off (Settings › Agent access): ${NO_POOL}`;
const OFF_FOR_PROJECT = `The agent tools are off for its project (Settings › Agent access): ${NO_POOL}`;
const OFF_FOR_APP = `The agent tools are off for its App (Settings › Agent access): ${NO_POOL}`;

/** Why a member works through its shell only, or `null` when it gets
 * the agent tools: its tool has none (a tool not described yet: nothing
 * claimed either way), its App's launches can't take them (its
 * adapter's words, AJ-E3002 R3), or the user switched them off, for
 * every session or its project or App (as the launch decides; nothing
 * claimed while that loads or before the question is answered). */
export function useShellOnly(tool: string, account?: string | null, project?: string | null): string | null {
  const spec = useTools().find((one) => one.id === tool);
  const appWhy = useAgentToolsWhy(account);
  const bridge = useAgentBridge();
  if (!spec) return null;
  if (!spec.supports.agent_tools) return SHELL_ONLY_WHY;
  if (appWhy) return appWhy;
  if (bridge?.asked && !bridge.on) return OFF_EVERYWHERE;
  if (bridge?.on && project && bridge.off_projects.includes(project)) return OFF_FOR_PROJECT;
  if (bridge?.on && account && bridge.off_profiles.includes(account)) return OFF_FOR_APP;
  return null;
}

export function ShellOnly({ tool, account, project }: { tool: string; account?: string | null; project?: string | null }): JSX.Element | null {
  const why = useShellOnly(tool, account, project);
  if (!why) return null;
  return (
    <span className="shrink-0 rounded-md bg-surface-2 px-1.5 text-label text-ink-2" title={why} data-testid="shell-only">
      Shell only
    </span>
  );
}

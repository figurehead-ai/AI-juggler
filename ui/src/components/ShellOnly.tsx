/**
 * D-62 (the P8 walk): a pool member whose tool has no agent tools yet
 * works through its shell only. It can't read the pool, post or take a
 * task, so it says so wherever a member is shown: its card on the canvas,
 * its row in the List view, its inspector. So does one whose App can't
 * take them (AJ-E3002 R3), with its adapter's reason.
 */
import type { JSX } from "react";

import { useAgentToolsWhy } from "../lib/agent-tools";
import { useTools } from "../lib/tools";

export const SHELL_ONLY_WHY = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";

/** Why a member works through its shell only, or `null` when it gets
 * the agent tools: its tool has none (a tool not described yet: nothing
 * claimed either way), or its App's launches can't take them (its
 * adapter's words, AJ-E3002 R3). */
export function useShellOnly(tool: string, account?: string | null): string | null {
  const spec = useTools().find((one) => one.id === tool);
  const appWhy = useAgentToolsWhy(account);
  if (!spec) return null;
  if (!spec.supports.agent_tools) return SHELL_ONLY_WHY;
  return appWhy;
}

export function ShellOnly({ tool, account }: { tool: string; account?: string | null }): JSX.Element | null {
  const why = useShellOnly(tool, account);
  if (!why) return null;
  return (
    <span className="shrink-0 rounded-md bg-surface-2 px-1.5 text-label text-ink-2" title={why} data-testid="shell-only">
      Shell only
    </span>
  );
}

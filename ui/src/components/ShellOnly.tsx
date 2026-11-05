/**
 * D-62 (the P8 walk): a pool member whose tool has no agent tools yet
 * works through its shell only. It can't read the pool, post or take a
 * task, so it says so wherever a member is shown: its card on the canvas,
 * its row in the List view, its inspector.
 */
import type { JSX } from "react";

import { useTools } from "../lib/tools";

export const SHELL_ONLY_WHY = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";

/** Its tool is described and has no agent tools (a tool not described
 * yet: nothing claimed either way). */
export function useShellOnly(tool: string): boolean {
  const spec = useTools().find((one) => one.id === tool);
  return !!spec && !spec.supports.agent_tools;
}

export function ShellOnly({ tool }: { tool: string }): JSX.Element | null {
  if (!useShellOnly(tool)) return null;
  return (
    <span className="shrink-0 rounded-md bg-surface-2 px-1.5 text-label text-ink-2" title={SHELL_ONLY_WHY} data-testid="shell-only">
      Shell only
    </span>
  );
}

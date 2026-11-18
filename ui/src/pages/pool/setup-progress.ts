/**
 * 0.4.6 (D-126 §3): how far a pool's setup is, read once for its page —
 * the header's "Setting up · 5 of 7", the inspector's ring and checklist —
 * and which way the page shows it:
 *
 * - **setting up**: the checklist, the planned team drawn, the accent;
 * - **running**: the calm live view, once a session has joined and the
 *   setup is done, or as soon as the user picks Hide setup (kept per pool
 *   on this Mac; Show setup brings it back).
 *
 * Done means every step (a default counts), the team tools on where
 * they're an item (a build that has them, a pool that isn't sealed), and
 * nothing keeping the pool from working (round 6's blockers).
 */
import { useCallback, useState } from "react";

import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { reachOf, useDesktopMembers } from "../../lib/connector";
import { setupBlockers, type SetupRow } from "../../lib/pool-setup";
import type { PoolRules } from "../../lib/pool-rules";
import type { TeamTools, ToolsPlace } from "../../lib/team-tools";
import { useTeamTools } from "./setup/TeamTools";
import { toolsItemShown } from "./setup/ToolsItem";

export type SetupProgress = {
  /** Where the team tools stand (null until read). */
  tools: TeamTools | null;
  /** They're the checklist's last item. */
  toolsItem: boolean;
  blockers: string[];
  done: number;
  total: number;
  /** Every step done, the tools on, nothing blocking. */
  complete: boolean;
};

export function useSetupProgress(input: { pool: Pick<PoolInfo, "id" | "sealed">; rows: readonly SetupRow[]; places: { projects: readonly ToolsPlace[]; apps: readonly ToolsPlace[] }; rules: PoolRules; ruleLabel: (rule: PoolRuleInfo) => string }): SetupProgress {
  const tools = useTeamTools(input.places.projects, input.places.apps);
  const desktop = useDesktopMembers(input.pool.id);
  // Round 6: what can't work is said, and the pool is never "done" then.
  const names = new Map<string, string>([...(input.rules.rules?.members ?? []).map((rule) => [rule.member, input.ruleLabel(rule)] as const), ...input.rules.watched.map((one) => [one.member, one.title ?? "A session in another app"] as const)]);
  const blockers = setupBlockers(tools, [...(desktop?.values() ?? [])].map((one) => ({ name: names.get(one.member) ?? "A desktop member", reach: reachOf(one) })));
  const toolsItem = toolsItemShown(tools, input.pool.sealed);
  const toolsOn = tools?.kind === "on";
  const done = input.rows.filter((row) => row.done).length + (toolsItem && toolsOn ? 1 : 0);
  const total = input.rows.length + (toolsItem ? 1 : 0);
  return { tools, toolsItem, blockers, done, total, complete: done === total && blockers.length === 0 && (toolsOn || !toolsItem) && tools !== null };
}

export type SetupPhase = "setup" | "running";

/** Setting up until a session has joined and the setup is done, or the
 * user hid it. */
export function setupPhase({ joined, complete, hidden }: { joined: boolean; complete: boolean; hidden: boolean }): SetupPhase {
  return hidden || (joined && complete) ? "running" : "setup";
}

const HIDDEN = (poolId: string) => `aij.pool.setup-hidden.${poolId}`;

/** Hide setup, kept per pool on this Mac (a viewer's choice, never the
 * pool's state). */
export function useSetupHidden(poolId: string): [boolean, (hidden: boolean) => void] {
  const [hidden, setState] = useState(() => {
    try {
      return localStorage.getItem(HIDDEN(poolId)) === "1";
    } catch {
      return false;
    }
  });
  const set = useCallback(
    (next: boolean) => {
      try {
        if (next) localStorage.setItem(HIDDEN(poolId), "1");
        else localStorage.removeItem(HIDDEN(poolId));
      } catch {
        // Storage unavailable: this visit only.
      }
      setState(next);
    },
    [poolId],
  );
  return [hidden, set];
}

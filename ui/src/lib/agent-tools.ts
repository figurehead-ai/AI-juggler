/**
 * AJ-E3002 R3: whether each App's sessions get AI Juggler's agent tools,
 * as its adapter answers for its folder (`AccountInfo.agent_tools_why`):
 * a tool that can take them may still not for one App (its home isn't
 * one AI Juggler made). Held once for the whole UI, from the accounts the
 * shell already loads; no tool is named here.
 */
import { useSyncExternalStore } from "react";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { AgentBridgeInfo } from "../types/generated/AgentBridgeInfo";

let why = new Map<string, string>();
const listeners = new Set<() => void>();

/** Takes each App's answer from a fresh accounts list. */
export function publishAgentTools(accounts: readonly Pick<AccountInfo, "id" | "agent_tools_why">[]): void {
  const next = new Map<string, string>();
  for (const account of accounts) {
    if (account.agent_tools_why) next.set(account.id, account.agent_tools_why);
  }
  why = next;
  for (const listener of listeners) listener();
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
const snapshot = () => why;

/** Why `account`'s sessions get no agent tools; `null` when they do or
 * nothing is known (outside React). */
export function agentToolsWhy(account: string | null | undefined): string | null {
  return account ? (why.get(account) ?? null) : null;
}

const NO_POOL = "it works through its shell, and can't read the pool, post or take a task.";
const OFF_EVERYWHERE = `The agent tools are off (Settings › Agent access): ${NO_POOL}`;
const OFF_FOR_PROJECT = `The agent tools are off for its project (Settings › Agent access): ${NO_POOL}`;
const OFF_FOR_APP = `The agent tools are off for its App (Settings › Agent access): ${NO_POOL}`;

/** The rc18 walk (E3002-R3 step 12): why a session of `account` in
 * `project` gets no agent tools because the user switched them off, for
 * every session, its project or its App, as the launch decides (the
 * backend's `settings::allows`); `null` when the switches let it, while
 * they load, or before the question is answered. No tool is named. */
export function switchedOffWhy(
  bridge: Pick<AgentBridgeInfo, "asked" | "on" | "off_projects" | "off_profiles"> | null | undefined,
  account: string | null | undefined,
  project: string | null | undefined,
): string | null {
  if (!bridge?.asked) return null;
  if (!bridge.on) return OFF_EVERYWHERE;
  if (project && bridge.off_projects.includes(project)) return OFF_FOR_PROJECT;
  if (account && bridge.off_profiles.includes(account)) return OFF_FOR_APP;
  return null;
}

/** A tool with no per-launch agent tools: why its sessions work through
 * their shell. */
const SHELL_ONLY_WHY = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";

/** Why sessions of `spec`'s tool, of App `account` in `project`, work
 * through their shell only, or `null` when they get the agent tools: the
 * tool has none (a tool not described yet: nothing claimed either way),
 * the App's launches can't take them (`appWhy`, its adapter's words,
 * AJ-E3002 R3), or the user switched them off. The one answer for a
 * session's node and an App or Tool member's (the P8 walk). */
export function shellOnlyWhy(
  spec: { supports: { agent_tools: boolean } } | undefined,
  appWhy: string | null,
  bridge: Parameters<typeof switchedOffWhy>[0],
  account: string | null | undefined,
  project: string | null | undefined,
): string | null {
  if (!spec) return null;
  if (!spec.supports.agent_tools) return SHELL_ONLY_WHY;
  return appWhy ?? switchedOffWhy(bridge, account, project);
}

/** [`agentToolsWhy`], re-rendering when it changes. */
export function useAgentToolsWhy(account: string | null | undefined): string | null {
  const map = useSyncExternalStore(subscribe, snapshot, snapshot);
  return account ? (map.get(account) ?? null) : null;
}

/**
 * AJ-E3002 R3: whether each App's sessions get AI Juggler's agent tools,
 * as its adapter answers for its folder (`AccountInfo.agent_tools_why`):
 * a tool that can take them may still not for one App (its home isn't
 * one AI Juggler made). Held once for the whole UI, from the accounts the
 * shell already loads; no tool is named here.
 */
import { useSyncExternalStore } from "react";
import type { AccountInfo } from "../types/generated/AccountInfo";

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

/** [`agentToolsWhy`], re-rendering when it changes. */
export function useAgentToolsWhy(account: string | null | undefined): string | null {
  const map = useSyncExternalStore(subscribe, snapshot, snapshot);
  return account ? (map.get(account) ?? null) : null;
}

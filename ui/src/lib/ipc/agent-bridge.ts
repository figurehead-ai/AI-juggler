/**
 * AJ-E1704, D-30 — whether sessions AI Juggler starts get its agent
 * tools: the one-time question's answer, and the switches per project
 * and profile (ENGINEERING-GUIDE U3: one IPC module per feature). Only
 * the app's own window calls these; an agent can't.
 *
 * The preview keeps its own answer, seeded by `__AIJ_AGENT_BRIDGE__`
 * (part of an {@link AgentBridgeInfo}; not asked yet by default), and
 * records the calls that change it in `__AIJ_AGENT_BRIDGE_CALLS__`.
 */
import { useSyncExternalStore } from "react";
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { AgentBridgeInfo } from "../../types/generated/AgentBridgeInfo";

export type { AgentBridgeInfo };
export type AgentToolsScope = "project" | "profile";

interface Seams {
  __AIJ_AGENT_BRIDGE__?: Partial<AgentBridgeInfo>;
  __AIJ_AGENT_BRIDGE_CALLS__?: string[];
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_AGENT_BRIDGE_CALLS__ ??= []).push(call);

let store: AgentBridgeInfo | null = null;

function preview(): AgentBridgeInfo {
  store ??= { asked: false, on: false, available: true, off_projects: [], off_profiles: [], ...seams().__AIJ_AGENT_BRIDGE__ };
  return { ...store, off_projects: [...store.off_projects], off_profiles: [...store.off_profiles] };
}

/** Tests: the preview starts over from its seed. */
export function resetPreviewAgentBridge(): void {
  store = null;
  current = null;
}

/** Whether sessions get the agent tools, and where they're off. */
export async function agentBridgeGet(): Promise<AgentBridgeInfo> {
  if (!hasLiveBackend()) return preview();
  return invoke<AgentBridgeInfo>("agent_bridge_get");
}

/** The answer to the one-time question, or the switch for every session. */
export async function agentBridgeAnswer(on: boolean): Promise<AgentBridgeInfo> {
  if (!hasLiveBackend()) {
    preview();
    record(`answer:${on}`);
    store = { ...store!, asked: true, on };
    return preview();
  }
  return invoke<AgentBridgeInfo>("agent_bridge_answer", { on });
}

/** Switches the tools off (or back on) for one project or profile. */
export async function agentBridgeSet(scope: AgentToolsScope, id: string, on: boolean): Promise<AgentBridgeInfo> {
  if (!hasLiveBackend()) {
    preview();
    record(`set:${scope}:${id}:${on}`);
    const key = scope === "project" ? "off_projects" : "off_profiles";
    const off = new Set(store![key]);
    if (on) off.delete(id);
    else off.add(id);
    store = { ...store!, [key]: [...off].sort() };
    return preview();
  }
  return invoke<AgentBridgeInfo>("agent_bridge_set", { scope, id, on });
}

// The one copy every view reads (the pool page's question, Settings).
let current: AgentBridgeInfo | null = null;
let reading: Promise<void> | null = null;
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

/** Keeps what the backend answered and redraws every view. */
export function setAgentBridge(info: AgentBridgeInfo): void {
  current = info;
  for (const listener of listeners) listener();
}

/** The answer, read once on first use (lazily: nothing reads it until a
 * pool's page or Settings › Agent access shows). `null` while it loads. */
export function useAgentBridge(): AgentBridgeInfo | null {
  const info = useSyncExternalStore(subscribe, () => current);
  if (current === null && reading === null) {
    reading = agentBridgeGet().then(setAgentBridge, () => undefined).finally(() => {
      reading = null;
    });
  }
  return info;
}

/**
 * AJ-E1520 (0.3) — the links between a session's Subagents, Usage and
 * Processes tabs: the Usage tab's "see which ›" opens the Subagents tab
 * sorted by most tokens; a Processes row's "started by subagent …"
 * opens that subagent's page; a subagent's process opens the Processes
 * tab with it selected. Each says what to show and picks the tab; the
 * tab takes the ask when it mounts, or hears it when already shown.
 */
import { showTab } from "../pages/PageShell";

export type SubagentsAsk = { sort?: "tokens" | "newest" | "longest"; subagent?: string };

const SUBAGENTS = "aij:subagents-ask";
const PROCESS = "aij:process-select";

let pendingSubagents: SubagentsAsk | null = null;
let pendingProcess: string | null = null;

/** Opens the session's Subagents tab as asked. */
export function openSubagents(ask: SubagentsAsk): void {
  pendingSubagents = ask;
  showTab("subagents");
  window.dispatchEvent(new CustomEvent(SUBAGENTS));
}

/** The ask waiting for the Subagents tab (taken once). */
export function takeSubagentsAsk(): SubagentsAsk | null {
  const ask = pendingSubagents;
  pendingSubagents = null;
  return ask;
}

/** Hears asks while the tab is shown. */
export function onSubagentsAsk(listener: () => void): () => void {
  window.addEventListener(SUBAGENTS, listener);
  return () => window.removeEventListener(SUBAGENTS, listener);
}

/** Opens the session's Processes tab with one process (its row key)
 * selected. */
export function showProcess(key: string): void {
  pendingProcess = key;
  showTab("processes");
  window.dispatchEvent(new CustomEvent(PROCESS));
}

/** The process waiting to be selected (taken once it is shown). */
export function peekProcess(): string | null {
  return pendingProcess;
}

export function clearProcess(): void {
  pendingProcess = null;
}

export function onProcessAsk(listener: () => void): () => void {
  window.addEventListener(PROCESS, listener);
  return () => window.removeEventListener(PROCESS, listener);
}

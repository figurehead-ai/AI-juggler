/**
 * AJ-E1520 — the subagents a session started (ENGINEERING-GUIDE U3: the
 * feature's calls). The backend reads them from the session's transcript
 * with the tool's own parser; the status row carries how many still run.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { SubtasksInfo } from "../../types/generated/SubtasksInfo";

export type { SubtasksInfo };
export type { SubtaskInfo } from "../../types/generated/SubtaskInfo";
export type { SubtaskStateInfo } from "../../types/generated/SubtaskStateInfo";

/** Every subagent a session started, in order. The preview has none
 * unless a spec seeds `subtasks` by session id. */
export async function subtasksList(sessionId: string): Promise<SubtasksInfo> {
  if (!hasLiveBackend()) {
    const fixture = (window as unknown as { __AIJ_FIXTURE__?: { subtasks?: Record<string, SubtasksInfo> } }).__AIJ_FIXTURE__;
    return fixture?.subtasks?.[sessionId] ?? { items: [], live: true };
  }
  return invoke<SubtasksInfo>("subtasks_list", { sessionId });
}

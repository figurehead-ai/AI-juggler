/**
 * ADR-0034 §3.4 (AJ-E3802) — the AI jobs, as the window reads them: which
 * jobs are available now (`ai_jobs`), each with the model chosen and why,
 * and a job's Beta switch (`ai_beta_set`). The backend says when the
 * answer changed (`ai://jobs-changed`).
 *
 * The preview and tests set `window.__AIJ_AI_JOBS__`; with none, every
 * job is absent, as on a Mac with no engine.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import { listenWhileShown } from "../shown-window";
import type { AiJob } from "../../types/generated/AiJob";
import type { AiJobInfo } from "../../types/generated/AiJobInfo";
import type { AiJobState } from "../../types/generated/AiJobState";
import type { AiJobWhy } from "../../types/generated/AiJobWhy";

export type { AiJob, AiJobInfo, AiJobState, AiJobWhy };

/** The event the backend sends when the answer changed. */
export const AI_JOBS_CHANGED = "ai://jobs-changed";

declare global {
  interface Window {
    __AIJ_AI_JOBS__?: AiJobInfo[];
  }
}

/** Every routed job's answer now. */
export async function aiJobs(): Promise<AiJobInfo[]> {
  if (!hasLiveBackend()) return [...(window.__AIJ_AI_JOBS__ ?? [])];
  return call("ai_jobs");
}

/** Turns `job`'s Beta on or off; every job's answer after it. */
export async function aiBetaSet(job: AiJob, on: boolean): Promise<AiJobInfo[]> {
  if (!hasLiveBackend()) {
    const jobs = (window.__AIJ_AI_JOBS__ ?? []).map((info) => (info.job === job ? { ...info, beta_on: on } : info));
    window.__AIJ_AI_JOBS__ = jobs;
    window.dispatchEvent(new Event(AI_JOBS_CHANGED));
    return [...jobs];
  }
  return call("ai_beta_set", { job, on });
}

/** Calls `on` when the answer changed, and once when the window is shown
 * again (the backend then asks the engines again if its last look is
 * stale: Apple Intelligence turned on or off meanwhile). Returns the stop. */
export function onAiJobsChanged(on: () => void): () => void {
  const heard = () => on();
  window.addEventListener(AI_JOBS_CHANGED, heard);
  const stop = hasLiveBackend() ? listenWhileShown<null>(AI_JOBS_CHANGED, heard, heard) : null;
  return () => {
    window.removeEventListener(AI_JOBS_CHANGED, heard);
    stop?.();
  };
}

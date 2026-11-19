/**
 * ADR-0034 §3.4 (AJ-E3802) — which AI jobs this Mac can do now: the one
 * answer every AI surface shows or hides itself by. No surface checks an
 * engine, a model or a setting itself: `useAiJob("chat")` says whether
 * Chat's question box is there, and why not.
 *
 * Lazy: nothing is asked until a view wants it (the backend then looks at
 * the engines for the first time; no model is loaded). Cached: one answer
 * shared by every view, painted at once when a view opens again. Kept
 * current by the backend's `ai://jobs-changed` and by the window being
 * shown again, never by a timer.
 */
import { useEffect } from "react";
import { sharedRead, useSharedRead, whenForgotten } from "./shared-read";
import { aiJobs, onAiJobsChanged, type AiJob, type AiJobInfo } from "./ipc/ai";

export type { AiJob, AiJobInfo };

/** Kept until the backend says it changed. */
const KEPT = Number.POSITIVE_INFINITY;

const READ = sharedRead(aiJobs);

let stop: (() => void) | null = null;
function listen(): void {
  if (stop) return;
  stop = onAiJobsChanged(() => void READ.refresh());
  whenForgotten(() => {
    stop?.();
    stop = null;
  });
}

/** Whether a job's feature is shown: it runs, or waits a moment for its
 * model. Every other state hides it (never shown broken). */
export function isShown(info: AiJobInfo | null | undefined): boolean {
  return info?.state === "ready" || info?.state === "waiting";
}

/** Every routed job's answer (`null` until read). */
export function useAiJobs(): AiJobInfo[] | null {
  useEffect(listen, []);
  return useSharedRead(READ, { maxAgeMs: KEPT }) ?? null;
}

/** `job`'s answer (`null` until read, or when the job isn't routed). */
export function useAiJob(job: AiJob): AiJobInfo | null {
  return useAiJobs()?.find((info) => info.job === job) ?? null;
}

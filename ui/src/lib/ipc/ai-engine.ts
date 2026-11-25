/**
 * AJ-E3825 — an AI job's engine, as the window reads it: started,
 * running, idle, answering or failed with why (`ai_engine_status`, which
 * starts nothing), and its lazy start (`ai_engine_start`: Bot opening
 * with an AI chosen). Bot's header reads it; AJ-E3826's Models card reads
 * the same status and today's counts.
 *
 * The preview and tests set `window.__AIJ_AI_ENGINE__`; with none, the
 * engine is off.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { AiJob } from "../../types/generated/AiJob";
import type { AiEngineStatusInfo } from "../../types/generated/AiEngineStatusInfo";
import type { AiFailure } from "../../types/generated/AiFailure";

export type { AiEngineStatusInfo, AiFailure };

declare global {
  interface Window {
    __AIJ_AI_ENGINE__?: AiEngineStatusInfo;
  }
}

const OFF: AiEngineStatusInfo = {
  state: "off",
  failure: null,
  model_id: null,
  model_name: null,
  engine: null,
  version: null,
  context_tokens: null,
  started_ms: null,
  answered_ms: null,
  today: { answers: 0, tokens_in: { n: 0, estimated: false }, tokens_out: { n: 0, estimated: false } },
};

/** `job`'s engine now (starts nothing). */
export async function aiEngineStatus(job: AiJob): Promise<AiEngineStatusInfo> {
  if (!hasLiveBackend()) return window.__AIJ_AI_ENGINE__ ?? OFF;
  return call("ai_engine_status", { job });
}

/** Starts `job`'s engine when it isn't running; how it stands after. */
export async function aiEngineStart(job: AiJob): Promise<AiEngineStatusInfo> {
  if (!hasLiveBackend()) return window.__AIJ_AI_ENGINE__ ?? OFF;
  return call("ai_engine_start", { job });
}

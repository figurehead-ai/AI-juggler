/**
 * AJ-E3826 — a job's engine with no AI chosen: what `ai_engine_status`
 * answers without a backend, and the preview's base for its engine
 * states. Data only (no transport), so the gallery specs can import it.
 */
import type { AiEngineStatusInfo } from "../types/generated/AiEngineStatusInfo";

/** Off: no AI chosen for the job (the preview's base too). */
export const ENGINE_OFF: AiEngineStatusInfo = {
  state: "off",
  failure: null,
  model_id: null,
  model_name: null,
  short_name: null,
  engine: null,
  version: null,
  context_tokens: null,
  started_ms: null,
  answered_ms: null,
  today: { answers: 0, tokens_in: { n: 0, estimated: false }, tokens_out: { n: 0, estimated: false } },
};

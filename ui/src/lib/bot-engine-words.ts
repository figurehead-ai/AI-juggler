/**
 * AJ-E3825 — Bot's engine and its answers in words: the header's model
 * and state, each failure as one plain line with its fix, the details
 * line's numbers (an estimate is labelled "≈ … est."), and the context
 * and memory breakdowns' labels.
 */
import type { AiCount } from "../types/generated/AiCount";
import type { AiEngineStatusInfo, AiFailure } from "./ipc/ai-engine";
import type { BotContextPart } from "../types/generated/BotContextPart";

/** The model's short name for the header ("Apple on-device"). */
export function engineLabel(status: AiEngineStatusInfo | null): string {
  if (status?.engine === "apple_builtin") return "Apple on-device";
  return status?.model_name ?? "Bot’s AI";
}

/** The model's full name, for sentences ("Apple's on-device model"). */
export function modelName(status: AiEngineStatusInfo | null): string {
  return status?.model_name ?? "Bot’s AI";
}

/** The header's state word. */
export function engineStateWords(status: AiEngineStatusInfo | null, starting: boolean): string {
  if (starting || status?.state === "starting") return "Starting…";
  switch (status?.state) {
    case "running":
      return "Running";
    case "answering":
      return "Answering…";
    case "idle":
      return "Idle";
    case "failed":
      return `Failed: ${failureWords(status.failure ?? "unknown", modelName(status)).short}`;
    default:
      return "";
  }
}

/** "Starting Apple's on-device model…", from the engine's own name. */
export function startingLine(status: AiEngineStatusInfo | null): string {
  return `Starting ${modelName(status)}…`;
}

/** A failure: a short reason (the header), the line, and its fix. */
export function failureWords(failure: AiFailure, model: string): { short: string; line: string; fix: string } {
  switch (failure) {
    case "off":
      return { short: "Apple Intelligence is off", line: "Apple Intelligence is off, so Bot can’t answer.", fix: "Turn it on in System Settings › Apple Intelligence & Siri, then try again." };
    case "not_downloaded":
      return { short: "model not on this Mac yet", line: `${model} isn’t on this Mac yet.`, fix: "If macOS is still getting Apple’s model, try again in a few minutes; a downloaded model is in Settings › AI › Models." };
    case "helper_didnt_start":
      return { short: "helper didn’t start", line: "Bot’s helper didn’t start.", fix: "Try again; if it keeps failing, check this Mac in Settings › AI › Models." };
    case "timed_out":
      return { short: "took too long", line: `${model} took too long to answer.`, fix: "Try again, or ask a shorter question." };
    case "refused":
      return { short: "declined", line: `${model} declined to answer that.`, fix: "Try asking another way." };
    case "too_long":
      return { short: "too long", line: `That was too long for ${model}.`, fix: "Start a New topic, or ask in fewer words." };
    case "not_this_mac":
      return { short: "can’t run here", line: `This Mac can’t run ${model}.`, fix: "Choose another model in Settings › AI › Models." };
    default:
      return { short: "error", line: "Bot’s AI stopped with an error.", fix: "Try again; the app log has the details." };
  }
}

/** A token count: "820", "3.1k"; an estimate "≈ 820 est.". */
export function countWords(count: AiCount): string {
  const n = count.n >= 1000 ? `${(count.n / 1000).toFixed(count.n >= 10_000 ? 0 : 1)}k` : String(count.n);
  return count.estimated ? `≈ ${n} est.` : n;
}

/** A window: "8k", "4,096" read as "4.1k". */
export function windowWords(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(n % 1000 === 0 ? 0 : 1)}k` : String(n);
}

/** A duration: "0.8 s", "12 s". */
export function secondsWords(ms: number): string {
  return ms < 10_000 ? `${(ms / 1000).toFixed(1)} s` : `${Math.round(ms / 1000)} s`;
}

export const CONTEXT_PART_WORDS: Record<BotContextPart, string> = {
  instructions: "Instructions",
  memory: "Bot’s memory",
  records: "Records and look-ups",
  history: "Conversation history",
  question: "Your question",
};

export const MEMORY_KIND_WORDS: Record<string, string> = {
  preferences: "Preferences and names",
  notes: "Notes",
  asked: "Earlier questions and answers",
  watches: "Watches",
};

/** D-148's label for the model's own answer, and its scope line. */
export const GENERAL_LABEL = "Bot’s own answer · not from your records";
export const SCOPE_LINE = "Bot is here for your AI apps and sessions: ask what needs you, what finished, or how a session is going.";

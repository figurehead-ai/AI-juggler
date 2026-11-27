/**
 * AJ-E1547 (PAT-OSP) — the system prompts' registry and gate over IPC
 * (ENGINEERING-GUIDE U3: one thin module per backend area).
 *
 * - `osPromptsList`: every prompt AI Juggler can cause, with whether this
 *   setup needs it (`needed`, D-127), what the system says of it now and
 *   its dialog's words (`dialog`, what `OsPromptPicture` draws). Never asks.
 * - `osPromptPrime(id)`: setup's Allow on a card that already showed the
 *   dialog and what to click: marks it primed and makes the call.
 * - `osPromptRequest(id)`: a click about to use what `id` guards (the
 *   Watches form's Watch for `notifications`): our sheet first when it
 *   isn't primed and the system hasn't decided; nothing when it is, or
 *   when the user said Not now to an entry offered again from its
 *   setting (unless the click is that setting, `fromSetting`).
 * - `osPromptStatus(id)`: one entry now.
 * - The sheet's side: `onOsPromptAsk` (the backend is about to make the
 *   system ask), `osPromptAnswer` (Continue or Not now), `onOsPromptOutcome`.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import type { OsPromptInfo } from "../../types/generated/OsPromptInfo";
import type { OsPromptOutcomeInfo } from "../../types/generated/OsPromptOutcomeInfo";

export type { OsPromptInfo, OsPromptOutcomeInfo };
export type { OsPromptDialogInfo } from "../../types/generated/OsPromptDialogInfo";

/** The backend is about to make the system ask: show the sheet first. */
const OS_PROMPT_ASK = "os-prompt://ask";
/** What came of a call the user said Continue to. */
const OS_PROMPT_OUTCOME = "os-prompt://outcome";

declare global {
  interface Window {
    /** The browser preview's and the tests' entries (none unless set). */
    __AIJ_OS_PROMPTS__?: OsPromptInfo[];
  }
}

const seams = () => window;

function previewOutcome(id: string, result: string): OsPromptOutcomeInfo {
  // AJ-E1555: never a state word as the sheet's body, the preview's too.
  return { id, result, status: "asked", words: "The preview has no system to ask: nothing changed here." };
}

/** Every system prompt AI Juggler can cause (never asks).
 * @public setup's "Let AI Juggler work with…" step reads it (AJ-E1545). */
export async function osPromptsList(): Promise<OsPromptInfo[]> {
  if (!hasBackend) return seams().__AIJ_OS_PROMPTS__ ?? [];
  return call("os_prompts_list");
}

/** One entry as it stands now (never asks). */
export async function osPromptStatus(id: string): Promise<OsPromptInfo | null> {
  if (!hasBackend) return seams().__AIJ_OS_PROMPTS__?.find((prompt) => prompt.id === id) ?? null;
  return call("os_prompt_status", { id });
}

/** Setup's Allow on a priming card: primes `id` and makes its call.
 * @public setup's "Let AI Juggler work with…" step calls it (AJ-E1545). */
export async function osPromptPrime(id: string): Promise<OsPromptOutcomeInfo> {
  if (!hasBackend) return previewOutcome(id, "called");
  return call("os_prompt_prime", { id });
}

/** A click about to use what `id` guards: the sheet first when needed.
 * `fromSetting`: the click is the entry's own setting (its "offered
 * again" place), which asks again after an earlier Not now; any other
 * use doesn't (0.5.2 G33, PAT-OSP). */
export async function osPromptRequest(id: string, options: { fromSetting?: boolean } = {}): Promise<OsPromptOutcomeInfo> {
  if (!hasBackend) return previewOutcome(id, "already");
  return call("os_prompt_request", { id, fromSetting: options.fromSetting ?? null });
}

/** The sheet's answer for `id`: Continue (`go`) or Not now. */
export async function osPromptAnswer(id: string, go: boolean): Promise<void> {
  if (!hasBackend) return;
  await call("os_prompt_answer", { id, go });
}

/** Called when the backend is about to make the system ask. */
export function onOsPromptAsk(handler: (prompt: OsPromptInfo) => void): Promise<() => void> {
  if (!hasBackend) return Promise.resolve(() => undefined);
  return listen<OsPromptInfo>(OS_PROMPT_ASK, (event) => handler(event.payload));
}

/** Called with what came of a call the user said Continue to. */
export function onOsPromptOutcome(handler: (outcome: OsPromptOutcomeInfo) => void): Promise<() => void> {
  if (!hasBackend) return Promise.resolve(() => undefined);
  return listen<OsPromptOutcomeInfo>(OS_PROMPT_OUTCOME, (event) => handler(event.payload));
}

/** The entry Bot's AI helper is set up under: its first start (Bot's
 * setup guide's "Check this Mac", `models-actions` checkAgain). */
export const BOT_HELPER = "helper";

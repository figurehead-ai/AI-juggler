/**
 * AJ-E205: the switch itself, from the user's click on the last screen —
 * the freeze choice, the keys answer, and (D-51) the ticked offers the
 * switch carries out itself. The preview records each call in seams so
 * tests can prove it fires only from the click.
 */
import { hasLiveBackend, type SwitchExecutedData } from "../bridge";
import { invoke } from "./transport";

/** AJ-E1103: why the next switch happens — set when the user picks a
 * target (the suggested one, or another), read once by
 * [`switchExecute`]. */
let pendingSwitchTrigger: "suggestion" | "manual" | null = null;

/** AJ-E1103: the user picked a switch target. */
export function noteSwitchPick(suggested: boolean): void {
  pendingSwitchTrigger = suggested ? "suggestion" : "manual";
}

/** AJ-E205: executes the switch with the user's freeze choice
 * (`finish-turn` default | `stop-now`) — the pipeline stages,
 * verifies, launches, and records the edge. `approved`: the set-up
 * check's offers the user ticked that the switch itself carries out
 * (D-51: this project's notes quoted in the handover, for a tool that
 * keeps none). */
export async function switchExecute(
  sessionId: string,
  targetProfileId: string,
  freezeChoice: string,
  redactionChoice?: string | null,
  redactionUnredact?: number[],
  approved: string[] = [],
): Promise<SwitchExecutedData> {
  if (!hasLiveBackend()) {
    const seams = window as unknown as {
      __AIJ_SWITCH_EXECUTE_FAIL__?: string;
      __AIJ_SWITCH_EXECUTE_HANG__?: boolean;
      __AIJ_SWITCH_EXECUTE_REDACTION__?: {
        redactionChoice: string | null | undefined;
        redactionUnredact: number[];
      }[];
      __AIJ_SWITCH_EXECUTE_APPROVED__?: string[][];
    };
    // The guardrail seam: the redaction payload is observable so
    // tests can prove it fires only from the click.
    seams.__AIJ_SWITCH_EXECUTE_REDACTION__ = [
      ...(seams.__AIJ_SWITCH_EXECUTE_REDACTION__ ?? []),
      {
        redactionChoice,
        redactionUnredact: redactionUnredact ?? [],
      },
    ];
    seams.__AIJ_SWITCH_EXECUTE_APPROVED__ = [...(seams.__AIJ_SWITCH_EXECUTE_APPROVED__ ?? []), approved];
    if (seams.__AIJ_SWITCH_EXECUTE_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_EXECUTE_FAIL__);
    }
    if (seams.__AIJ_SWITCH_EXECUTE_HANG__) {
      return new Promise<SwitchExecutedData>(() => {});
    }
    return {
      session_id: `sess-preview-${Date.now()}`,
      pid: 0,
      edge_id: `edge-preview-${Date.now()}`,
      source_session_id: sessionId,
    };
  }
  const trigger = pendingSwitchTrigger ?? "manual";
  pendingSwitchTrigger = null;
  return invoke<SwitchExecutedData>("switch_execute", {
    sessionId,
    targetProfileId,
    freezeChoice,
    redactionChoice: redactionChoice ?? null,
    redactionUnredact: redactionUnredact ?? [],
    trigger,
    approved,
  });
}

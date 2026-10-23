/**
 * AJ-E1513 — the switch prepared before the limit (ENGINEERING-GUIDE U3:
 * one thin file per feature): whether a session's switch is staged, for
 * the limit dialog's "Ready to continue on …". In the preview,
 * `__AIJ_STAGED__` stands in.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { StagedSwitchInfo } from "../../types/generated/StagedSwitchInfo";

export type { StagedSwitchInfo };

/** The staged switch for `sessionId`, or null when none is ready. */
export async function stagingReady(sessionId: string): Promise<StagedSwitchInfo | null> {
  if (!hasLiveBackend()) {
    const staged = (window as unknown as { __AIJ_STAGED__?: StagedSwitchInfo }).__AIJ_STAGED__;
    return staged && staged.session_id === sessionId ? staged : null;
  }
  return invoke<StagedSwitchInfo | null>("staging_ready", { sessionId });
}

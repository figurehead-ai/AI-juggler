/**
 * D-76 (AJ-E211): what a switch would leave out of a very large
 * conversation, under its advanced options (reads only). By default
 * everything travels.
 */
import { hasLiveBackend } from "../bridge";
import { call } from "./transport";
import type { SwitchCutArgs } from "../../types/generated/SwitchCutArgs";
import type { SwitchCutInfo } from "../../types/generated/SwitchCutInfo";

export type { SwitchCutArgs, SwitchCutInfo };

/** The preview's (and the tests') conversation, when one is set. */
const seam = (): ((cut: SwitchCutArgs | null) => SwitchCutInfo) | undefined =>
  (window as unknown as { __AIJ_SWITCH_CUT__?: (cut: SwitchCutArgs | null) => SwitchCutInfo }).__AIJ_SWITCH_CUT__;

/** The preview's default: a switch that carries everything, no options. */
export const NO_CUT: SwitchCutInfo = {
  supported: false,
  turns: 0,
  first_ms: null,
  last_ms: null,
  bytes: 0,
  turns_left_out: 0,
  bytes_left_out: 0,
  from_ms: null,
  at_compaction: false,
  words: "",
  loses: null,
};

/** What `cut` would leave out of `sessionId`'s conversation there. */
export async function switchCutPreview(sessionId: string, targetProfileId: string, cut: SwitchCutArgs | null): Promise<SwitchCutInfo> {
  if (!hasLiveBackend()) return seam()?.(cut) ?? NO_CUT;
  return call("switch_cut_preview", { sessionId, targetProfileId, cut });
}

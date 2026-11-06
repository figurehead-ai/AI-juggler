/**
 * 0.3.9: the disk space a switch uses, read before it's confirmed (reads
 * only): the conversation's size, what the copy writes, what's free, and
 * what to do when that's low or short.
 */
import { hasLiveBackend } from "../bridge";
import { call } from "./transport";
import type { SwitchSpaceInfo } from "../../types/generated/SwitchSpaceInfo";

export type { SwitchSpaceInfo };

/** The preview's (and the tests') answer, when one is set. */
const seam = (): SwitchSpaceInfo | undefined => (window as unknown as { __AIJ_SWITCH_SPACE__?: SwitchSpaceInfo }).__AIJ_SWITCH_SPACE__;

/** The preview's default: a clone, plenty of room. */
export const EXAMPLE_SPACE: SwitchSpaceInfo = {
  kind: "plenty",
  words: "This conversation is 42 MB; it's copied as a clone, so it takes almost no disk space (120 GB free).",
  advice: null,
  carried_bytes: 42_000_000,
  writes_bytes: 0,
  free_bytes: 120_000_000_000,
};

/** The disk space switching `sessionId` to `targetProfileId` uses. */
export async function switchSpace(sessionId: string, targetProfileId: string): Promise<SwitchSpaceInfo> {
  if (!hasLiveBackend()) return seam() ?? EXAMPLE_SPACE;
  return call("switch_space_check", { sessionId, targetProfileId });
}

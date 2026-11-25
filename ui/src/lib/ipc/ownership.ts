/**
 * AJ-E2328: the worktree ownership check and its Repair (one thin file per
 * feature). In the preview, `__AIJ_OWNERSHIP__` stands in for the check
 * (nothing found by default) and `__AIJ_OWNERSHIP_REPAIRED__` for what a
 * Repair returns.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import { changingBackups } from "./switch-move";
import type { OwnershipInfo } from "../../types/generated/OwnershipInfo";
import type { OwnershipRepairedInfo } from "../../types/generated/OwnershipRepairedInfo";

export type { OwnershipInfo, OwnershipRepairedInfo };

declare global {
  interface Window {
    __AIJ_OWNERSHIP__?: OwnershipInfo;
    __AIJ_OWNERSHIP_REPAIRED__?: OwnershipRepairedInfo;
  }
}

const NOTHING: OwnershipInfo = { apps: [], findings: [], tidied: [], unreadable: null };

/** What the check finds now (reads only). */
export async function ownershipCheck(): Promise<OwnershipInfo> {
  if (!hasLiveBackend()) return structuredClone(window.__AIJ_OWNERSHIP__ ?? NOTHING);
  return call("switch_ownership", {});
}

/** Repairs one finding (the user's click, after the sheet). */
export async function ownershipRepair(key: string): Promise<OwnershipRepairedInfo> {
  if (!hasLiveBackend()) return changingBackups(Promise.resolve(window.__AIJ_OWNERSHIP_REPAIRED__ ?? { words: "Repaired.", notes: [] }));
  // 0.5.1 F93: a repair keeps a backup for its Undo.
  return changingBackups(call("switch_ownership_repair", { key }));
}

/**
 * AJ-E1538 — searching the words of sessions started outside AI Juggler,
 * opt-in. The index is built in the background from the tools' own files
 * and deleted when it's turned off. The preview keeps its state in
 * `__AIJ_OUTSIDE_SEARCH__` and records turns of the switch in
 * `__AIJ_OUTSIDE_SEARCH_CALLS__`.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { OutsideSearchInfo } from "../../types/generated/OutsideSearchInfo";
import type { OutsideSearchEstimateInfo } from "../../types/generated/OutsideSearchEstimateInfo";

export type { OutsideSearchInfo, OutsideSearchEstimateInfo };

type Preview = {
  __AIJ_OUTSIDE_SEARCH__?: OutsideSearchInfo;
  __AIJ_OUTSIDE_SEARCH_ESTIMATE__?: OutsideSearchEstimateInfo;
  __AIJ_OUTSIDE_SEARCH_CALLS__?: boolean[];
};
const preview = () => window as unknown as Preview;
const OFF: OutsideSearchInfo = { enabled: false, size_bytes: null, sessions: 0, indexed: 0 };

/** Whether it's on, and how far the index has got. */
export async function outsideSearchStatus(): Promise<OutsideSearchInfo> {
  if (!hasLiveBackend()) return preview().__AIJ_OUTSIDE_SEARCH__ ?? OFF;
  return invoke<OutsideSearchInfo>("outside_search_status");
}

/** The index's likely size before it's turned on. */
export async function outsideSearchEstimate(): Promise<OutsideSearchEstimateInfo> {
  if (!hasLiveBackend()) return preview().__AIJ_OUTSIDE_SEARCH_ESTIMATE__ ?? { size_bytes: 0, sessions: 0 };
  return invoke<OutsideSearchEstimateInfo>("outside_search_estimate");
}

/** Turns it on (building starts) or off (the index is deleted). */
export async function outsideSearchSet(on: boolean): Promise<OutsideSearchInfo> {
  if (!hasLiveBackend()) {
    (preview().__AIJ_OUTSIDE_SEARCH_CALLS__ ??= []).push(on);
    const now = preview().__AIJ_OUTSIDE_SEARCH__ ?? OFF;
    const next: OutsideSearchInfo = on
      ? { ...now, enabled: true, size_bytes: now.size_bytes ?? 0 }
      : { ...now, enabled: false, size_bytes: null, indexed: 0 };
    preview().__AIJ_OUTSIDE_SEARCH__ = next;
    return next;
  }
  return invoke<OutsideSearchInfo>("outside_search_set", { on });
}

/** "640 MB" / "1.2 GB" / "300 KB". */
export function bytesWords(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} KB`;
}

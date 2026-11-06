/**
 * AJ-E1007: session lifecycle: search, archive, replay, retention.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { AccountStorageInfo } from "../../types/generated/AccountStorageInfo";
import type { DeletePreviewInfo } from "../../types/generated/DeletePreviewInfo";
import type { ReplayInfo } from "../../types/generated/ReplayInfo";
import type { RetentionPreviewInfo } from "../../types/generated/RetentionPreviewInfo";
import type { TranscriptHitInfo } from "../../types/generated/TranscriptHitInfo";
import { previewHooks } from "../preview-hooks";


interface HistorySeams {
  __AIJ_HISTORY_CALLS__?: string[];
  __AIJ_TRANSCRIPT_HITS__?: TranscriptHitInfo[];
  __AIJ_REPLAY__?: ReplayInfo;
  __AIJ_DELETE_PREVIEW__?: DeletePreviewInfo;
  __AIJ_ACCOUNT_STORAGE__?: AccountStorageInfo[];
  __AIJ_RETENTION_PREVIEW__?: RetentionPreviewInfo;
}
const historySeams = () => window as unknown as HistorySeams;
const historyCall = (called: string) => {
  const seams = historySeams();
  seams.__AIJ_HISTORY_CALLS__ = [...(seams.__AIJ_HISTORY_CALLS__ ?? []), called];
};
const setMockLifecycle = (sessionId: string, event: string) => {
  const row = previewHooks().fixtures.MOCK_SESSIONS.find((session) => session.id === sessionId);
  if (row) {
    row.lifecycle = event;
    row.lifecycle_at_ms = Date.now();
  }
};

/** Archives an ended Instance: out of the active views, nothing deleted. */
export async function instanceArchive(sessionId: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`archive:${sessionId}`);
    setMockLifecycle(sessionId, "archived");
    return Date.now();
  }
  return call("instance_archive", { sessionId });
}

/** Unarchives an Instance — back exactly as it was. */
export async function instanceUnarchive(sessionId: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`unarchive:${sessionId}`);
    setMockLifecycle(sessionId, "unarchived");
    return Date.now();
  }
  return call("instance_unarchive", { sessionId });
}

/** What deleting an Instance would remove and keep (and its token). */
export async function instanceDeletePreview(sessionId: string): Promise<DeletePreviewInfo> {
  if (!hasBackend) {
    historyCall(`delete-preview:${sessionId}`);
    return (
      historySeams().__AIJ_DELETE_PREVIEW__ ?? {
        session_id: sessionId,
        account_label: "Work Max",
        refused: null,
        removes: ["Its record in AI Juggler", "Its history: 4 events", "Its search index: 120 lines"],
        keeps: ["The tool's own transcript file", "The process and cleanup audit", "Its usage in your usage totals"],
        token: "mock-token",
      }
    );
  }
  return call("instance_delete_preview", { sessionId });
}

/** Deletes an Instance's AI Juggler records (with its preview's token). */
export async function instanceDelete(sessionId: string, token: string): Promise<void> {
  if (!hasBackend) {
    historyCall(`delete:${sessionId}:${token}`);
    const at = previewHooks().fixtures.MOCK_SESSIONS.findIndex((session) => session.id === sessionId);
    if (at >= 0) previewHooks().fixtures.MOCK_SESSIONS.splice(at, 1);
    return;
  }
  return call("instance_delete", { sessionId, token });
}

/** Transcript text search (local index). */
export async function transcriptSearch(query: string): Promise<TranscriptHitInfo[]> {
  if (!hasBackend) {
    historyCall(`search:${query}`);
    const needle = query.trim().toLowerCase();
    return (historySeams().__AIJ_TRANSCRIPT_HITS__ ?? []).filter((hit) => hit.snippet.toLowerCase().includes(needle));
  }
  return call("transcript_search", { query });
}

/** The read-only replay of an ended or archived Instance. */
export async function instanceReplay(sessionId: string): Promise<ReplayInfo> {
  if (!hasBackend) {
    historyCall(`replay:${sessionId}`);
    const seeded = historySeams().__AIJ_REPLAY__;
    if (seeded) return { ...seeded, session_id: sessionId };
    throw new Error("No transcript recorded for this session");
  }
  return call("instance_replay", { sessionId });
}

/** Every Account's retention policy and storage used. */
export async function accountStorage(): Promise<AccountStorageInfo[]> {
  if (!hasBackend) {
    const seeded = historySeams().__AIJ_ACCOUNT_STORAGE__;
    if (seeded) return seeded.map((row) => ({ ...row }));
    return previewHooks().fixtures.MOCK_ACCOUNTS.map((account) => {
      const rows = previewHooks().fixtures.MOCK_SESSIONS.filter((session) => session.account_id === account.id);
      return {
        account_id: account.id,
        label: account.label,
        retention_days: null,
        instances: rows.length,
        history_events: rows.length * 3,
        index_lines: 0,
        transcript_bytes: 0,
      };
    });
  }
  return call("account_storage");
}

/** Sets an Account's retention (null = keep forever). */
export async function retentionSet(accountId: string, days: number | null): Promise<void> {
  if (!hasBackend) {
    historyCall(`retention:${accountId}:${days ?? "forever"}`);
    const row = (historySeams().__AIJ_ACCOUNT_STORAGE__ ?? []).find((entry) => entry.account_id === accountId);
    if (row) row.retention_days = days;
    return;
  }
  return call("retention_set", { accountId, days });
}

/** The retention dry run (nothing is deleted). */
export async function retentionPreview(accountId: string): Promise<RetentionPreviewInfo> {
  if (!hasBackend) {
    historyCall(`retention-preview:${accountId}`);
    return (
      historySeams().__AIJ_RETENTION_PREVIEW__ ?? {
        account_id: accountId,
        retention_days: null,
        deletes: [],
        kept: [],
        keeps: [],
        token: null,
      }
    );
  }
  return call("retention_preview", { accountId });
}

/** Applies the retention cleanup its dry run listed. */
export async function retentionApply(accountId: string, token: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`retention-apply:${accountId}:${token}`);
    return historySeams().__AIJ_RETENTION_PREVIEW__?.deletes.length ?? 0;
  }
  return call("retention_apply", { accountId, token });
}

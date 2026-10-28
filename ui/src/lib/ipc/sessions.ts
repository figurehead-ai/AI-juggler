/**
 * AJ-E1537 — the sessions feature's calls (ENGINEERING-GUIDE U3).
 */
import { invoke } from "./transport";
import { hasLiveBackend, type InstanceInfo } from "../bridge";
import type { Page, PageRequest } from "../paged";
import type { SessionRowInfo } from "../../types/generated/SessionRowInfo";
import type { SessionsCountsInfo } from "../../types/generated/SessionsCountsInfo";
import type { SessionsQuery } from "../../types/generated/SessionsQuery";
import type { SessionEventInfo } from "../../types/generated/SessionEventInfo";

export type { SessionEventInfo, SessionRowInfo, SessionsCountsInfo, SessionsQuery };

/** The copyable line that resumes a session in a terminal: built by its
 * tool's adapter, on its account, from its folder. */
export async function sessionsResumeCommand(sessionId: string): Promise<string> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_RESUME_COMMAND__?: string }).__AIJ_RESUME_COMMAND__;
    return seeded ?? `cd ~/work && tool --resume ${sessionId}`;
  }
  return invoke<string>("sessions_resume_command", { sessionId });
}

/** AJ-E1509: a launch setting's own choice about restarting its crashed
 * sessions; `null` follows Settings › Sessions. */
export async function sessionsRestartChoiceSet(recipeId: string, restart: boolean | null): Promise<void> {
  if (!hasLiveBackend()) return;
  return invoke<void>("sessions_restart_choice_set", { recipeId, restart });
}

/** History, a project's or an account's Sessions, a page at a time: open
 * first, then newest; narrowed and searched by the backend (0.1.5
 * review: these read every session at once). */
export async function sessionsPage(query: SessionsQuery, page: PageRequest): Promise<Page<SessionRowInfo>> {
  return invoke<Page<SessionRowInfo>>("sessions_page", { query, page });
}

/** A session's History tab (wf09): its status changes, lifecycle steps
 * and switches, oldest first, a page at a time. The design preview reads
 * `window.__AIJ_SESSION_HISTORY__[sessionId]`. */
export async function sessionsHistory(sessionId: string, page: PageRequest): Promise<Page<SessionEventInfo>> {
  if (!hasLiveBackend()) {
    const all = (window as unknown as { __AIJ_SESSION_HISTORY__?: Record<string, SessionEventInfo[]> }).__AIJ_SESSION_HISTORY__?.[sessionId] ?? [];
    const start = page.cursor ? Number(page.cursor) : 0;
    const end = start + (page.limit ?? 50);
    return { items: all.slice(start, end), next_cursor: end < all.length ? String(end) : null, total: all.length };
  }
  return invoke<Page<SessionEventInfo>>("sessions_history", { sessionId, page });
}

/** How many sessions a list holds: open, ended and archived. */
export async function sessionsCounts(query: SessionsQuery): Promise<SessionsCountsInfo> {
  return invoke<SessionsCountsInfo>("sessions_counts", { query });
}

/** A query with only what's given set (the rest `null`, as serde sends). */
export function sessionsQuery(over: Partial<SessionsQuery> = {}): SessionsQuery {
  return { project_id: null, account_id: null, which: null, text: null, utc_offset_min: -new Date().getTimezoneOffset(), ...over };
}

/** These sessions as the app's list has them (one past the working set:
 * an old one opened from History). */
export async function sessionsGet(ids: string[]): Promise<InstanceInfo[]> {
  if (!hasLiveBackend() || ids.length === 0) return [];
  return invoke<InstanceInfo[]>("sessions_get", { ids });
}

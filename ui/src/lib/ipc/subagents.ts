/**
 * AJ-E1520 — the Subagents tab's calls (ENGINEERING-GUIDE U3: the
 * feature's calls). 0.2: a session's subagents and one's conversation;
 * 0.3: a query (view, search, type, order, open groups) answered as rows
 * of a grouped tree, one subagent's page, and a project's subagents.
 * The backend asks the session's tool's adapter, read-only; a saved
 * index makes a long session open at once.
 */
import { channel, invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { PageRequest } from "../../types/generated/PageRequest";
import type { SubagentsInfo } from "../../types/generated/SubagentsInfo";
import type { SubagentInfo } from "../../types/generated/SubagentInfo";
import type { SubagentRowInfo } from "../../types/generated/SubagentRowInfo";
import type { SubagentsQuery } from "../../types/generated/SubagentsQuery";
import type { SubagentDetailInfo } from "../../types/generated/SubagentDetailInfo";
import type { LastMessagesInfo } from "../../types/generated/LastMessagesInfo";
import type { LastMessagesAsk } from "./conversations";
import type { ConversationDeltaInfo } from "../../types/generated/ConversationDeltaInfo";
import type { ConversationSnapshotInfo } from "../../types/generated/ConversationSnapshotInfo";
import { previewHooks } from "../preview-hooks";

export type { SubagentsInfo, SubagentInfo, SubagentRowInfo, SubagentsQuery, SubagentDetailInfo };

/** The query with nothing asked: All, newest first. */
export const NO_QUERY: SubagentsQuery = { view: null, search: null, kind: null, sort: null, open: [], closed: [] };

/** A session's subagents as rows: groups, then each group's tree. */
export async function subagentsList(accountId: string, conversationId: string, page: PageRequest, query: SubagentsQuery = NO_QUERY): Promise<SubagentsInfo> {
  if (!hasLiveBackend()) return previewHooks().subagents.previewList(conversationId, query, page);
  return invoke<SubagentsInfo>("subagents_list", { accountId, conversationId, query, page });
}

/** One subagent's own page. */
export async function subagentsDetail(accountId: string, conversationId: string, subagentId: string): Promise<SubagentDetailInfo> {
  if (!hasLiveBackend()) return previewHooks().subagents.previewDetail(conversationId, subagentId);
  return invoke<SubagentDetailInfo>("subagents_detail", { accountId, conversationId, subagentId });
}

/** A project's subagents: its newest sessions' together. */
export async function subagentsProject(projectId: string, page: PageRequest, query: SubagentsQuery = NO_QUERY): Promise<SubagentsInfo> {
  if (!hasLiveBackend()) return previewHooks().subagents.previewProject(projectId, query, page);
  return invoke<SubagentsInfo>("subagents_project", { projectId, query, page });
}

/** One subagent's conversation: `before` pages back, `from` follows
 * along, `known` hears "unchanged" without a read.
 * @deprecated 0.3.5: `subagentsFollow`; kept until 0.4.0. */
export async function subagentsConversation(accountId: string, conversationId: string, subagentId: string, ask: LastMessagesAsk = {}): Promise<LastMessagesInfo> {
  if (!hasLiveBackend()) {
    const turns = previewHooks().subagents.seededTurns(conversationId, subagentId)?.map((turn, index) => previewHooks().conversationFeed.seedTurn(turn, index));
    if (!turns) return { state: "none", turns: [], redacted: 0, earlier: null, stamp: null };
    const stamp = `preview:${turns.length}`;
    if (ask.before == null && ask.known === stamp) return { state: "unchanged", turns: [], redacted: 0, earlier: null, stamp };
    return { state: "shown", turns, redacted: 0, earlier: null, stamp };
  }
  return invoke<LastMessagesInfo>("subagents_conversation", {
    accountId,
    conversationId,
    subagentId,
    known: ask.known ?? null,
    before: ask.before ?? null,
    from: ask.from ?? null,
    max: ask.max ?? null,
  });
}

/** 0.3.5 (ADR-0024): follows one subagent's conversation while it shows,
 * as `conversationsFollow` does a session's (unfollow, keep and earlier
 * pages go through the conversations' calls, by subscription). */
export async function subagentsFollow(accountId: string, conversationId: string, subagentId: string, max: number, onDelta: (delta: ConversationDeltaInfo) => void): Promise<ConversationSnapshotInfo> {
  if (!hasLiveBackend()) {
    const turns = previewHooks().subagents.seededTurns(conversationId, subagentId);
    return turns ? previewHooks().conversationFeed.previewFollow(`${accountId}\u0000${conversationId}\u0000${subagentId}`, turns, "stretches", max, onDelta) : previewHooks().conversationFeed.previewWithout("none");
  }
  return invoke<ConversationSnapshotInfo>("subagents_follow", { accountId, conversationId, subagentId, max, channel: channel(onDelta) });
}

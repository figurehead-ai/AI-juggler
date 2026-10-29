/**
 * AJ-E1520 (0.2) — the session page's Subagents tab (ENGINEERING-GUIDE
 * U3: the feature's calls). The backend asks the session's tool's
 * adapter: every tool answers whether it records subagents, and lists a
 * session's from its own records, read-only. A page at a time; a
 * subagent's conversation reads like the session's own last messages.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { PageRequest } from "../../types/generated/PageRequest";
import type { SubagentsInfo } from "../../types/generated/SubagentsInfo";
import type { SubagentInfo } from "../../types/generated/SubagentInfo";
import type { LastMessagesInfo } from "../../types/generated/LastMessagesInfo";
import type { LastMessagesAsk } from "./conversations";

export type { SubagentsInfo, SubagentInfo };

/** What a preview (or a spec) seeds, by conversation id. */
type Seeded = {
  state?: string;
  reason?: string | null;
  live?: boolean;
  reading?: boolean;
  conversations_note?: string | null;
  items: SubagentInfo[];
  /** Each subagent's turns, by its id. */
  turns?: Record<string, LastMessagesInfo["turns"]>;
};

function fixture(): Record<string, Seeded> | undefined {
  return (window as unknown as { __AIJ_FIXTURE__?: { subagents?: Record<string, Seeded> } }).__AIJ_FIXTURE__?.subagents;
}

/** The preview's paging: the same order and cursor shape as the backend's (rows in order, the cursor an index). */
function previewList(conversationId: string, page: PageRequest): SubagentsInfo {
  const seeded = fixture()?.[conversationId];
  const empty = { items: [], next_cursor: null, total: 0 };
  if (!seeded) return { state: "listed", reason: null, page: empty, running: 0, reading: false, live: false, conversations_note: null };
  const start = page.cursor ? Number(page.cursor.replace("preview:", "")) : 0;
  const limit = page.limit ?? 50;
  const items = seeded.items.slice(start, start + limit);
  const next = start + limit < seeded.items.length ? `preview:${start + limit}` : null;
  return {
    state: seeded.state ?? "listed",
    reason: seeded.reason ?? null,
    page: { items, next_cursor: next, total: seeded.items.length },
    running: seeded.items.filter((item) => item.status === "running").length,
    reading: seeded.reading ?? false,
    live: seeded.live ?? false,
    conversations_note: seeded.conversations_note ?? null,
  };
}

/** A session's subagents, a page at a time (newest first, each followed
 * by the ones it started). */
export async function subagentsList(accountId: string, conversationId: string, page: PageRequest): Promise<SubagentsInfo> {
  if (!hasLiveBackend()) return previewList(conversationId, page);
  return invoke<SubagentsInfo>("subagents_list", { accountId, conversationId, page });
}

/** One subagent's conversation: `before` pages back, `from` follows
 * along, `known` hears "unchanged" without a read. */
export async function subagentsConversation(accountId: string, conversationId: string, subagentId: string, ask: LastMessagesAsk = {}): Promise<LastMessagesInfo> {
  if (!hasLiveBackend()) {
    const turns = fixture()?.[conversationId]?.turns?.[subagentId];
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

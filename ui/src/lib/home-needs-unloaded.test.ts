/**
 * E3104 Req. 2: Home's Needs you counts what the bell counts. A session's
 * need whose session the list hasn't loaded (past its working set) still
 * shows, in the notice's own words, opened by its ids.
 */
import { describe, expect, it } from "vitest";
import { homeNeedsOf } from "./needs-you";
import type { NoticeInfo } from "./ipc/notices";
import type { RailItem } from "./rail-items";

const notice = (over: Partial<NoticeInfo> & { target: Partial<NoticeInfo["target"]> }): NoticeInfo =>
  ({
    id: 1,
    kind: "asks",
    subject: "session:old-1",
    title: "Refactor the parser asks you",
    detail: "storefront · Claude Code · Work Max",
    needs: true,
    unread: true,
    snoozed: false,
    ...over,
    target: { session_id: null, conversation_id: null, account_id: null, project_id: null, ...over.target },
  }) as unknown as NoticeInfo;

describe("E3104 needs of sessions not loaded", () => {
  it("one of AI Juggler's shows from the notice and opens its session", () => {
    const [need] = homeNeedsOf([notice({ target: { session_id: "old-1" } })], [], [], [], null, Date.now());
    expect(need).toMatchObject({ kind: "session", title: "Refactor the parser asks you", sessionId: "old-1" });
    expect(need?.item).toBeUndefined();
  });

  it("another app's opens by its account and conversation", () => {
    const [need] = homeNeedsOf([notice({ kind: "stuck", subject: "conversation:c-9", target: { account_id: "claude-work", conversation_id: "c-9" } })], [], [], [], null, Date.now());
    expect(need).toMatchObject({ kind: "session", conversationIds: { accountId: "claude-work", id: "c-9" } });
  });

  it("a loaded session's need still uses its rail item", () => {
    const item = { key: "session:s-1", title: "feat/read-model", project: "ai-juggler", place: { app: "Claude Code", profile: "Work Max", kind: "own", host: null }, need: "asks", lastMs: 1 } as unknown as RailItem;
    const [need] = homeNeedsOf([notice({ subject: "session:s-1", target: { session_id: "s-1" } })], [item], [], [], null, Date.now());
    expect(need?.item).toBe(item);
  });
});

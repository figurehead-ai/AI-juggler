/**
 * The 0.4 review: a project's and a tool's "N working" count what the
 * rail draws as Working, so the number and the rows agree: a session
 * starting or running with nothing asking, and another app's live
 * conversation the rail says is working.
 */
import { describe, expect, it } from "vitest";
import { conversationWorking, openByProject, sessionWorking } from "./project-summary";
import type { InstanceInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";

const session = (id: string, status: string, over: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({ id, status, project_id: "p-1", provider: "claude-code", account_id: "claude-work", needs_attention: null, attention: null, ...over }) as unknown as InstanceInfo;
const conversation = (id: string, state: string, live = true): ConversationInfo =>
  ({ id, account_id: "claude-work", tool: "claude-code", project_id: "p-1", live, state, source: "app" }) as unknown as ConversationInfo;

describe("working, as the rail draws it", () => {
  it("a starting or running session works; one asking you or ended doesn't", () => {
    expect(sessionWorking(session("a", "running"))).toBe(true);
    expect(sessionWorking(session("b", "starting"))).toBe(true);
    expect(sessionWorking(session("c", "waiting_user"))).toBe(false);
    expect(sessionWorking(session("d", "finished"))).toBe(false);
  });

  it("another app's conversation works when it's live and working", () => {
    expect(conversationWorking(conversation("x", "working"))).toBe(true);
    expect(conversationWorking(conversation("y", "asks_you"))).toBe(false);
    expect(conversationWorking(conversation("z", "working", false))).toBe(false);
  });

  it("a project counts a starting session as working", () => {
    const open = openByProject([session("a", "running"), session("b", "starting"), session("c", "waiting_user")], []).get("p-1");
    expect(open?.open).toBe(3);
    expect(open?.working).toBe(2);
  });
});

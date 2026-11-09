/**
 * D-62 (AJ-E3008): a broad member's group node draws a line to every
 * session it brings in; a Watched member is drawn dashed with its fix.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as conversations from "../../lib/conversations";
import { CONTINUE_WATCHED } from "../../lib/pools";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { RuleNodes } from "./RuleNodes";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const rule = { member: "app:cli:work", kind: "app", role: "member", level: null, joined_at: 1, sessions: 2, session_ids: ["s-1", "s-2"] };

describe("D-62 the canvas's member rules", () => {
  it("an App's node says what it brings, with a line to each session on the canvas", () => {
    const placeOf = (id: string) => (id === "s-1" ? { x: 10, y: 20, w: 100, h: 40 } : null);
    render(<RuleNodes rules={[rule]} watched={[]} labelOf={() => "Claude Code · Work"} selection={{ kind: "none" }} onSelect={() => undefined} placeOf={placeOf} />);
    expect(screen.getByTestId("canvas-rule-node").textContent).toContain("2 sessions now · new ones join");
    // s-2 isn't drawn on the canvas: one line, to s-1.
    expect(screen.getByTestId("canvas-rule-lines").querySelectorAll("path")).toHaveLength(1);
  });

  it("a Watched member says it can't reply", () => {
    const watched = { member: "conversation:f/c", folder: "f", conversation_id: "c", title: "Fix sign-in copy", tool: "Claude app", account_label: null, project_id: null, role: "member", continued_session: null };
    render(<RuleNodes rules={[]} watched={[watched]} labelOf={() => ""} selection={{ kind: "none" }} onSelect={() => undefined} />);
    expect(screen.getByTestId("canvas-watched-node").textContent).toContain("can't reply");
  });

  it("AJ-E3006 R2: its Continue in AI Juggler says a new process starts, with the pool's tools, in this pool", () => {
    const watched = { member: "conversation:f/c", folder: "f", conversation_id: "c", title: "Fix sign-in copy", tool: "Claude app", account_label: null, project_id: null, role: "member", continued_session: null };
    const copy: ConversationInfo = { id: "c", tool: "claude-code", account_id: "f", account_label: "F", project_id: null, cwd: "/w", title: "Fix sign-in copy", git_branch: null, started_ms: 0, last_activity_ms: 0, tokens: null, live: false, session_id: null, places: [], pid: null, state: null, probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null };
    vi.spyOn(conversations, "useConversations").mockReturnValue({ all: [copy], refresh: () => {}, loaded: true, building: false });
    render(<RuleNodes rules={[]} watched={[watched]} labelOf={() => ""} selection={{ kind: "none" }} onSelect={() => undefined} />);
    const button = screen.getByTestId("canvas-watched-continue");
    expect(button.getAttribute("title")).toBe(CONTINUE_WATCHED);
    expect(CONTINUE_WATCHED).toMatch(/new process/);
    expect(CONTINUE_WATCHED).toMatch(/pool's tools/);
    expect(CONTINUE_WATCHED).toMatch(/stays in this pool/);
  });
});

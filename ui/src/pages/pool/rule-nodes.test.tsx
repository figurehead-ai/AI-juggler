/**
 * D-62 (AJ-E3008): a broad member's group node draws a line to every
 * session it brings in; a Watched member is drawn dashed with its fix.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RuleNodes } from "./RuleNodes";

afterEach(() => cleanup());

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
});

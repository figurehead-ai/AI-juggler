/**
 * D-62 (AJ-E3008): a broad member's group node draws a line to every
 * session it brings in.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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
    // The rc27 P1 walk: "now" is its open sessions (the backend's count).
    expect(screen.getByTestId("canvas-rule-node").textContent).toContain("2 open now · new ones join");
    // s-2 isn't drawn on the canvas: one line, to s-1.
    expect(screen.getByTestId("canvas-rule-lines").querySelectorAll("path")).toHaveLength(1);
  });

  // AJ-E3013: a session another app runs is drawn by DesktopMemberCard;
  // its states are in desktop-members.test.tsx.
});

// The P8 walk (rc32, step 4): a Kimi Code App member said nothing of
// working through its shell; only a seeded Kimi session's node did.
describe("P8 an App member whose sessions work through their shell", () => {
  it("says Shell only on its node, with the reason its sessions give", () => {
    const kimi = { ...rule, member: "app:cli:kimi-1" };
    const claude = { ...rule, member: "app:cli:work" };
    const toolOf = (one: { member: string }) => (one.member === "app:cli:kimi-1" ? { tool: "kimi-code", account: "kimi-1" } : { tool: "claude-code", account: "work" });
    render(<RuleNodes rules={[kimi, claude]} watched={[]} labelOf={(one) => one.member} toolOf={toolOf} selection={{ kind: "none" }} onSelect={() => undefined} />);
    const [kimiNode, claudeNode] = screen.getAllByTestId("canvas-rule-node");
    const mark = kimiNode!.querySelector('[data-testid="shell-only"]');
    expect(mark?.getAttribute("aria-description")).toMatch(/can't read the pool, post or take a task/);
    expect(kimiNode!.textContent).toContain("new ones join");
    expect(claudeNode!.querySelector('[data-testid="shell-only"]')).toBeNull();
  });
});

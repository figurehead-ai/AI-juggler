/**
 * The P8 walk: a pool member whose tool has no agent tools (Kimi Code
 * today; OpenCode and Grok Build have them since 0.4.1) wasn't labelled
 * anywhere but the New pool wizard's count.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { publishAgentTools } from "../lib/agent-tools";
import { ShellOnly } from "./ShellOnly";

afterEach(() => {
  cleanup();
  publishAgentTools([]);
});

describe("Shell only", () => {
  it("marks a member whose tool has no agent tools, and says why", () => {
    render(<ShellOnly tool="kimi-code" />);
    const mark = screen.getByTestId("shell-only");
    expect(mark.textContent).toBe("Shell only");
    expect(mark.getAttribute("title")).toMatch(/can't read the pool, post or take a task/);
  });

  it("says nothing for a tool with agent tools, or one not described yet", () => {
    render(
      <>
        <ShellOnly tool="claude-code" />
        <ShellOnly tool="codex" />
        <ShellOnly tool="opencode" />
        <ShellOnly tool="grok-build" />
        <ShellOnly tool="a-tool-nobody-described" />
      </>,
    );
    expect(screen.queryByTestId("shell-only")).toBeNull();
  });

  it("marks a member whose App can't take the tools, with its adapter's reason", () => {
    const why = "This App uses a Grok Build home AI Juggler didn't make: it works through its shell.";
    publishAgentTools([
      { id: "grok-theirs", agent_tools_why: why },
      { id: "grok-ours", agent_tools_why: null },
    ]);
    render(
      <>
        <ShellOnly tool="grok-build" account="grok-ours" />
        <ShellOnly tool="grok-build" account="grok-theirs" />
      </>,
    );
    const marks = screen.getAllByTestId("shell-only");
    expect(marks).toHaveLength(1);
    expect(marks[0]?.getAttribute("title")).toBe(why);
  });
});

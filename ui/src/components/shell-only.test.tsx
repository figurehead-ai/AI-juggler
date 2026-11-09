/**
 * The P8 walk: a pool member whose tool has no agent tools (Kimi Code
 * today; OpenCode and Grok Build have them since 0.4.1) wasn't labelled
 * anywhere but the New pool wizard's count.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { publishAgentTools } from "../lib/agent-tools";
import { resetPreviewAgentBridge, setAgentBridge } from "../lib/ipc/agent-bridge";
import { ShellOnly } from "./ShellOnly";

afterEach(() => {
  cleanup();
  publishAgentTools([]);
  resetPreviewAgentBridge();
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

  // The rc18 walk (E3002-R3 step 12): the tools off for a project
  // still showed its Grok member as having them.
  it("marks a member where the user switched the tools off, for its project or its App", () => {
    setAgentBridge({ asked: true, on: true, available: true, off_projects: ["p-off"], off_profiles: ["app-off"] });
    render(
      <>
        <ShellOnly tool="grok-build" account="grok-ours" project="p-on" />
        <ShellOnly tool="grok-build" account="grok-ours" project="p-off" />
        <ShellOnly tool="opencode" account="app-off" project="p-on" />
        <ShellOnly tool="claude-code" account="claude" />
      </>,
    );
    const marks = screen.getAllByTestId("shell-only");
    expect(marks).toHaveLength(2);
    expect(marks[0]?.getAttribute("title")).toMatch(/off for its project \(Settings › Agent access\)/);
    expect(marks[1]?.getAttribute("title")).toMatch(/off for its App \(Settings › Agent access\)/);
    for (const mark of marks) expect(mark.getAttribute("title")).toMatch(/can't read the pool, post or take a task/);
  });

  it("says so when the tools are off for every session, and claims nothing before the question is answered", () => {
    setAgentBridge({ asked: true, on: false, available: true, off_projects: [], off_profiles: [] });
    const off = render(<ShellOnly tool="grok-build" account="grok-ours" project="p-on" />);
    expect(screen.getByTestId("shell-only").getAttribute("title")).toMatch(/^The agent tools are off \(Settings › Agent access\)/);
    off.unmount();
    setAgentBridge({ asked: false, on: false, available: true, off_projects: ["p-off"], off_profiles: [] });
    render(<ShellOnly tool="grok-build" account="grok-ours" project="p-off" />);
    expect(screen.queryByTestId("shell-only")).toBeNull();
  });
});

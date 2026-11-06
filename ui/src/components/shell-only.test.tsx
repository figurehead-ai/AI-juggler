/**
 * The P8 walk: a pool member whose tool has no agent tools (OpenCode
 * today) wasn't labelled anywhere but the New pool wizard's count.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ShellOnly } from "./ShellOnly";

afterEach(cleanup);

describe("Shell only", () => {
  it("marks a member whose tool has no agent tools, and says why", () => {
    render(<ShellOnly tool="opencode" />);
    const mark = screen.getByTestId("shell-only");
    expect(mark.textContent).toBe("Shell only");
    expect(mark.getAttribute("title")).toMatch(/can't read the pool, post or take a task/);
  });

  it("says nothing for a tool with agent tools, or one not described yet", () => {
    render(
      <>
        <ShellOnly tool="claude-code" />
        <ShellOnly tool="codex" />
        <ShellOnly tool="a-tool-nobody-described" />
      </>,
    );
    expect(screen.queryByTestId("shell-only")).toBeNull();
  });
});

/**
 * AJ-E1549 R3 (0.5.1 F13) — the sign-in's terminal frame never paints
 * blank: while xterm's code loads, it holds a skeleton, then the terminal.
 * Its own file: the terminal's code must not be loaded by an earlier test.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SignInCard } from "./SignInCard";

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

afterEach(cleanup);

it("draws a skeleton in the terminal frame while its code loads, then the terminal", async () => {
  render(
    <SignInCard
      signIn={{ id: "login-1", account_id: "acct:openai:alice", tool: "codex", title: "Sign in · Codex", state: "running", page_host: null, code: null, pastes_code: false, last_lines: [], started_at: 0 }}
      terminalId="login-1"
      tool="codex"
      title="Sign in · Codex"
      onTryAgain={vi.fn()}
      onDismiss={vi.fn()}
    />,
  );
  const frame = screen.getByTestId("sign-in-terminal");
  expect(screen.getByRole("status", { name: "Loading the terminal" })).toBeTruthy();
  expect(frame.textContent).toBe("");
  expect(frame.children.length).toBeGreaterThan(0);
  expect(await screen.findByTestId("terminal-login-1")).toBeTruthy();
  expect(screen.queryByRole("status", { name: "Loading the terminal" })).toBeNull();
});

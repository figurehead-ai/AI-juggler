/**
 * AJ-E1612 (dry run, 2026-11-16): a Sign in terminal's footer names the
 * tool of the App it signs in, from that tool's spec (its adapter), never a
 * fixed vendor: a Kimi Code sign-in said "Complete Anthropic's sign-in".
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SessionPanel } from "./WorkPanels";
import { genericTool, setToolsForTests } from "../lib/tools";
import { account } from "../test/fixtures";

vi.mock("../components/TerminalView", () => ({ TerminalView: () => null }));

const kimi = { ...genericTool("kimi-code"), display_name: "Kimi Code", short_name: "Kimi" };
const claude = { ...genericTool("claude-code"), display_name: "Claude Code", short_name: "Claude" };

function panel(terminal: { id: string; title: string; ended: boolean; account?: string }) {
  return render(
    <SessionPanel
      session={null}
      project={null}
      loginTerminal={terminal}
      profiles={[]}
      accounts={[account({ id: "kimi-a", provider: "kimi-code", label: "lab A", login_state: "logged_out" }), account({ id: "work" })]}
      onOpenProject={() => undefined}
      onStopped={() => undefined}
      onHandover={() => undefined}
    />,
  );
}

describe("the Sign in terminal's footer", () => {
  beforeEach(() => setToolsForTests([claude, kimi]));
  afterEach(cleanup);

  it("names the signed-in App's own tool, not another vendor", () => {
    panel({ id: "login-1", title: "Sign in · lab A", ended: true, account: "kimi-a" });
    const footer = screen.getByTestId("sign-in-footer").textContent ?? "";
    expect(footer).toContain("Complete Kimi Code's sign-in above");
    expect(footer).not.toContain("Anthropic");
    expect(footer).not.toContain("Claude");
  });

  it("is not shown for a terminal that signs no App in", () => {
    panel({ id: "login-2", title: "Main", ended: true });
    expect(screen.queryByTestId("sign-in-footer")).toBeNull();
  });
});

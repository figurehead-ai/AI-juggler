/**
 * AJ-E1612 (the dry run, 2026-11-16) vitest: Home's Needs you rows tell
 * their Apps apart to assistive tech. Each row is one named group (the
 * first) or list item (the rest), named with its App in full; its Sign in
 * sits inside it, named for that App. Six tools' Apps all labelled "lab A"
 * once gave six buttons called "Sign in — lab A needs its sign-in", and a
 * driver pressed the wrong one. Visible words stay short.
 */
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeNeeds } from "./HomeNeeds";
import { needsParts, type HomeNeed } from "../lib/home-needs";
import { genericTool, setToolsForTests } from "../lib/tools";
import { account } from "../test/fixtures";

const TOOLS = [
  { ...genericTool("opencode"), display_name: "OpenCode" },
  { ...genericTool("kimi-code"), display_name: "Kimi Code" },
  { ...genericTool("grok-build"), display_name: "Grok Build" },
];

function signIn(provider: string): HomeNeed {
  const app = account({ id: `${provider}-a`, provider, label: "lab A", login_state: "logged_out" });
  return { key: `signin:${app.id}`, kind: "signin", title: "lab A needs its sign-in", detail: `${provider}'s own sign-in`, severe: false, account: app };
}

describe("Needs you rows and their Sign in, for assistive tech", () => {
  beforeEach(() => setToolsForTests(TOOLS));
  afterEach(cleanup);

  it("gives two Apps' rows two distinct buttons, each within its own row", () => {
    render(<HomeNeeds needs={[signIn("opencode"), signIn("kimi-code"), signIn("grok-build")]} handlers={{ onOpenSession: vi.fn() }} />);
    // Each need is its own card, a named list item (D-118's mock).
    const first = screen.getByRole("listitem", { name: "OpenCode CLI · lab A needs its sign-in" });
    const kimi = screen.getByRole("listitem", { name: "Kimi Code CLI · lab A needs its sign-in" });
    const grok = screen.getByRole("listitem", { name: "Grok Build CLI · lab A needs its sign-in" });
    expect(screen.getAllByRole("button", { name: /^Sign in to .* · lab A$/ })).toHaveLength(3);
    // Each App's button is inside its own row, and only there.
    expect(within(first).getByRole("button", { name: "Sign in to OpenCode CLI · lab A" })).toBeTruthy();
    expect(within(kimi).getByRole("button", { name: "Sign in to Kimi Code CLI · lab A" })).toBeTruthy();
    expect(within(grok).getByRole("button", { name: "Sign in to Grok Build CLI · lab A" })).toBeTruthy();
    expect(within(kimi).queryByRole("button", { name: /OpenCode|Grok/ })).toBeNull();
    expect(within(first).queryByRole("button", { name: /Kimi|Grok/ })).toBeNull();
    // What shows stays short and calm: "Sign in", and the title as before.
    expect(within(kimi).getAllByText("lab A needs its sign-in")).toHaveLength(1);
    expect(within(kimi).getByRole("button").textContent?.startsWith("Sign in")).toBe(true);
  });

  it("names the heading's sign-ins by their App in its spoken parts", () => {
    expect(needsParts([signIn("opencode"), signIn("kimi-code")], { caps: [], messages: 0, tasks: 0 })).toEqual(["OpenCode CLI · lab A needs its sign-in", "Kimi Code CLI · lab A needs its sign-in"]);
  });

  it("keeps a need that knows no App as it was", () => {
    const bare: HomeNeed = { key: "signin:x", kind: "signin", title: "Work needs its sign-in", detail: "", severe: false };
    render(<HomeNeeds needs={[bare]} handlers={{ onOpenSession: vi.fn() }} />);
    const row = screen.getByRole("listitem", { name: "Work needs its sign-in" });
    expect(within(row).getByRole("button", { name: "Sign in — Work needs its sign-in" })).toBeTruthy();
  });
});


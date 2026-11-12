/**
 * P2 (the D3 persona walk): an ended session's page said "ran 3 h ago ·
 * scrollback replayed" over an empty terminal: AI Juggler keeps a
 * terminal's output only while it runs. It now says that line only when
 * output was kept; else it shows the session's conversation, or says
 * plainly that nothing was kept, with History's search.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EndedTerminal } from "./EndedTerminal";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const NOW = 1_800_000_000_000;

/** The design preview's seam: the terminals that kept output. */
function keep(ids: string[] | undefined): void {
  Object.assign(window, { __AIJ_TERMINAL_KEPT__: ids });
}

afterEach(() => {
  cleanup();
  keep(undefined);
});

function ended(over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-old", launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "lab", account_label: "Lab", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "finished", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: null, git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 3 * 86_400_000,
    last_activity_ms: NOW - 3 * 86_400_000 + 600_000, machine_id: "this-mac", cwd: "/w/api", kind: "terminal", isolation_label: null, project_id: null, conversation_id: null, label: "Fix the login flow",
    ...over,
  };
}

const terminal = <div data-testid="instance-terminal" />;

describe("an ended terminal session's main pane (P2)", () => {
  it("with its output kept: the terminal, replayed, and the line that says so", async () => {
    keep(["sess-old"]);
    const view = render(<EndedTerminal session={ended()} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-kept");
    expect(view.getByTestId("instance-terminal")).toBeTruthy();
    expect(view.container.textContent).toContain("scrollback replayed");
  });

  it("none kept, with a conversation: its conversation, never an empty terminal under \"replayed\"", async () => {
    const view = render(<EndedTerminal session={ended({ conversation_id: "c-login" })} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-conversation");
    expect(view.queryByTestId("instance-terminal")).toBeNull();
    expect(view.getByTestId("last-messages")).toBeTruthy();
    expect(view.container.textContent).toContain("Its terminal output wasn't kept, so this is its conversation.");
    expect(view.container.textContent).not.toContain("replayed");
  });

  it("none kept and no conversation: says so plainly, and History's search finds it", async () => {
    const opened = vi.fn();
    const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:open-screen", listen);
    const view = render(<EndedTerminal session={ended()} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-none");
    expect(view.queryByTestId("instance-terminal")).toBeNull();
    expect(view.container.textContent).toContain("No output was kept");
    expect(view.container.textContent).toContain("Claude Code recorded no conversation for this session.");
    expect(view.container.textContent).not.toContain("replayed");
    fireEvent.click(view.getByTestId("ended-terminal-history"));
    expect(opened).toHaveBeenCalledWith({ id: "history", query: "Fix the login flow" });
    window.removeEventListener("aij:open-screen", listen);
  });

  it("asks before it shows anything: no terminal under a claim while it isn't known", () => {
    const view = render(<EndedTerminal session={ended()} terminal={terminal} now={NOW} />);
    expect(view.getByTestId("ended-terminal-asking").getAttribute("aria-busy")).toBe("true");
    expect(view.container.textContent).toBe("");
  });
});

/**
 * P2 (the D3 persona walk): an ended session's page said "ran 3 h ago ·
 * scrollback replayed" over an empty terminal: AI Juggler keeps a
 * terminal's output only while it runs. It now says that line only when
 * output was kept; else it shows the session's conversation, or says
 * plainly that nothing was kept, with History's search.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionTerminal, forgetKept } from "./SessionTerminal";
import { endedCause } from "../../lib/ended-how";
import { pastWhen } from "../../lib/time-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const NOW = 1_800_000_000_000;

/** The design preview's seam: the terminals that kept output. */
function keep(ids: string[] | undefined): void {
  Object.assign(window, { __AIJ_TERMINAL_KEPT__: ids });
}

afterEach(() => {
  cleanup();
  keep(undefined);
  forgetKept();
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
    const view = render(<SessionTerminal session={ended()} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-kept");
    expect(view.getByTestId("instance-terminal")).toBeTruthy();
    expect(view.container.textContent).toContain("scrollback replayed");
  });

  it("none kept, with a conversation: its conversation, never an empty terminal under \"replayed\"", async () => {
    const view = render(<SessionTerminal session={ended({ conversation_id: "c-login" })} live={false} terminal={terminal} now={NOW} />);
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
    const view = render(<SessionTerminal session={ended()} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-none");
    expect(view.queryByTestId("instance-terminal")).toBeNull();
    expect(view.container.textContent).toContain("No output was kept");
    expect(view.container.textContent).toContain("Claude Code recorded no conversation for this session.");
    expect(view.container.textContent).not.toContain("replayed");
    fireEvent.click(view.getByTestId("ended-terminal-history"));
    expect(opened).toHaveBeenCalledWith({ id: "history", query: "Fix the login flow" });
    window.removeEventListener("aij:open-screen", listen);
  });

  // The 0.4.6 RC walk K2 (F-K2-6): opened ended, the pane was a blank
  // black terminal for about a second while that was asked.
  it("opened ended, while it's asked: a skeleton at once, never a blank terminal, and claims nothing", () => {
    const view = render(<SessionTerminal session={ended()} live={false} terminal={terminal} now={NOW} />);
    const asking = view.getByTestId("ended-terminal-asking");
    expect(view.getByRole("status", { name: "Reading what its terminal kept" })).toBeTruthy();
    expect(asking.querySelectorAll(".bg-skeleton").length).toBeGreaterThan(0);
    expect(view.queryByTestId("instance-terminal")).toBeNull();
    expect(view.container.textContent).toBe("");
  });

  it("opened again, what it kept shows at once (the answer is kept)", async () => {
    const first = render(<SessionTerminal session={ended({ conversation_id: "c-login" })} live={false} terminal={terminal} now={NOW} />);
    await first.findByTestId("ended-terminal-conversation");
    first.unmount();
    const again = render(<SessionTerminal session={ended({ conversation_id: "c-login" })} live={false} terminal={terminal} now={NOW} />);
    expect(again.getByTestId("ended-terminal-conversation")).toBeTruthy();
    expect(again.queryByTestId("ended-terminal-asking")).toBeNull();
    // Run again since (its last activity moved): asked again.
    again.unmount();
    const ranAgain = render(<SessionTerminal session={ended({ conversation_id: "c-login", last_activity_ms: NOW - 60_000 })} live={false} terminal={terminal} now={NOW} />);
    expect(ranAgain.getByTestId("ended-terminal-asking")).toBeTruthy();
  });

  it("running, its terminal alone; ending as you watch, the same terminal stays (never rebuilt)", async () => {
    keep(["sess-old"]);
    const view = render(<SessionTerminal session={ended({ status: "running" })} live terminal={terminal} now={NOW} />);
    const shown = view.getByTestId("instance-terminal");
    expect(view.container.textContent).not.toContain("replayed");
    view.rerender(<SessionTerminal session={ended()} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-kept");
    expect(view.getByTestId("instance-terminal")).toBe(shown);
    expect(view.container.textContent).toContain("scrollback replayed");
  });
});

// SK-5 (the 0.4.6 soak, K3): an errored session's page said only "Error
// 8:35 AM" and "No output was kept…".
describe("an errored session says why it stopped (SK-5)", () => {
  const at = pastWhen(NOW - 3 * 86_400_000 + 600_000, NOW);

  it("its exit code and when, in words, and Start again opens New session in its folder on its App", async () => {
    const asked = vi.fn();
    const listen = (event: Event) => asked(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:new-session", listen);
    const view = render(<SessionTerminal session={ended({ status: "failed", ending: { how: "error", detail: "exit code 1" } })} live={false} terminal={terminal} now={NOW} />);
    const card = await view.findByTestId("ended-terminal-none");
    expect(card.textContent).toContain("It stopped with an error");
    expect(view.getByTestId("ended-why").textContent).toBe(`Claude Code exited with code 1, at ${at}.`);
    expect(card.textContent).toContain("Start it again in the same folder");
    fireEvent.click(view.getByTestId("ended-start-again"));
    expect(asked).toHaveBeenCalledWith({ accountId: "lab", dir: "/w/api" });
    window.removeEventListener("aij:new-session", listen);
  });

  it("with no detail, when, and that its tool didn't say", async () => {
    const view = render(<SessionTerminal session={ended({ status: "failed", ending: { how: "error", detail: null } })} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-none");
    expect(view.getByTestId("ended-why").textContent).toBe(`It stopped at ${at}; Claude Code didn't say why.`);
  });

  it("with its conversation shown, the why is the line above it", async () => {
    const view = render(<SessionTerminal session={ended({ conversation_id: "c-1", ending: { how: "crashed", detail: "signal 9" } })} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-conversation");
    expect(view.getByTestId("ended-why").textContent).toBe(`Claude Code was killed (signal 9), at ${at}.`);
  });

  it("one that finished says nothing of the sort", async () => {
    const view = render(<SessionTerminal session={ended({ ending: { how: "finished", detail: null } })} live={false} terminal={terminal} now={NOW} />);
    await view.findByTestId("ended-terminal-none");
    expect(view.queryByTestId("ended-why")).toBeNull();
    expect(view.queryByTestId("ended-start-again")).toBeNull();
  });

  it("the words for codes and signals", () => {
    expect(endedCause("exit code 127", "Codex")).toBe("Codex exited with code 127 (its command wasn't found)");
    expect(endedCause("signal 15", "Codex")).toBe("Codex was asked to quit (signal 15)");
    expect(endedCause("signal 31", "Codex")).toBe("Codex was stopped (signal 31)");
    expect(endedCause(null, "Codex")).toBeNull();
  });
});

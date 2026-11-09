/**
 * The P8 walk: a pool member whose tool has no agent tools (Kimi Code
 * today; OpenCode and Grok Build have them since 0.4.1) wasn't labelled
 * anywhere but the New pool wizard's count.
 */
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { publishAgentTools } from "../lib/agent-tools";
import { resetPreviewAgentBridge, setAgentBridge } from "../lib/ipc/agent-bridge";
import { ShellOnly } from "./ShellOnly";
import { PoolCardRows } from "./PoolCards";
import { MOCK_SESSIONS } from "../preview/fixtures";
import type { ContextCard, PoolMemberRow } from "../lib/ipc/pools";
import type { PagedList } from "../lib/paged";

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
    expect(mark.getAttribute("aria-description")).toMatch(/can't read the pool, post or take a task/);
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
    expect(marks[0]?.getAttribute("aria-description")).toBe(why);
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
    expect(marks[0]?.getAttribute("aria-description")).toMatch(/off for its project \(Settings › Agent access\)/);
    expect(marks[1]?.getAttribute("aria-description")).toMatch(/off for its App \(Settings › Agent access\)/);
    for (const mark of marks) expect(mark.getAttribute("aria-description")).toMatch(/can't read the pool, post or take a task/);
  });

  it("says so when the tools are off for every session, and claims nothing before the question is answered", () => {
    setAgentBridge({ asked: true, on: false, available: true, off_projects: [], off_profiles: [] });
    const off = render(<ShellOnly tool="grok-build" account="grok-ours" project="p-on" />);
    expect(screen.getByTestId("shell-only").getAttribute("aria-description")).toMatch(/^The agent tools are off \(Settings › Agent access\)/);
    off.unmount();
    setAgentBridge({ asked: false, on: false, available: true, off_projects: ["p-off"], off_profiles: [] });
    render(<ShellOnly tool="grok-build" account="grok-ours" project="p-off" />);
    expect(screen.queryByTestId("shell-only")).toBeNull();
  });

  // The rc19 walk: the chip showed in the canvas, the List view and an
  // expanded row, and nowhere said why; hovering it on the canvas showed
  // nothing (its reason was a native title only).
  describe("its reason, reachable", () => {
    const OFF = /^The agent tools are off for its project \(Settings › Agent access\)/;
    const offForProject = () => setAgentBridge({ asked: true, on: true, available: true, off_projects: ["p-off"], off_profiles: [] });

    it("shows a tip with the reason on hover, and on keyboard focus; Escape closes it", async () => {
      offForProject();
      render(<ShellOnly tool="grok-build" account="grok-ours" project="p-off" />);
      const mark = screen.getByTestId("shell-only");
      expect(mark.hasAttribute("title"), "no native title: it never showed in the app's view").toBe(false);
      expect(mark.getAttribute("aria-description")).toMatch(OFF);
      expect(screen.queryByRole("tooltip")).toBeNull();
      await act(async () => {
        fireEvent.pointerEnter(mark);
      });
      expect(screen.getByRole("tooltip").textContent).toMatch(OFF);
      expect(mark.getAttribute("aria-describedby")).toBe(screen.getByRole("tooltip").id);
      await act(async () => {
        fireEvent.pointerLeave(mark);
      });
      expect(screen.queryByRole("tooltip")).toBeNull();
      // A tab stop of its own outside a row.
      expect(mark.tabIndex).toBe(0);
      // Tab into it (a click's focus shows no tip).
      fireEvent.keyDown(document.body, { key: "Tab" });
      await act(async () => {
        mark.focus();
      });
      expect(screen.getByRole("tooltip").textContent).toMatch(OFF);
      await act(async () => {
        fireEvent.keyDown(window, { key: "Escape" });
      });
      expect(screen.queryByRole("tooltip")).toBeNull();
    });

    it("on a canvas node, shows its tip while the node has the keyboard's focus", async () => {
      offForProject();
      render(
        <div data-node="m1">
          <button type="button" data-testid="node-name">
            Builder
          </button>
          <ShellOnly tool="grok-build" account="grok-ours" project="p-off" inRow />
        </div>,
      );
      expect(screen.getByTestId("shell-only").hasAttribute("tabindex"), "the node keeps its one tab stop").toBe(false);
      fireEvent.keyDown(document.body, { key: "Tab" });
      await act(async () => {
        screen.getByTestId("node-name").focus();
      });
      expect(screen.getByRole("tooltip").textContent).toMatch(OFF);
      await act(async () => {
        screen.getByTestId("node-name").blur();
      });
      expect(screen.queryByRole("tooltip")).toBeNull();
    });

    it("the List view's row and its expanded row write the reason out", () => {
      offForProject();
      const session = { ...MOCK_SESSIONS[0]!, provider: "grok-build", account_id: "grok-ours", project_id: "p-off" };
      const card: ContextCard = {
        session_id: session.id,
        name: "Builder",
        place: "shop · Grok",
        tool: "Grok Build",
        account: "Grok",
        state: "running",
        role: "member",
        task: { value: "None", unavailable: null },
        said: { value: null, unavailable: "Unavailable" },
        last_activity_ms: null,
        files: [],
        files_unavailable: null,
      };
      const list: PagedList<PoolMemberRow> = {
        items: [{ session, card, via: "session", held_by: null }],
        total: 1,
        loading: false,
        stale: false,
        loadingMore: false,
        hasMore: false,
        error: null,
        loadMore: () => {},
        refresh: () => {},
      };
      render(<PoolCardRows list={list} sessions={[session]} now={Date.now()} />);
      const row = screen.getByTestId("pool-card");
      expect(within(row).getByTestId("shell-only").textContent).toBe("Shell only");
      expect(within(row).getByTestId("pool-card-shell-why").textContent).toMatch(OFF);
      fireEvent.click(within(row).getByTestId("pool-card-more"));
      expect(within(row).getByTestId("pool-card-detail-shell").textContent).toMatch(OFF);
    });
  });
});

/**
 * 0.3.4, the page-switching report — a session's page opened again has
 * its Messages and Tasks tabs in its first frame (they popped in a read
 * later), the tab it was left on comes back even when it's one of those,
 * and its hidden tabs (Quality, Processes, Subagents) read nothing until
 * they're opened.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "../../components/RightInspector";
import * as bridge from "../../lib/bridge";
import * as messages from "../../lib/ipc/messages";
import * as tasks from "../../lib/ipc/tasks";
import type { InstanceInfo } from "../../lib/bridge";

vi.mock("../../components/TerminalView", () => ({ TerminalView: () => <div data-testid="instance-terminal" /> }));
vi.mock("../../lib/pools", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../lib/pools")>()),
  usePoolPlace: (sessionId: string | null) => (sessionId ? { session_id: sessionId, pool_id: "pool-1", role: "member" } : null),
}));

const NOW = 1_728_800_000_000;
const instance = {
  id: "sess-1",
  launch_recipe_id: "p-1",
  adopted: false,
  provider: "claude-code",
  account_id: "claude-work",
  account_label: "Work Max",
  model: "opus",
  status: "running",
  quota: null,
  tokens: null,
  cpu_pct: 12,
  rss_bytes: 1_000_000_000,
  needs_attention: null,
  attention: null,
  started_at_ms: NOW - 60_000,
  last_activity_ms: NOW - 5_000,
  machine_id: "this-mac",
  cwd: "/w/alpha",
  kind: "terminal",
  project_id: null,
  conversation_id: "conv-1",
} as unknown as InstanceInfo;

const page = () =>
  render(
    <RightInspector
      selection={{ kind: "instance", id: "sess-1" }}
      data={{ instance, project: null, account: null, provider: null, machineId: "this-mac" }}
      sessions={[instance]}
      projects={[]}
      accounts={[]}
      profiles={[]}
      terminalTitle={null}
      freshness={{}}
      now={NOW}
      onOpenProject={() => {}}
      onOpenSession={() => {}}
      onLaunch={() => {}}
      onStopped={() => {}}
      onHandover={() => {}}
      onNew={() => {}}
    />,
  );

const selected = (view: ReturnType<typeof page>) => view.container.querySelector("[role='tab'][aria-selected='true']")?.id;

let threads: number;
beforeEach(() => {
  localStorage.clear();
  threads = 1;
  vi.spyOn(messages, "messagesThreads").mockImplementation(async () => ({ threads: Array.from({ length: threads }, () => ({})), next_cursor: null }) as never);
  vi.spyOn(tasks, "tasksOfSession").mockResolvedValue({ items: [{}], next_cursor: null, total: 1 } as never);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("0.3.4 a session's tabs, kept", () => {
  it("opened again, its Messages and Tasks tabs are there in the first frame", async () => {
    const first = page();
    // The first visit learns them a read later.
    expect(first.container.querySelector("#page-tab-messages")).toBeNull();
    await waitFor(() => expect(first.container.querySelector("#page-tab-messages")).not.toBeNull());
    await waitFor(() => expect(first.container.querySelector("#page-tab-tasks")).not.toBeNull());
    first.unmount();
    const again = page();
    expect(again.container.querySelector("#page-tab-messages")).not.toBeNull();
    expect(again.container.querySelector("#page-tab-tasks")).not.toBeNull();
  });

  it("a page left on Messages opens on Messages, on the first visit once it's known", async () => {
    localStorage.setItem("aij.page.tab.session", "messages");
    const first = page();
    await waitFor(() => expect(selected(first)).toBe("page-tab-messages"));
    first.unmount();
    const again = page();
    expect(selected(again)).toBe("page-tab-messages");
  });

  it("a session without messages shows its body, and a message arriving later doesn't pull the page off it", async () => {
    localStorage.setItem("aij.page.tab.session", "messages");
    threads = 0;
    const heard: (() => void)[] = [];
    vi.spyOn(messages, "onMessagesChanged").mockImplementation((handler) => {
      heard.push(handler);
      return () => undefined;
    });
    const view = page();
    await act(async () => undefined);
    expect(view.container.querySelector("#page-tab-messages")).toBeNull();
    expect(selected(view)).toBe("page-tab-terminal");
    // A first message: the tab comes, the page stays where it is.
    threads = 1;
    await act(async () => heard.forEach((again) => again()));
    await waitFor(() => expect(view.container.querySelector("#page-tab-messages")).not.toBeNull());
    expect(selected(view)).toBe("page-tab-terminal");
  });
});

describe("0.3.4 hidden tabs read nothing until opened", () => {
  it("Quality reads when its Usage tab shows, not with the page", async () => {
    const quality = vi.spyOn(bridge, "instanceQuality").mockResolvedValue(null);
    const view = page();
    await act(async () => undefined);
    expect(quality).not.toHaveBeenCalled();
    fireEvent.click(view.container.querySelector("#page-tab-usage")!);
    await waitFor(() => expect(quality).toHaveBeenCalledTimes(1));
  });

  it("Processes mounts its view when opened, not with the page", async () => {
    const view = page();
    await act(async () => undefined);
    expect(view.container.querySelector("[data-testid='session-processes']")).toBeNull();
    fireEvent.click(view.container.querySelector("#page-tab-processes")!);
    await waitFor(() => expect(view.container.querySelector("[data-testid='session-processes']")).not.toBeNull());
  });

  it("the Usage tab opened again shows its last signals at once", async () => {
    vi.spyOn(bridge, "instanceQuality").mockResolvedValue({
      tokens_bars: null,
      avg_tokens_per_min: 0,
      errors_not_measurable: true,
      error_label: "",
      context_used: null,
      context_window: null,
      cumulative: { input: 0, output: 0 },
      session_cost_usd: null,
      quota_forecast: "Not using its limit right now",
      stuck: null,
    } as never);
    localStorage.setItem("aij.page.tab.session", "usage");
    const first = page();
    await waitFor(() => expect(first.container.querySelector("[data-testid='quality-forecast']")).not.toBeNull());
    first.unmount();
    const again = page();
    expect(again.container.querySelector("[data-testid='quality-forecast']")).not.toBeNull();
  });
});

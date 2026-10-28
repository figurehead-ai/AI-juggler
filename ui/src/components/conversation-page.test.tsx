/**
 * AJ-E2108 vitest — a session's page holds still while the list refreshes
 * under it (the 0.1.4 report: "keeps refreshing, loses and gains the tab
 * up top"): its tabs never come and go with its token counts, a refresh
 * that misses its row never flashes "gone", and only a row missing for
 * GONE_MS is called gone.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage, GONE_MS } from "./ConversationPage";
import { ConversationsProvider, useConversations } from "../lib/conversations";
import * as ipc from "../lib/ipc/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";

const NOW = 1_800_000_000_000;

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  delete (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__;
});

function conversation(overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id: "c",
    tool: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    project_id: "p1",
    cwd: "/w/app",
    title: "Fix the flaky test",
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    tokens: null,
    live: true,
    session_id: null,
    places: [],
    pid: 7,
    state: "working",
    viewed_ms: null,
    probably: false,
    background: false,
    inactive: false,
    subtask_count: null,
    ...overrides,
  };
}

let refresh: () => void = () => {};
function Refresher(): null {
  refresh = useConversations().refresh;
  return null;
}

function page(rows: ConversationInfo[]): JSX.Element {
  (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__ = { conversations: rows };
  return (
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <Refresher />
        <ConversationPage accountId="a1" id="c" now={NOW} />
      </ActionHostProvider>
    </ConversationsProvider>
  );
}

/** The list comes back with `rows` (a backend refresh). */
async function listIs(rows: ConversationInfo[]) {
  (window as unknown as { __AIJ_FIXTURE__: { conversations: ConversationInfo[] } }).__AIJ_FIXTURE__.conversations = rows;
  await act(async () => {
    refresh();
    await Promise.resolve();
  });
}

const tabs = () => screen.getAllByRole("tab").map((tab) => tab.textContent);

describe("AJ-E2108 a session's page holds still", () => {
  it("keeps both tabs while its token counts come and go, and the tab you're on", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ state: "none", turns: [], redacted: 0, earlier: null, stamp: null });
    render(page([conversation()]));
    // AJ-E2404: a session open in its app also has its processes.
    await waitFor(() => expect(tabs()).toEqual(["Overview", "Usage", "Processes"]));
    fireEvent.click(screen.getByRole("tab", { name: "Usage" }));
    // 0.1.5: its charts once the usage history has its turns; until then
    // (as here), its own counts.
    expect(await screen.findByTestId("session-usage-none")).toBeTruthy();
    await listIs([conversation({ tokens: { input: 1200, output: 300, cache_read: 9000, cache_write: 0, reasoning: 0 } })]);
    await waitFor(() => expect(screen.getByTestId("session-usage-total").textContent).toBe("10.5k"));
    expect(tabs()).toEqual(["Overview", "Usage", "Processes"]);
    // A refresh without the counts: the tab, and the last counts, stay.
    await listIs([conversation({ tokens: null })]);
    expect(tabs()).toEqual(["Overview", "Usage", "Processes"]);
    expect(screen.getByRole("tab", { name: "Usage" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("session-usage-total").textContent).toBe("10.5k");
  });

  it("a refresh that misses its row never flashes gone; missing for GONE_MS, it is", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    // Opens on Overview (the tab the test before left is remembered).
    localStorage.removeItem("aij.page.tab.conversation");
    const read = vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ state: "none", turns: [], redacted: 0, earlier: null, stamp: null });
    render(page([conversation()]));
    await waitFor(() => expect(screen.getByTestId("conversation-page")).toBeTruthy());
    await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    await listIs([]);
    expect(screen.getByTestId("conversation-page")).toBeTruthy();
    expect(screen.queryByTestId("conversation-gone")).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(GONE_MS - 1_000);
    });
    await listIs([conversation()]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(GONE_MS);
    });
    expect(screen.queryByTestId("conversation-gone")).toBeNull();
    // The page never remounted: its conversation was read once.
    expect(read).toHaveBeenCalledTimes(1);
    await listIs([]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(GONE_MS + 100);
    });
    expect(screen.getByTestId("conversation-gone")).toBeTruthy();
  });
});

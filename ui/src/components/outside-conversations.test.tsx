/**
 * AJ-E1529 — sessions started outside AI Juggler: in the rail under their
 * project (live ones always, then the newest few), on a page of their own
 * with "Continue in AI Juggler" and "Copy the resume command", counted as
 * running on Accounts, and in the go-to palette. Read-only otherwise (D-08).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX, ReactNode } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage, GONE_MS } from "./ConversationPage";
import { ConversationRow, railSlice, RECENT_SHOWN, stateWord, withOutsideGroups } from "./OutsideConversations";
import { GoToPalette } from "./GoToPalette";
import { ConversationsProvider } from "../lib/conversations";
import { activityWords } from "../lib/account-words";
import type { ActionHost } from "../lib/actions";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { ProjectInfo } from "../lib/bridge";

afterEach(() => {
  cleanup();
  delete (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__;
});

const NOW = 1_800_000_000_000;

function conversation(id: string, overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id,
    tool: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    project_id: "p1",
    cwd: "/w/app",
    title: `Conversation ${id}`,
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    tokens: null,
    live: false,
    session_id: null,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null, probably: false,
    background: false,
    inactive: false,
    ...overrides,
  };
}

function place(label: string, pid: number): ConversationInfo["places"][number] {
  return { app: label.split(" · ")[0]!, app_id: null, profile: null, instance: null, pid, kind: null, app_pid: null, label };
}

function withFixture(conversations: ConversationInfo[], children: ReactNode, host: ActionHost = {}): JSX.Element {
  (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__ = { conversations };
  return (
    <ConversationsProvider>
      <ActionHostProvider host={host}>{children}</ActionHostProvider>
    </ConversationsProvider>
  );
}

describe("AJ-E1529 the rail's share of outside conversations", () => {
  it("AJ-E2103: shows the open ones only; the Ended filter the newest few, and counts the rest", () => {
    const all = [
      conversation("live-1", { live: true }),
      conversation("live-2", { live: true }),
      conversation("asks", { live: true, state: "asks_you" }), // leads the rail instead
      conversation("headless", { live: true, background: true }), // behind "Also:"
      ...Array.from({ length: 6 }, (_, n) => conversation(`old-${n}`)),
      conversation("mine", { session_id: "sess-1" }), // AI Juggler's own: shown as its session
      conversation("elsewhere", { project_id: "p2" }),
    ];
    const { shown, more } = railSlice(all, "p1");
    expect(shown.map((c) => c.id)).toEqual(["live-1", "live-2"]);
    expect(more).toBe(0);
    const ended = railSlice(all, "p1", "ended");
    expect(ended.shown.map((c) => c.id)).toEqual(Array.from({ length: RECENT_SHOWN }, (_, n) => `old-${n}`));
    expect(ended.more).toBe(6 - RECENT_SHOWN);
    expect(railSlice(all, "p1", "live").shown.map((c) => c.id)).toEqual(["live-1", "live-2", "asks", "headless"]);
  });

  it("a project where only an outside session runs gets its group", () => {
    const projects = [{ id: "p1", name: "app" }, { id: "p2", name: "api" }, { id: "p3", name: "web" }] as ProjectInfo[];
    const groups = withOutsideGroups([{ id: "p1", label: "app", rollup: "", rows: [] }], [conversation("x", { project_id: "p2", live: true }), conversation("y", { project_id: "p3" })], projects);
    expect(groups.map((g) => g.id)).toEqual(["p1", "p2"]);
  });

  it("a row says it's outside AI Juggler, names its account to a screen reader, and opens its page", () => {
    const open = vi.fn();
    render(withFixture([], <ConversationRow conversation={conversation("c", { live: true })} selected={false} density="comfortable" now={NOW} onOpen={open} />));
    const row = screen.getByTestId("rail-outside");
    expect(row.textContent).toContain("outside AI Juggler");
    expect(row.textContent).toContain("on Work Max");
    // Its tool, in words: the mark beside it is hidden from screen readers
    // (QA, walk 21).
    expect(row.textContent).toContain("Claude Code on Work Max");
    // AJ-E1532: the account at a glance — its avatar, initials and colour —
    // beside the tool's mark.
    expect(row.querySelector("[data-testid='account-avatar']")?.textContent).toBe("WM");
    expect(row.querySelector("[data-tool-mark], svg")).toBeTruthy();
    fireEvent.click(row);
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ id: "c" }));
  });

  it("AJ-E2102: an open one's click goes to it; an ended one's opens its page", () => {
    const goTo = vi.fn();
    const open = vi.fn();
    const live = conversation("c", { live: true, state: "working", places: [place("Claude · Work", 1), place("Terminal", 2)] });
    render(withFixture([], <ConversationRow conversation={live} selected={false} density="comfortable" now={NOW} onOpen={open} />, { goToConversation: goTo }));
    const row = screen.getByTestId("rail-outside");
    expect(row.textContent).toContain("Working");
    // AJ-E2103: open in two places is one row that says so.
    expect(screen.getByTestId("outside-places").textContent).toContain("open in 2 places");
    expect(row.textContent).toContain("open in Claude · Work and Terminal");
    fireEvent.click(row);
    expect(goTo).toHaveBeenCalledWith(expect.objectContaining({ id: "c" }));
    expect(open).not.toHaveBeenCalled();
    cleanup();
    render(withFixture([], <ConversationRow conversation={conversation("e")} selected={false} density="comfortable" now={NOW} onOpen={open} />, { goToConversation: goTo }));
    fireEvent.click(screen.getByTestId("rail-outside"));
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ id: "e" }));
    expect(goTo).toHaveBeenCalledTimes(1);
  });

  it("states read in E21's words, the rail's own (AJ-E2104 Req.3)", () => {
    const word = (overrides: Partial<ConversationInfo>) => stateWord(conversation("s", overrides));
    expect(word({ live: true, state: "asks_you" })).toBe("Asks you");
    expect(word({ live: true, state: "done_review" })).toBe("Done, review it");
    expect(word({ live: true, state: "idle" })).toBe("Idle");
    expect(word({ live: true, state: "working" })).toBe("Working");
    expect(word({ live: true, state: "asks_you", probably: true })).toBe("Probably asks you");
    // A tool that doesn't say: Idle, never a guess at Working.
    expect(word({ live: true, state: null })).toBe("Idle");
    // Inactive is where it's listed (the View), not a state.
    expect(word({ inactive: true })).toBe("Ended");
    expect(word({})).toBe("Ended");
  });

  it("AJ-E2102: Go to it leads an open one's menu", () => {
    const host: ActionHost = { goToConversation: vi.fn(), openConversation: vi.fn() };
    render(withFixture([], <ConversationRow conversation={conversation("c", { live: true })} selected={false} density="comfortable" now={NOW} onOpen={() => {}} />, host));
    fireEvent.click(screen.getByRole("button", { name: /^Conversation actions for / }));
    const names = screen.getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);
    expect(names.slice(0, 2)).toEqual(["Go to it", "Open its page"]);
    fireEvent.click(screen.getByRole("menuitem", { name: "Go to it" }));
    expect(host.goToConversation).toHaveBeenCalledTimes(1);
  });

  it("its menu is read-only: open, continue (not while live elsewhere), copy the resume line, its project", () => {
    const host: ActionHost = { openConversation: vi.fn(), continueConversation: vi.fn(), copyConversationResume: vi.fn(), openProjectById: vi.fn() };
    render(withFixture([], <ConversationRow conversation={conversation("c", { live: true })} selected={false} density="comfortable" now={NOW} onOpen={() => {}} />, host));
    fireEvent.click(screen.getByRole("button", { name: /^Conversation actions for / }));
    const names = screen.getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);
    expect(names).toEqual(["Open its page", "Show its project", "Continue in AI Juggler", "Copy the resume command"]);
    const carry = screen.getByRole("menuitem", { name: "Continue in AI Juggler" }) as HTMLButtonElement;
    expect(carry.disabled).toBe(true);
    expect(carry.title).toBe("It's open in another session now");
    expect(names).not.toContain("Stop");
  });
});

describe("AJ-E1529 a conversation's page", () => {
  it("shows what it is and continues it here, or copies its resume line", async () => {
    const host: ActionHost = { continueConversation: vi.fn(), copyConversationResume: vi.fn() };
    render(withFixture([conversation("c")], <ConversationPage accountId="a1" id="c" now={NOW} />, host));
    await waitFor(() => expect(screen.getByTestId("conversation-page")).toBeTruthy());
    expect((await screen.findByTestId("outside-chip")).textContent).toBe("outside AI Juggler");
    expect(screen.getByTestId("conversation-page").textContent).toContain("/w/app");
    fireEvent.click(screen.getByTestId("conversation-continue"));
    fireEvent.click(screen.getByTestId("conversation-copy-resume"));
    expect(host.continueConversation).toHaveBeenCalledTimes(1);
    expect(host.copyConversationResume).toHaveBeenCalledTimes(1);
  });

  it("AJ-E2108: while it's open in its app, Go to it leads and Continue waits, saying why", async () => {
    const host: ActionHost = { goToConversation: vi.fn(), continueConversation: vi.fn() };
    render(withFixture([conversation("c", { live: true })], <ConversationPage accountId="a1" id="c" now={NOW} />, host));
    await waitFor(() => expect(screen.getByTestId("session-go-to")).toBeTruthy());
    expect(screen.queryByTestId("conversation-continue")).toBeNull();
    fireEvent.click(screen.getByTestId("session-go-to"));
    expect(host.goToConversation).toHaveBeenCalledTimes(1);
    const carry = (await screen.findByTestId("continue-carry-in")) as HTMLButtonElement;
    expect(carry.disabled).toBe(true);
    expect(screen.getByTestId("continue-on").textContent).toContain("It's open in its app now");
  });

  it("a conversation that went away says so, once it has stayed away", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(withFixture([], <ConversationPage accountId="a1" id="gone" now={NOW} />));
    expect(await screen.findByTestId("conversation-loading")).toBeTruthy();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(GONE_MS + 100);
    });
    expect(screen.getByTestId("conversation-gone")).toBeTruthy();
    vi.useRealTimers();
  });
});

describe("AJ-E1529 outside sessions count as running", () => {
  it("on an account's card", () => {
    expect(activityWords([], 2)).toBe("2 running");
    expect(activityWords([], 0)).toBe("Nothing running");
  });

  it("in the go-to palette, which opens their page", async () => {
    const openConversation = vi.fn();
    render(
      withFixture(
        [conversation("c", { live: true, title: "Fix the login test" }), conversation("ended")],
        <GoToPalette open onClose={() => {}} sessions={[]} onFocusWindow={() => {}} onOpenSession={() => {}} hotkey="Alt+Space" onHotkeyChange={async () => {}} />,
        { openConversation },
      ),
    );
    const row = await screen.findByTestId("goto-row-outside::a1::c");
    expect(row.textContent).toContain("Fix the login test");
    expect(row.textContent).toContain("outside AI Juggler");
    expect(screen.queryByTestId("goto-row-outside::a1::ended")).toBeNull();
    await act(async () => {
      fireEvent.click(row);
    });
    expect(openConversation).toHaveBeenCalledWith(expect.objectContaining({ id: "c" }));
  });
});

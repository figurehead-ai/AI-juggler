/**
 * AJ-E1529 — sessions started outside AI Juggler: in the rail under their
 * project (live ones always, then the newest few), on a page of their own
 * with "Continue in AI Juggler" and "Copy resume command", counted as
 * running on Accounts, and in the go-to palette. Read-only otherwise (D-08).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX, ReactNode } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage } from "./ConversationPage";
import { ConversationRow, railSlice, RECENT_SHOWN, withOutsideGroups } from "./OutsideConversations";
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
    ...overrides,
  };
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
  it("shows every live one, then the newest few, and counts the rest", () => {
    const all = [
      conversation("live-1", { live: true }),
      conversation("live-2", { live: true }),
      ...Array.from({ length: 6 }, (_, n) => conversation(`old-${n}`)),
      conversation("mine", { session_id: "sess-1" }), // AI Juggler's own: shown as its session
      conversation("elsewhere", { project_id: "p2" }),
    ];
    const { shown, more } = railSlice(all, "p1");
    expect(shown.map((c) => c.id)).toEqual(["live-1", "live-2", ...Array.from({ length: RECENT_SHOWN }, (_, n) => `old-${n}`)]);
    expect(more).toBe(6 - RECENT_SHOWN);
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
    // The account at a glance: its disc, initials and its own colour.
    expect(row.querySelector(".acct-disc")?.textContent).toBe("WM");
    fireEvent.click(row);
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ id: "c" }));
  });

  it("its menu is read-only: open, continue (not while live elsewhere), copy the resume line, its project", () => {
    const host: ActionHost = { openConversation: vi.fn(), continueConversation: vi.fn(), copyConversationResume: vi.fn(), openProjectById: vi.fn() };
    render(withFixture([], <ConversationRow conversation={conversation("c", { live: true })} selected={false} density="comfortable" now={NOW} onOpen={() => {}} />, host));
    fireEvent.click(screen.getByRole("button", { name: /^Conversation actions for / }));
    const names = screen.getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);
    expect(names).toEqual(["Open", "Continue in AI Juggler", "Copy resume command", "Show its project"]);
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
    expect(screen.getByTestId("outside-chip").textContent).toBe("outside AI Juggler");
    expect(screen.getByTestId("conversation-page").textContent).toContain("/w/app");
    fireEvent.click(screen.getByTestId("conversation-continue"));
    fireEvent.click(screen.getByTestId("conversation-copy-resume"));
    expect(host.continueConversation).toHaveBeenCalledTimes(1);
    expect(host.copyConversationResume).toHaveBeenCalledTimes(1);
  });

  it("while it's live elsewhere, Continue waits and says why", async () => {
    render(withFixture([conversation("c", { live: true })], <ConversationPage accountId="a1" id="c" now={NOW} />));
    await waitFor(() => expect((screen.getByTestId("conversation-continue") as HTMLButtonElement).disabled).toBe(true));
    expect(screen.getByTestId("conversation-page").textContent).toContain("open in another session now");
  });

  it("a conversation that went away says so", async () => {
    render(withFixture([], <ConversationPage accountId="a1" id="gone" now={NOW} />));
    expect(await screen.findByTestId("conversation-gone")).toBeTruthy();
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

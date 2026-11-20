/**
 * The 0.4.6 rc3h walk: on a session AI Juggler drove, with no turn yet,
 * "Earlier turns are in its conversation. Show them" opened its
 * conversation's page, which read "Started outside AI Juggler: shown here
 * to read and follow. AI Juggler never stops or changes it.", with
 * nothing to show and no way to write to it.
 *
 * - "Show them" is there only once its tool has written the conversation
 *   (it was resumed, or a turn began): before, there are no earlier turns.
 * - A conversation a session of AI Juggler's runs is AI Juggler's: its page
 *   never says it started outside, and leads to its session's page, where
 *   you write to it. One no session of AI Juggler's has still says so.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX, ReactNode } from "react";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage } from "./ConversationPage";
import { ConversationsProvider } from "../lib/conversations";
import { DrivenPanel } from "../pages/session/DrivenPanel";
import { instance } from "../test/fixtures";
import type { ActionHost } from "../lib/actions";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { RuntimeInfo, RuntimeViewInfo } from "../lib/ipc/runtime";

const NOW = 1_800_000_000_000;

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
});

const runtime: RuntimeInfo = {
  kind: "driven",
  label: "Claude Code without a terminal",
  beta: true,
  state: "idle",
  asks: [],
  can_steer: false,
  fell_back: null,
  error: null,
  limits: [],
  cut_off: null,
};

/** h-a's page: no turn held, its tool's conversation written or not. */
const view = (written: boolean): RuntimeViewInfo => ({ session_id: "sess-h", runtime, items: [], streaming: null, model: null, written });

describe("Show them", () => {
  const session = instance({ id: "sess-h", status: "running", account_id: "acc-1", conversation_id: "conv-h", project_id: null, runtime });

  function panel(written: boolean, opened: string[]): void {
    render(
      <ActionHostProvider host={{ openConversationById: (account, id) => opened.push(`${account}/${id}`) }}>
        <DrivenPanel session={session} view={view(written)} />
      </ActionHostProvider>,
    );
  }

  it("isn't offered before its tool has written the conversation", async () => {
    panel(false, []);
    await waitFor(() => expect(screen.getByTestId("driven-panel")).toBeTruthy());
    expect(screen.queryByTestId("driven-earlier")).toBeNull();
  });

  it("is, once it has, and opens that conversation", async () => {
    const opened: string[] = [];
    panel(true, opened);
    const earlier = await screen.findByTestId("driven-earlier");
    fireEvent.click(earlier.querySelector("button")!);
    expect(opened).toEqual(["acc-1/conv-h"]);
  });
});

function conversation(overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id: "conv-h",
    tool: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    project_id: "p1",
    cwd: "/w/web",
    title: "Reply with the single word lab.",
    git_branch: "main",
    started_ms: NOW - 60_000,
    last_activity_ms: NOW - 10_000,
    tokens: null,
    live: true,
    session_id: "sess-h",
    places: [],
    pid: null,
    state: "idle",
    viewed_ms: null,
    probably: false,
    background: false,
    inactive: false,
    subtask_count: null,
    ...overrides,
  };
}

function page(row: ConversationInfo, host: ActionHost): JSX.Element {
  Reflect.set(window, "__AIJ_FIXTURE__", { conversations: [row] });
  const children: ReactNode = <ConversationPage accountId={row.account_id} id={row.id} now={NOW} />;
  return (
    <ConversationsProvider>
      <ActionHostProvider host={host}>{children}</ActionHostProvider>
    </ConversationsProvider>
  );
}

describe("a conversation a session of AI Juggler's runs", () => {
  it("is never 'started outside', and leads to its session's page", async () => {
    const host: ActionHost = { openSessionById: vi.fn(), goToConversation: vi.fn() };
    render(page(conversation(), host));
    await waitFor(() => expect(screen.getByTestId("where-it-runs")).toBeTruthy());
    expect(screen.getByTestId("conversation-page").textContent).not.toContain("outside AI Juggler");
    expect(screen.queryByTestId("where-outside")).toBeNull();
    // Its door is its session's page (Go to it would only bring AI
    // Juggler forward).
    expect(screen.queryByTestId("session-go-to")).toBeNull();
    fireEvent.click(screen.getByTestId("conversation-open-session"));
    expect(host.openSessionById).toHaveBeenCalledWith("sess-h");
  });

  it("ended, it continues here as before, and its session is one click away", async () => {
    const host: ActionHost = { openSessionById: vi.fn(), continueConversation: vi.fn() };
    render(page(conversation({ live: false, state: null }), host));
    await waitFor(() => expect(screen.getByTestId("conversation-continue")).toBeTruthy());
    expect(screen.queryByTestId("where-outside")).toBeNull();
    fireEvent.click(screen.getByTestId("conversation-open-session"));
    expect(host.openSessionById).toHaveBeenCalledWith("sess-h");
  });

  it("one no session of AI Juggler's has still says it started outside", async () => {
    render(page(conversation({ session_id: null }), { openSessionById: vi.fn(), goToConversation: vi.fn() }));
    expect((await screen.findByTestId("outside-chip")).textContent).toBe("outside AI Juggler");
    expect(screen.queryByTestId("conversation-open-session")).toBeNull();
    expect(screen.getByTestId("session-go-to")).toBeTruthy();
  });
});

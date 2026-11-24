/**
 * AJ-E2326 R3: a refused move never leads to a terminal here. The Move
 * choice has its own icon (never Continue in AI Juggler's carry-in arrow);
 * a conversation a move holds offers no Continue in AI Juggler; and a
 * conversation last open in a desktop app opens there first, Continue in
 * AI Juggler second, saying it starts a terminal.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationPage } from "./ConversationPage";
import { ConversationsProvider } from "../lib/conversations";
import { IconCarryIn, IconMove } from "./icons-actions";
import { CONTINUE_IN_TERMINAL, continueWhy, heldByMove } from "../lib/resume-refusal";
import type { ActionHost } from "../lib/actions";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";

/** The preview seams the page reads (the conversations and the places). */
function seam(name: "__AIJ_FIXTURE__" | "__AIJ_PLACES__", value: unknown): void {
  Object.assign(window, { [name]: value });
}

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

const NOW = 1_800_000_000_000;

function conversation(overrides: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id: "c",
    tool: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    project_id: "p1",
    cwd: "/w/app",
    title: "Conversation c",
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    tokens: null,
    live: false,
    session_id: null,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null,
    probably: false,
    background: false,
    inactive: false,
    subtask_count: null,
    ...overrides,
  };
}

const PLACE: KnownPlaceInfo = {
  id: "app:claude-app:work",
  app_id: "claude-app",
  app: "Claude",
  instance: "work",
  profile: "Work",
  label: "Claude · Work",
  app_pid: null,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: "a1",
  seen_ms: NOW - 60_000,
  cannot_start: null,
  recent: [],
  found: true,
};

function page(row: ConversationInfo, host: ActionHost) {
  seam("__AIJ_FIXTURE__", { conversations: [row] });
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={host}>
        <ConversationPage accountId="a1" id="c" now={NOW} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("AJ-E2326 R3: the move never offers the terminal", () => {
  it("the Move choice's icon is its own, never Continue in AI Juggler's", () => {
    expect(renderToStaticMarkup(<IconMove />)).not.toBe(renderToStaticMarkup(<IconCarryIn />));
  });

  it("a conversation a move holds offers no Continue in AI Juggler, and says why", () => {
    const held = conversation({ resume_refusal: { why: "this conversation's move just stopped and it is opening again where it was; continue it there", moving: true } });
    expect(heldByMove(held)).toBe(true);
    expect(continueWhy(held)).toContain("opening again where it was");
    expect(heldByMove(conversation())).toBe(false);
    expect(heldByMove(conversation({ resume_refusal: { why: "a copy" } }))).toBe(false);
  });

  it("last open in a desktop app: Open in Claude leads, Continue in AI Juggler follows and says it starts a terminal", async () => {
    seam("__AIJ_PLACES__", { places: [PLACE], sessions: { "claude-code/c": PLACE.id } });
    page(conversation(), { continueConversation: vi.fn() });
    const open = await screen.findByTestId("conversation-open-in");
    expect(open.textContent).toBe("Open in Claude");
    const carry = screen.getByTestId("conversation-continue");
    expect(carry.title).toBe(CONTINUE_IN_TERMINAL);
    // The primary comes first, and is the open (not the terminal).
    expect(open.compareDocumentPosition(carry) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(open.className).not.toBe(carry.className);
  });

  it("never open in a desktop app: Continue in AI Juggler stays the primary", async () => {
    seam("__AIJ_PLACES__", { places: [] });
    page(conversation(), { continueConversation: vi.fn() });
    await waitFor(() => expect(screen.getByTestId("conversation-continue")).toBeTruthy());
    expect(screen.queryByTestId("conversation-open-in")).toBeNull();
    expect(screen.getByTestId("conversation-continue").title).toBe("");
  });
});

/**
 * 0.4.1 (the rc7 walk): right after "Opened in Claude app · aij-test", the
 * switched copy's page read "Ended / Not open now". A copy a switch just
 * opened reads Starting in its app until the app reports it open, and
 * only for a minute and a half at most.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { markOpening, OPENING_MS, openingIn, resetOpening } from "../../lib/just-opened";
import { fromConversation, INACTIVE_WHERE } from "./model";
import { SessionChips, whereWords } from "./SessionHead";
import { openedPage } from "../../components/switch/WhereChoice";

afterEach(() => {
  cleanup();
  resetOpening();
});

const copy = (over: Partial<ConversationInfo> = {}): ConversationInfo => ({
  id: "c-9",
  tool: "claude-code",
  account_id: "test",
  account_label: "Test",
  project_id: null,
  cwd: "/w",
  title: "Copy",
  git_branch: null,
  started_ms: 0,
  last_activity_ms: 1_000,
  tokens: null,
  live: false,
  session_id: null,
  places: [],
  pid: null,
  state: null,
  probably: false,
  viewed_ms: null,
  background: false,
  inactive: false,
  subtask_count: null,
  ...over,
});

describe("0.4.1 a copy a switch just opened", () => {
  it("reads Starting in its app, never Ended or Not open now", () => {
    markOpening("test", "c-9", "Claude app · Test");
    const model = fromConversation(copy());
    expect(model.state.words).toBe("Starting");
    expect(whereWords(model, Date.now())).toBe("Starting in Claude app · Test…");
    render(<SessionChips model={model} account={undefined} now={Date.now()} />);
    expect(screen.getByTestId("session-state-chip").textContent).toBe("Starting");
  });

  it("reads what the app reports once it's open", () => {
    markOpening("test", "c-9", "Claude app · Test");
    const model = fromConversation(copy({ live: true, state: "idle" }));
    expect(model.state.words).not.toBe("Starting");
    expect(model.opening ?? null).toBeNull();
  });

  it("rc8: once its app lists it with no turn running, it reads Inactive there, not Starting or Ended", () => {
    markOpening("test", "c-9", "Claude app · Test");
    const model = fromConversation(copy({ inactive: true }));
    expect(model.state.words).toBe("Inactive");
    expect(whereWords(model, Date.now())).toBe(INACTIVE_WHERE);
    render(<SessionChips model={model} account={undefined} now={Date.now()} />);
    expect(screen.getByTestId("session-state-chip").textContent).toBe("Inactive");
    resetOpening();
    // Any conversation its running app lists idle reads the same.
    expect(fromConversation(copy({ inactive: true, session_id: "sess-1" })).state.words).toBe("Inactive");
  });

  it("stops after a minute and a half: then what the app says stands", () => {
    markOpening("test", "c-9", "Claude app · Test", 0);
    expect(openingIn("test", "c-9", OPENING_MS)).toBe("Claude app · Test");
    expect(openingIn("test", "c-9", OPENING_MS + 1)).toBeNull();
    expect(fromConversation(copy()).state.words).not.toBe("Starting");
  });

  it("a Desktop switch marks its copy; a CLI one marks nothing", () => {
    const desktop = { place_id: "p", name: "Claude app · Test", app_id: "claude", running: false };
    const preflight = { target_tool: "claude-code", destinations: { suggested: "desktop", desktop, desktop_why_not: null, cli_why_not: null } };
    const result = { session_id: "conversation:test:c-9", conversation_key: "conversation:test:c-9" };
    expect(openedPage(result, "desktop", preflight)).toBe("conversation:test:c-9");
    expect(openingIn("test", "c-9")).toBe("Claude app · Test");
    resetOpening();
    expect(openedPage({ ...result, session_id: "sess-2" }, "cli", preflight)).toBe("sess-2");
    expect(openingIn("test", "c-9")).toBeNull();
  });
});

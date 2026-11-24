/**
 * 0.5.1 F40 (F-PL1-20): the Switch account dialog's "Now in …" names the
 * App the conversation belongs to, by its id (ADR-0032's `app`), as the
 * page's App chip does. A Claude app conversation with no window open
 * read "Now in Claude Code CLI · work" (its folder's CLI App); after a
 * move it must name the App it moved to ("Claude app · personal").
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/ipc/switch-targets";
import { focusHandover } from "../lib/handover-focus";
import { ContinueOn } from "../pages/session/ContinueOn";
import { fromConversation } from "../pages/session/model";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

const NOW = 1_800_000_000_000;

const place = (instance: string, account: string): KnownPlaceInfo => ({
  id: `app:claude-app:${instance}`,
  app_id: "claude-app",
  app: "Claude",
  instance,
  profile: null,
  label: `Claude · ${instance}`,
  app_pid: 100,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: account,
  seen_ms: NOW - 60_000,
  cannot_start: null,
  recent: [],
  found: true,
});

function conversation(overrides: Partial<ConversationInfo>): ConversationInfo {
  return {
    id: "c",
    tool: "claude-code",
    account_id: "work-folder",
    account_label: "work",
    project_id: null,
    cwd: "/w/app",
    title: "Conversation c",
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    tokens: null,
    live: false,
    inactive: true,
    session_id: null,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null,
    probably: false,
    background: false,
    subtask_count: null,
    ...overrides,
  };
}

async function nowIn(row: ConversationInfo): Promise<string> {
  Object.assign(window, { __AIJ_PLACES__: { places: [place("work", "work-folder"), place("personal", "personal-folder")] } });
  vi.spyOn(bridge, "switchTargets").mockResolvedValue([]);
  render(<ContinueOn model={fromConversation(row)} accounts={[]} displays={[]} onPick={vi.fn()} />);
  act(() => void focusHandover());
  const dialog = await screen.findByTestId("continue-dialog");
  // The places are read once the page asks: the line follows them.
  await vi.waitFor(() => expect(dialog.textContent).toMatch(/Now in Claude app/));
  return dialog.textContent ?? "";
}

describe("F40: Now in names the conversation's App by its id", () => {
  it("a Claude app conversation with no window open is the Claude app's, never its folder's CLI App", async () => {
    const text = await nowIn(conversation({ app: "app:claude-app:work", started_in: "claude-app" }));
    expect(text).toContain("Now in Claude app · work");
    expect(text).not.toContain("Claude Code CLI");
  });

  it("after a move it names the App it moved to", async () => {
    const text = await nowIn(conversation({ account_id: "personal-folder", account_label: "personal", app: "app:claude-app:personal", started_in: "claude-app" }));
    expect(text).toContain("Now in Claude app · personal");
    expect(text).not.toContain("work");
  });
});

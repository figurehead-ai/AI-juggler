/**
 * ADR-0032 (D-100): a session's App is the one the backend names, on every
 * surface. A CLI App's session whose process ran inside a desktop app (its
 * first place) is that CLI App's: the rail's Needs you row says so (0.4.3),
 * and Home's own Needs you says the same words (0.4.4), never where it ran.
 */
import { cleanup, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, parseQuery, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { buildLines } from "../lib/rail-lines";
import { itemsOf, placeWords } from "../lib/rail-items";
import { needsFor } from "../test/needs-kit";
import { account, setSeam } from "../test/fixtures";
import type { ConversationInfo, PlaceInfo } from "../lib/ipc/conversations";
import type { AccountInfo } from "../types/generated/AccountInfo";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
const ACCOUNTS: AccountInfo[] = [account({ id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" })];

/** Where it ran: inside the Claude app's Work profile. */
const DESKTOP: PlaceInfo = { app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Work", instance: "Work", pid: 500, kind: "desktop", app_pid: 41, label: "Claude · Work" };

/** It asks you; the backend says it's the CLI App's on its folder. */
const ASKING: ConversationInfo = {
  id: "t1", tool: "claude-code", account_id: "claude-work", account_label: "Work Max", project_id: null, cwd: "/w/app", title: "Fix the parser", git_branch: "feature",
  started_ms: NOW - 3_600_000, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [DESKTOP], app: "cli:claude-work",
  pid: 500, state: "asks_you", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null,
};

beforeEach(() => {
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  for (const seam of Object.keys(window).filter((found) => found.startsWith("__AIJ_"))) Reflect.deleteProperty(window, seam);
  resetRailViewCache();
});

/** The rail's Needs you row's words for it. */
function railWords(): string {
  const items = itemsOf({ sessions: [], conversations: [ASKING], projects: [], seen: {}, needOf: new Map([["c:claude-work/t1", "asks"]]) });
  const built = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: [] });
  const need = built.lines.find((line) => line.kind === "need");
  if (need?.kind !== "need") throw new Error("no Needs you row in the rail");
  return [need.item.project, ...placeWords(need.item.place)].filter(Boolean).join(" · ");
}

describe("ADR-0032 Home's Needs you names a session by the App that owns it", () => {
  it("a CLI App's session that ran in a desktop app: its CLI App, as the rail's row says", async () => {
    setSeam("__AIJ_FIXTURE__", { conversations: [ASKING] });
    setSeam("__AIJ_PLACES__", { places: [] });
    needsFor({ sessions: [], conversations: [ASKING], accounts: ACCOUNTS, thisMac: null });
    const view = render(
      <ConversationsProvider>
        <HomeScreen sessions={[]} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
      </ConversationsProvider>,
    );
    const needs = await view.findByTestId("home-needs");
    const row = await waitFor(() => within(needs).getByTestId("home-need-session"));
    expect(railWords()).toContain("Claude Code CLI · Work Max");
    expect(row.textContent).toContain(railWords());
    expect(row.textContent).not.toContain("Claude app");
  });
});

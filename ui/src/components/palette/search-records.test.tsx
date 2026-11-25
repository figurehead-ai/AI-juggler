/**
 * AJ-E3824 (D-144): Search is the one search. With nothing typed,
 * Suggested holds the asks Bot's "Or go straight to" were and Actions
 * holds Watch a session…; a Suggested entry searches its words here; the
 * records come back by kind (sessions, tasks, pools, usage), each opening
 * its page by id; a record already listed shows once; nothing found says
 * what was searched and shows Suggested again.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GoToPalette } from "../GoToPalette";
import { ActionHostProvider } from "../ItemMenu";
import { instance } from "../../test/fixtures";
import type { AppDoings } from "../../lib/palette-commands";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";
import { resetBackendHealth } from "../../lib/backend-health";

const NOW = Date.now();
const live = (id: string, title: string): InstanceInfo => instance({ id, account_label: "Team", status: "waiting_user", kind: "terminal", cwd: "/w/shop", label: title, last_activity_ms: NOW });

const record = (over: Partial<BotRowInfo> & Pick<BotRowInfo, "kind" | "id" | "title">): BotRowInfo => ({
  name_hidden: false, state: null, at_ms: null, started_ms: null, provider: null, app_id: null, app_label: null, desktop: false, project: null, project_id: null, pool: null, pool_id: null, holder: null,
  files_changed: null, branch: null, used_percent: null, window: null, resets_at_ms: null, blocked_by: null, quote: null, then_state: null, gone: false, ...over,
});

const ASKS_YOU = record({ kind: "session", id: "s-web", title: "web", state: "asks_you", provider: "claude-code", app_label: "Team", project: "shop" });
const ALSO_OPEN = record({ kind: "session", id: "s-open", title: "Open already", state: "asks_you", provider: "claude-code" });
const BLOCKED = record({ kind: "task", id: "t-export", title: "Wire the export button", state: "blocked", pool: "Release", pool_id: "pool-release" });
const TEAM = record({ kind: "account", id: "f-team", title: "Team", used_percent: 54, window: "weekly" });

function doings() {
  return {
    openScreen: vi.fn(), newSession: vi.fn(), newProject: vi.fn(), notifications: vi.fn(), shortcuts: vi.fn(), toggleRail: vi.fn(), searchHistory: vi.fn<(words: string) => void>(), fleet: false,
    openPool: vi.fn<(id: string) => void>(), openAccount: vi.fn<(id: string) => void>(), watchSession: vi.fn(),
  } satisfies AppDoings;
}

function openPalette() {
  const app = doings();
  const onOpenSession = vi.fn<(id: string) => void>();
  const onClose = vi.fn();
  const view = render(
    <ActionHostProvider host={{}}>
      <GoToPalette open onClose={onClose} sessions={[live("s-open", "Open already")]} onFocusWindow={vi.fn()} onOpenSession={onOpenSession} hotkey="" onHotkeyChange={async () => {}} app={app} />
    </ActionHostProvider>,
  );
  const input = () => document.body.querySelector<HTMLInputElement>("input[role='combobox']")!;
  const groups = () => [...document.body.querySelectorAll<HTMLElement>("ul[role='group']")].map((group) => group.getAttribute("aria-label"));
  const labels = (key: string) => [...view.getByTestId(`goto-group-${key}`).querySelectorAll("[role='option']")].map((row) => row.textContent?.replace(/↩.*$/, ""));
  return { view, input, groups, labels, app, onOpenSession, onClose };
}

beforeEach(() => {
  localStorage.clear();
  window.__AIJ_SEARCH__ = {
    "what needs me": { sessions: [ASKS_YOU], tasks: [BLOCKED], pools: [{ id: "pool-release", name: "Release", tasks: 1 }], usage: [TEAM] },
    "open already": { sessions: [ALSO_OPEN], tasks: [], pools: [], usage: [] },
  };
});
afterEach(() => {
  cleanup();
  delete window.__AIJ_SEARCH__;
  localStorage.clear();
  resetBackendHealth();
});

describe("AJ-E3824 nothing typed", () => {
  it("Suggested holds What needs me, What finished today, Runway by account and What’s running", () => {
    const { groups, labels } = openPalette();
    expect(groups()).toContain("Suggested");
    expect(labels("suggested")).toEqual(["What needs me", "What finished today", "Runway by account", "What’s running"]);
  });

  it("Watch a session… is a command under Actions, and opens Watches' form", () => {
    const { view, labels, app, onClose } = openPalette();
    expect(labels("actions")).toContain("Watch a session…");
    fireEvent.click(within(view.getByTestId("goto-group-actions")).getByText("Watch a session…"));
    expect(app.watchSession).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalled();
  });

  it("typing “watch” finds Watch a session…", () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "watch" } });
    expect(within(view.getByTestId("goto-group-actions")).getByText("Watch a session…")).toBeTruthy();
  });
});

describe("AJ-E3824 the records", () => {
  it("a Suggested entry searches its words here, and the records come back by kind, each once", async () => {
    const { view, input, onClose } = openPalette();
    fireEvent.click(within(view.getByTestId("goto-group-suggested")).getByText("What needs me"));
    expect(input().value).toBe("What needs me");
    expect(onClose).not.toHaveBeenCalled();
    await view.findByTestId("goto-group-tasks");
    expect(within(view.getByTestId("goto-group-sessions")).getByText("web")).toBeTruthy();
    expect(within(view.getByTestId("goto-group-tasks")).getByTestId("goto-record-line").textContent).toBe("Release · Blocked");
    expect(within(view.getByTestId("goto-group-pools")).getByText("1 task found")).toBeTruthy();
    expect(within(view.getByTestId("goto-group-usage")).getByTestId("goto-record-line").textContent).toBe("this week · 54 % used");
  });

  it("a session already listed shows once (its own row, not the record's)", async () => {
    const { view, input } = openPalette();
    fireEvent.change(input(), { target: { value: "open already" } });
    await new Promise((done) => setTimeout(done, 250));
    const sessions = view.getByTestId("goto-group-sessions");
    expect(sessions.querySelectorAll("[role='option']")).toHaveLength(1);
    expect(sessions.querySelector("[data-testid='goto-row-s-open']")).not.toBeNull();
  });

  it("each record opens its page by id", async () => {
    for (const [group, row, expectCall] of [
      ["sessions", "goto-row-record::session::s-web", (p: ReturnType<typeof openPalette>) => expect(p.onOpenSession).toHaveBeenCalledWith("s-web")],
      ["tasks", "goto-row-record::task::t-export", (p: ReturnType<typeof openPalette>) => expect(p.app.openPool).toHaveBeenCalledWith("pool-release")],
      ["pools", "goto-row-pool::pool-release", (p: ReturnType<typeof openPalette>) => expect(p.app.openPool).toHaveBeenCalledWith("pool-release")],
      ["usage", "goto-row-record::account::f-team", (p: ReturnType<typeof openPalette>) => expect(p.app.openAccount).toHaveBeenCalledWith("f-team")],
    ] as const) {
      const palette = openPalette();
      fireEvent.change(palette.input(), { target: { value: "what needs me" } });
      fireEvent.click(within(await palette.view.findByTestId(`goto-group-${group}`)).getByTestId(row));
      expectCall(palette);
      cleanup();
    }
  });

  it("nothing found: says so and what was searched, then Suggested", async () => {
    const { view, input, groups } = openPalette();
    fireEvent.change(input(), { target: { value: "qqqzz" } });
    expect((await view.findByTestId("goto-no-matches")).textContent).toBe("No matches for “qqqzz”.");
    expect(view.getByTestId("goto-searched").textContent).toBe("Looked in sessions, tasks, pools, usage, apps, projects, accounts and settings.");
    expect(groups()).toEqual(["Search", "Suggested"]);
    expect(input().getAttribute("aria-activedescendant")).toBe("goto-option-command__search-history");
  });
});

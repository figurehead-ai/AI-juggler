/**
 * AJ-E3307 (persona runs M2, D2) — how long each session has waited on
 * you, one rule everywhere (lib/waiting): since it began to wait (its
 * attention's start; a tool's ask, when asked), never its last report.
 * Home's Needs you says it on each row and puts the longest wait first
 * (deep/11 §4.1 "waiting 24m", sort "Waiting longest"; §6.1 oldest
 * first); the rail's row, the session's header and the bell say the same.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeNeeds } from "./HomeNeeds";
import { rowWhen } from "./row/SessionRow";
import { homeNeedsOf } from "../lib/needs-you";
import { itemsOf } from "../lib/rail-items";
import { needLine } from "../lib/notice-words";
import { askExcerpt, waitLine } from "../lib/waiting";
import { fromInstance } from "../pages/session/model";
import type { NoticeInfo } from "../lib/ipc/notices";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { instance } from "../test/fixtures";

afterEach(cleanup);

const NOW = new Date(2026, 10, 10, 8, 38).getTime();
const H = 3_600_000;

/** Waiting since `askedAgo`; its tool last reported `touchedAgo` back. */
function asking(id: string, askedAgo: number, touchedAgo: number): InstanceInfo {
  return instance({
    id, status: "waiting_user", needs_attention: "waiting_user", account_id: "a", account_label: "Work", cwd: `/Users/you/Dev/${id}`, git_branch: id,
    started_at_ms: NOW - 9 * H, last_activity_ms: NOW - touchedAgo,
    attention: { rule: "tool_asks", inputs: ["Claude needs your permission to use Bash"], since_ms: NOW - askedAgo },
  });
}

const notice = (id: string): NoticeInfo => ({
  id: 1, kind: "asks", needs: true, title: id, detail: "", project_id: null, project: null,
  target: { session_id: id, conversation_id: null, account_id: "a", project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null },
  action: null, meter: null, created_ms: NOW - 60_000, unread: true, subject: `s:${id}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
});

// Asked at 01:38 and touched a minute ago; asked at 07:38 and quiet since.
const early = asking("api", 7 * H, 60_000);
const late = asking("web", H, H);
const railItems = () => itemsOf({ sessions: [late, early], conversations: [], projects: [], seen: {} });

describe("AJ-E3307 — how long it has waited, one rule", () => {
  it("orders Home's Needs you by the longest wait, not the last report", () => {
    const needs = homeNeedsOf([notice("web"), notice("api")], railItems(), [], [late, early], null, NOW);
    expect(needs.map((need) => need.item?.session?.id)).toEqual(["api", "web"]);
  });

  it("says on each of Home's rows how long it has waited", () => {
    const needs = homeNeedsOf([notice("web"), notice("api")], railItems(), [], [late, early], null, NOW);
    render(<HomeNeeds needs={needs} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    expect(screen.getByTestId("need-wait").textContent).toBe("waiting 7 h");
    expect(screen.getByTestId("need-line").textContent).toContain("Asks you · waiting 1 h");
  });

  it("the rail's row, the session's header and the bell say the same wait", () => {
    const item = railItems().find((one) => one.session?.id === "api")!;
    expect(rowWhen(item, NOW)).toBe("waiting 7 h");
    const head = fromInstance(early);
    expect(head.state.since).toBe(NOW - 7 * H);
    // The bell's line, from the session's wait (its `quietSince`).
    expect(needLine(notice("api"), NOW, head.state.since)).toBe("waiting 7 h");
  });
});

describe("AJ-E3307 — what it asks, when its tool said (deep/11 §10)", () => {
  it("shows the ask's own words on Home's rows: `waiting {t} — “{excerpt}”`", () => {
    const needs = homeNeedsOf([notice("web"), notice("api")], railItems(), [], [late, early], null, NOW);
    render(<HomeNeeds needs={needs} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    expect(screen.getByTestId("need-asks").textContent).toBe("“Claude needs your permission to use Bash”");
    expect(screen.getByTestId("need-line").textContent).toContain("waiting 1 h — “Claude needs your permission to use Bash”");
  });

  it("says nothing it wasn't told — \"open it to see\" — and keeps an ask to its first line", () => {
    expect(askExcerpt({ attention: { rule: "waiting_prompt", inputs: ["idle 3m"] } })).toBeNull();
    // A hook's Notification alone: the type, never the words.
    expect(askExcerpt({ runtime: { asks: [{ summary: "It waits for your answer" }] } })).toBeNull();
    expect(askExcerpt({ runtime: { asks: [{ summary: "Which database?\nPostgres or SQLite" }] } })).toBe("Which database?");
    expect(askExcerpt({ attention: null, runtime: { asks: [{ summary: "Bash: " + "x".repeat(200) }] } })!.length).toBe(90);
    const unsaid = { ...asking("cli", 2 * H, 2 * H), attention: { rule: "waiting_prompt", inputs: [], since_ms: NOW - 2 * H } };
    expect(waitLine({ need: "asks", session: unsaid }, NOW)).toBe("waiting 2 h · open it to see");
    expect(waitLine({ need: "stuck", session: unsaid }, NOW)).toBe("waiting 2 h");
  });
});


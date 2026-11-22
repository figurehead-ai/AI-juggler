/**
 * D-118's mock (Home round 3): Needs you now gives each need its own card
 * and lists them in the rail's order, the one waiting longest first: a
 * session that asked 12 min ago before an agent's question of 5 min (the
 * rail listed "Migrate the billing webhooks" first; Home listed the
 * question first). The first card's primary is the one filled button.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeNeeds, needsOrder } from "./HomeNeeds";
import { homeNeedsOf } from "../lib/needs-you";
import { itemsOf } from "../lib/rail-items";
import type { MessageNeedInfo } from "../lib/ipc/messages";
import type { NoticeInfo } from "../lib/ipc/notices";
import { instance } from "../test/fixtures";

afterEach(cleanup);

const NOW = new Date(2026, 10, 18, 10, 13).getTime();
const MIN = 60_000;

const asking = instance({
  id: "billing", status: "waiting_user", needs_attention: "waiting_user", account_id: "a", account_label: "Work Max", cwd: "/Users/you/Dev/payments-api",
  started_at_ms: NOW - 60 * MIN, last_activity_ms: NOW - MIN, attention: { rule: "waiting_prompt", inputs: [], since_ms: NOW - 12 * MIN },
});
const notice: NoticeInfo = {
  id: 1, kind: "asks", needs: true, title: "billing", detail: "", project_id: null, project: null,
  target: { session_id: "billing", conversation_id: null, account_id: "a", project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null },
  action: null, meter: null, created_ms: NOW - MIN, unread: true, subject: "s:billing", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
};
const question = (ago: number): MessageNeedInfo => ({
  key: "question:m-q1", kind: "question", pool_id: "pool-release", pool_name: "Release pool", thread_id: "thr-export", message_id: "m-q1", session_id: null, ask_id: null, offer_always: false,
  title: "builder-1 asks you", detail: "Export flow", preview: "Use the v2 export API, or keep v1 for this release?", sender: "builder-1", thread: "Export flow", at_ms: NOW - ago,
});
const needs = () => homeNeedsOf([notice], itemsOf({ sessions: [asking], conversations: [], projects: [], seen: {} }), [], [asking], null, NOW);

describe("Needs you now: one card each, the one waiting longest first", () => {
  it("a session waiting 12 min comes before an agent's question of 5 min, and after one of 20 min", () => {
    expect(needsOrder(needs(), [question(5 * MIN)], []).map((entry) => entry.kind)).toEqual(["need", "message"]);
    expect(needsOrder(needs(), [question(20 * MIN)], []).map((entry) => entry.kind)).toEqual(["message", "need"]);
  });

  it("draws each as its own card; the first holds the one filled primary", () => {
    render(<HomeNeeds needs={needs()} messages={[question(5 * MIN)]} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    const cards = [...screen.getByTestId("home-needs").querySelectorAll<HTMLElement>("[data-home-item]")];
    expect(cards.map((card) => card.dataset.testid)).toEqual(["home-need-session", "message-need-question"]);
    expect(cards[0]!.dataset.first).toBe("true");
    expect(cards[0]!.className).toContain("rounded-xl");
    expect(cards[1]!.className).toContain("rounded-xl");
    expect(cards[0]!.querySelector("button[aria-keyshortcuts='Enter']")?.textContent).toContain("Open its page");
    expect(cards[1]!.querySelector("button[aria-keyshortcuts='Enter']")).toBeNull();
    // What the session asks, said honestly when its tool didn't say.
    expect(cards[0]!.querySelector("[data-testid='need-asks']")?.textContent).toBe(" — open it to see what it asks");
    expect(cards[1]!.querySelector("[data-testid='need-asks']")?.textContent).toBe(": “Use the v2 export API, or keep v1 for this release?”");
  });
});

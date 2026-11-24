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
import { primariesIn } from "./primitives";
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

describe("0.5.1 critique (93-messages): one filled button in Needs you", () => {
  const held = (key: string, ago: number): MessageNeedInfo => ({
    ...question(ago), key, kind: "ask", ask_id: key, title: "«Reviewer» wants to message Builder", detail: "above what the pool lets agents do", preview: null,
  });

  it("an agent's held message above the cards takes it; the cards and a second held one are tinted", () => {
    render(<HomeNeeds needs={needs()} messages={[held("ask:1", 30 * MIN), held("ask:2", 20 * MIN), question(5 * MIN)]} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    const zone = screen.getByTestId("home-needs");
    const filled = primariesIn(zone);
    expect(filled).toHaveLength(1);
    expect(filled[0]!.closest("[data-testid='ask-card']")).toBe(zone.querySelectorAll("[data-testid='ask-card']")[0]);
    expect(zone.querySelector("[data-first='true']")).toBeNull();
  });

  it("with no held message, the first card keeps it", () => {
    render(<HomeNeeds needs={needs()} messages={[question(5 * MIN)]} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    const filled = primariesIn(screen.getByTestId("home-needs"));
    expect(filled).toHaveLength(1);
    expect(filled[0]!.textContent).toContain("Open its page");
  });
});

describe("0.5.1 F80: a stopped session stays pinned in Needs you now", () => {
  // Home's own list comes in its kinds' order (sessions that ask or look
  // stuck, limits, then what failed); the pinned three must be the
  // loudest, as the rail lists them: L3 "Stopped with an error" first.
  const own = (key: string, kind: "session" | "limit" | "failed", level: 2 | 3, ago: number) => ({
    key, kind, title: key, detail: "", severe: kind !== "session", level, since: NOW - ago,
  });
  const homeOwn = () => [
    own("asks", "session", 2, 12 * MIN),
    own("stuck", "session", 2, 3 * 60 * MIN),
    own("limit", "limit", 2, 4 * 60 * MIN),
    own("failed-a", "failed", 3, 10 * 60 * MIN),
    own("failed-b", "failed", 3, 60 * MIN),
  ];

  it("orders Home's own needs by level, then the longest wait", () => {
    expect(needsOrder(homeOwn(), [], []).map((entry) => entry.key)).toEqual(["failed-a", "failed-b", "limit", "stuck", "asks"]);
  });

  it("pins both stopped sessions; the folded ones are the quieter", () => {
    render(<HomeNeeds needs={homeOwn()} now={NOW} handlers={{ onOpenSession: vi.fn() }} />);
    const zone = screen.getByTestId("home-needs");
    const pinned = [...zone.querySelectorAll<HTMLElement>("[data-home-item]")].map((card) => card.dataset.needKey);
    expect(pinned).toEqual(["failed-a", "failed-b", "limit"]);
    expect(zone.querySelector("[data-kind='failed'][data-first='true']")).not.toBeNull();
    expect(screen.getByTestId("home-needs-more").textContent).toBe("+2 more need you");
  });
});

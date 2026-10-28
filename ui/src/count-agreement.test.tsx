/**
 * AJ-E2502 (§1.1, the approved design of 2026-10-27), D-49: one count.
 * The backend works Needs you out (here the preview's stand-in for it);
 * the bell, Home and the rail all read that one list and its one count,
 * the model's own needs (an account almost out of room) included. The app
 * badge and the menu-bar item read the same model
 * (`crates/juggler-app/tests/notification_model.rs`).
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { resetNoticeStandIn, type NoticeSeed } from "./lib/ipc/notices";
import { resetNotices } from "./lib/notices-store";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

const w = () => window as unknown as { __AIJ_NOTICES__?: NoticeSeed };
const T0 = Date.now();

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
  resetNoticeStandIn();
  resetNotices();
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_NOTICES__;
  localStorage.clear();
  sessionStorage.clear();
});

/** Home's heading's count: "2 things need you", or "3 sessions and 2
 * more things need you" (both parts). */
const count = (heading: string) => [...heading.matchAll(/(\d+) (?:sessions?|more|things?)/g)].reduce((sum, match) => sum + Number(match[1]), 0);

it("the bell, Home and the rail show one Needs you count, the model's own needs included", async () => {
  w().__AIJ_NOTICES__ = {
    app: [
      {
        id: 9,
        kind: "quota",
        needs: true,
        title: "Work Max is almost out of room",
        detail: "7% of its 5-hour window left",
        project_id: null,
        project: null,
        target: { session_id: null, conversation_id: null, account_id: "claude-max", project_id: null, page: null, task_id: null, pool_id: null },
        facts: { window: "5-hour", resets_at_ms: T0 + 3_600_000, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Work Max" },
        action: "hand-over",
        meter: 7,
        created_ms: T0,
        unread: true,
        subject: "quota:claude-max",
        snoozed: false,
        snoozed_until_ms: null,
        muted: false,
        dismissed: false,
      },
    ],
  };
  const { default: App } = await import("./App");
  const view = render(<App />);
  const heading = await view.findByTestId("home-heading");
  await waitFor(() => expect(view.getByTestId("home-needs").textContent).toContain("Work Max is almost out of room"), { timeout: 4000 });
  // Home's heading counts the rows it shows (the 0.2 walk, F4), read
  // again when the backend's count moves: the two meet.
  await waitFor(() => expect(view.getByTestId("bell-count").textContent).toBe(String(count(heading.textContent ?? ""))), { timeout: 4000 });
  const home = count(heading.textContent ?? "");
  expect(home).toBeGreaterThanOrEqual(2);
  expect(view.getByTestId("notification-bell").getAttribute("aria-label")).toContain(`${home} things need you`);
  await waitFor(() => expect(view.getByTestId("rail-needs-you").textContent).toContain(String(home)), { timeout: 4000 });
  expect(view.getAllByTestId("rail-notice").some((row) => row.textContent?.includes("Work Max is almost out of room"))).toBe(true);
});

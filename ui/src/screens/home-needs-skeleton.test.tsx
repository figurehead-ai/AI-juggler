/**
 * D-103.6 (P9's Home part): Home's Needs you holds its place with a
 * skeleton until the needs have loaded, so nothing under it moves when
 * they arrive; with none, the skeleton goes and nothing takes its place.
 * The heading never says "All clear" before they're known.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { needsHeading } from "../lib/home-needs";
import { resetNoticeStandIn, standInNeeds } from "../lib/ipc/notices";
import { applySummary, resetNotices } from "../lib/notices-store";
import { forgetPaged } from "../lib/paged";
import { previewNeedsOf } from "../preview/needs";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { account, instance, setSeam } from "../test/fixtures";

const NOW = new Date(2026, 10, 12, 15, 0, 0).getTime();
const ACCOUNTS: AccountInfo[] = [account({ id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" })];
const ASKING = instance({ id: "s-ask", account_id: "claude-work", account_label: "Work Max", status: "waiting_user", started_at_ms: NOW - 600_000, last_activity_ms: NOW - 60_000 });

beforeEach(() => {
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
  // A cold start: nothing worked out yet (the preview's first tick to come).
  resetNoticeStandIn();
  resetNotices();
  forgetPaged();
  setSeam("__AIJ_FIXTURE__", { conversations: [] });
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  for (const seam of Object.keys(window).filter((found) => found.startsWith("__AIJ_"))) Reflect.deleteProperty(window, seam);
  resetRailViewCache();
});

function home(sessions: InstanceInfo[]) {
  return render(
    <ConversationsProvider>
      <HomeScreen sessions={sessions} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
    </ConversationsProvider>,
  );
}

/** The backend's tick (here the preview's stand-in) answers. */
function needsArrive(sessions: InstanceInfo[]): void {
  act(() => applySummary(standInNeeds(previewNeedsOf({ sessions, conversations: [], accounts: ACCOUNTS, thisMac: null }))));
}

describe("D-103.6 Home's Needs you while the needs load", () => {
  it("shows a skeleton in its place, then the card in the same place", async () => {
    const view = home([ASKING]);
    const skeleton = view.getByTestId("placeholder-need");
    expect(skeleton.getAttribute("aria-label")).toBe("Loading what needs you");
    expect(view.queryByTestId("home-needs")).toBeNull();
    // Never "All clear" before they're known.
    expect(view.getByTestId("home-heading").textContent).toBe("Checking what needs you…");
    needsArrive([ASKING]);
    await waitFor(() => expect(view.getByTestId("home-needs")).toBeTruthy());
    expect(view.queryByTestId("placeholder-need")).toBeNull();
    expect(view.getByTestId("home-heading").textContent).toBe("1 session needs you");
  });

  it("is gone when there are none", async () => {
    const view = home([]);
    expect(view.getByTestId("placeholder-need")).toBeTruthy();
    needsArrive([]);
    await waitFor(() => expect(view.queryByTestId("placeholder-need")).toBeNull());
    expect(view.queryByTestId("home-needs")).toBeNull();
    expect(view.getByTestId("home-heading").textContent).not.toBe("Checking what needs you…");
  });

  it("the heading counts what is known, and is calm only once the needs are", () => {
    const calm = { runOut: [], running: false, anything: true };
    expect(needsHeading(0, { ...calm, known: false })).toBe("Checking what needs you…");
    expect(needsHeading({ sessions: 1, others: 0 }, { ...calm, known: false })).toBe("1 session needs you");
    expect(needsHeading(0, { ...calm, known: true })).toBe("All clear");
  });
});

/**
 * The 0.4.6 RC walk K2 (F-K2-7a): Home's "Pick up where you left off"
 * counted only the sessions the app holds (at most 200 ended in 7 days),
 * so "Show all 200" and "Mark all 200 reviewed" said "all" in a lab with
 * 1 000 to review. Its count is the backend's (`sessions_counts` over
 * the `to_review` list in the window), Show all pages every one from that
 * list (never all at once), and Mark all marks every one by that list,
 * its Undo putting that batch back.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Page, PageRequest } from "../lib/paged";
import type { SessionRowInfo, SessionsQuery } from "../lib/ipc/sessions";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { account, instance } from "../test/fixtures";

vi.mock("../lib/bridge", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/bridge")>()),
  hasLiveBackend: () => true,
}));

const NOW = new Date(2026, 10, 16, 15, 0, 0).getTime();
const TO_REVIEW = 1000;
const ended = (n: number): InstanceInfo => instance({ id: `e-${n}`, account_id: "claude-work", account_label: "Work Max", status: "finished", started_at_ms: NOW - (n + 2) * 300_000, last_activity_ms: NOW - (n + 1) * 300_000 });
const counted: SessionsQuery[] = [];
const paged: { query: SessionsQuery; page: PageRequest }[] = [];
const marked: SessionsQuery[] = [];
const undone: number[] = [];
let countNow: () => Promise<number> = async () => TO_REVIEW;

vi.mock("../lib/ipc/sessions", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/sessions")>()),
  sessionsCounts: async (query: SessionsQuery) => {
    counted.push(query);
    const all = await countNow();
    return { all, open: 0, ended: all, archived: 0 };
  },
  sessionsPage: async (query: SessionsQuery, page: PageRequest): Promise<Page<SessionRowInfo>> => {
    paged.push({ query, page });
    const start = page.cursor ? Number(page.cursor) : 0;
    const end = Math.min(TO_REVIEW, start + (page.limit ?? 50));
    const items = Array.from({ length: end - start }, (_, i): SessionRowInfo => ({ key: `s/e-${start + i}`, session: ended(start + i), conversation: null, snippet: null }));
    return { items, next_cursor: end < TO_REVIEW ? String(end) : null, total: TO_REVIEW };
  },
}));
vi.mock("../lib/ipc/review", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/review")>()),
  sessionsMarkReviewed: async (query: SessionsQuery) => {
    marked.push(query);
    return { count: TO_REVIEW, at_ms: 4242 };
  },
  sessionsUnmarkReviewed: async (at: number) => {
    undone.push(at);
    return TO_REVIEW;
  },
}));

const { HomeScreen } = await import("./Home");
const { ConversationsProvider } = await import("../lib/conversations");
const { forgetPaged } = await import("../lib/paged");
const { forgetReviewCount } = await import("../lib/home-review");
const { resetToasts, toastEntries, undoToast } = await import("../components/Toast");

const ACCOUNTS: AccountInfo[] = [account({ id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" })];

beforeEach(() => {
  countNow = async () => TO_REVIEW;
});
afterEach(() => {
  cleanup();
  forgetPaged();
  forgetReviewCount();
  resetToasts();
  counted.length = 0;
  paged.length = 0;
  marked.length = 0;
  undone.length = 0;
});

/** The app holds 5 of them (its working set). */
function home(onChanged = vi.fn()) {
  return render(
    <ConversationsProvider>
      <HomeScreen sessions={Array.from({ length: 5 }, (_, n) => ended(n))} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} onChanged={onChanged} />
    </ConversationsProvider>,
  );
}

describe("Pick up where you left off counts every one (F-K2-7a)", () => {
  it("says every one to review, never only those the app holds, and says no number before it's counted", async () => {
    let answer: (n: number) => void = () => {};
    countNow = () => new Promise((resolve) => (answer = resolve));
    home();
    // The held rows at once; no number claimed yet.
    expect(screen.getAllByTestId("home-recent-item")).toHaveLength(3);
    expect(screen.getByTestId("home-recent-all").textContent).toBe("Show all");
    expect(screen.getByTestId("home-review-all").textContent).toBe("Mark all reviewed");
    await act(async () => answer(TO_REVIEW));
    expect(screen.getByTestId("home-recent-all").textContent).toBe("Show all 1000");
    expect(screen.getByTestId("home-review-all").textContent).toBe("Mark all 1000 reviewed");
    // The backend's list: AI Juggler's own to review, in the 7 days up to now.
    expect(counted[0]).toMatchObject({ which: "to_review" });
    expect(Math.abs(Date.now() - 7 * 86_400_000 - counted[0]!.since_ms!)).toBeLessThan(60_000);
  });

  it("Show all pages every one from the backend, never all at once", async () => {
    home();
    await waitFor(() => expect(screen.getByTestId("home-recent-all").textContent).toBe("Show all 1000"));
    fireEvent.click(screen.getByTestId("home-recent-all"));
    await waitFor(() => expect(screen.getByTestId("home-recent-paged")).toBeTruthy());
    expect(paged[0]!.query).toMatchObject({ which: "to_review" });
    expect(paged[0]!.page.cursor).toBeNull();
    // A page read, only those in view drawn, the rest on scrolling.
    expect(screen.getAllByTestId("home-recent-item").length).toBeLessThanOrEqual(50);
    expect(screen.getByTestId("home-recent-paged-more").textContent).toContain("950 more");
  });

  it("Mark all marks every one by the backend's list, and Undo puts that batch back", async () => {
    const onChanged = vi.fn();
    home(onChanged);
    await waitFor(() => expect(screen.getByTestId("home-review-all").textContent).toBe("Mark all 1000 reviewed"));
    await act(async () => fireEvent.click(screen.getByTestId("home-review-all")));
    expect(marked).toHaveLength(1);
    expect(marked[0]).toMatchObject({ which: "to_review" });
    await waitFor(() => expect(toastEntries().map((toast) => toast.options.message)).toContain("Marked 1000 reviewed."));
    await act(async () => undoToast("reviewed:all"));
    expect(undone).toEqual([4242]);
    expect(onChanged).toHaveBeenCalledTimes(2);
  });
});

/**
 * AJ-E2105 R2: Home's Needs you holds the agents' kinds too, each with
 * its own primary: an agent's message waiting (Read), an agent's ask
 * (Allow once), a chain paused for you (Deliver), and a prepared launch
 * (Start session) — and the heading counts them.
 */
import { cleanup, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { resetPreviewMessages } from "../lib/ipc/messages";
import { resetPreviewTasks } from "../lib/ipc/tasks";
import { needsFor } from "../test/needs-kit";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { account, setSeam } from "../test/fixtures";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
const ACCOUNTS: AccountInfo[] = [
  account({ id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in" }),
  account({ id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in" }),
];

const WHEN = NOW - 60_000;
const msgNeed = (key: string, kind: string, title: string, ask: string | null) => ({ key, kind, pool_id: "pool-team", pool_name: "Checkout team", thread_id: "thr-1", message_id: `m-${key}`, session_id: null, ask_id: ask, offer_always: false, title, detail: "Checkout team", preview: "", at_ms: WHEN });

beforeEach(() => {
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
  resetPreviewMessages();
  resetPreviewTasks();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  for (const seam of Object.keys(window).filter((found) => found.startsWith("__AIJ_"))) Reflect.deleteProperty(window, seam);
  resetRailViewCache();
});

it("holds the agents' messages, asks, paused chains and prepared launches, each with its own primary, and counts them", async () => {
  setSeam("__AIJ_FIXTURE__", { conversations: [] });
  const messages = [
    msgNeed("ask:ask-1", "ask", "Builder wants to message Planner", "ask-1"),
    msgNeed("deliver:ask-2", "deliver", "A reply would wake Reviewer", "ask-2"),
    msgNeed("waiting:m-3", "waiting", "Planner wrote to you", null),
  ];
  const tasks = [{ id: "tneed-2", kind: "launch", pool_id: "pool-team", pool_name: "Checkout team", task_id: "aj-7f3c", task_title: "Build the orders API", title: "«Planner» asks for a new session for «Build the orders API»", detail: "«Use the Work account»", offer_always: false, project_id: null, at_ms: WHEN }];
  setSeam("__AIJ_MESSAGES__", { threads: [], messages: {}, needs: messages });
  setSeam("__AIJ_TASKS__", { needs: tasks });
  // The backend counts them too (Needs you is the backend's, D-49).
  needsFor({ sessions: [], conversations: [], accounts: ACCOUNTS, thisMac: null, messages, tasks });
  const view = render(
    <ConversationsProvider>
      <HomeScreen sessions={[]} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
    </ConversationsProvider>,
  );
  const needs = await view.findByTestId("home-needs");
  await waitFor(() => expect(within(needs).getByTestId("task-need-launch")).toBeTruthy());
  // An agent's ask: Allow once; a paused chain: Deliver (its card says what waits).
  await waitFor(() => expect(within(needs).getAllByText(/An agent asks to send|A chain of replies paused for you/)).toHaveLength(2));
  expect(within(needs).getAllByRole("button", { name: /^Allow once/ }).length).toBeGreaterThan(0);
  // A message waiting: Read.
  expect(within(within(needs).getByTestId("message-need-waiting")).getByRole("button", { name: /^Read/ })).toBeTruthy();
  // A prepared launch: you start it.
  expect(within(within(needs).getByTestId("task-need-launch")).getByRole("button", { name: /Start/ })).toBeTruthy();
  expect(view.getByTestId("home-heading").textContent).toMatch(/^4 /);
});

/** The rc27 pool pass: Home said 16 while the bell said 15. Home read the
 * agents' messages again at once on each change and added them to the
 * backend's needs; the bell's count moves on the backend's tick. Home
 * lists what the bell counts: a message the backend hasn't listed yet
 * waits for it. */
it("lists and counts only the messages the bell counts, while a newer one waits for the backend", async () => {
  setSeam("__AIJ_FIXTURE__", { conversations: [] });
  const counted = msgNeed("waiting:m-1", "waiting", "Planner wrote to you", null);
  const fresh = msgNeed("waiting:m-2", "waiting", "Builder wrote to you", null);
  setSeam("__AIJ_MESSAGES__", { threads: [], messages: {}, needs: [counted, fresh] });
  // The backend has counted the first so far.
  needsFor({ sessions: [], conversations: [], accounts: ACCOUNTS, thisMac: null, messages: [counted] });
  const view = render(
    <ConversationsProvider>
      <HomeScreen sessions={[]} accounts={ACCOUNTS} projects={[]} now={NOW} onOpenSession={vi.fn()} onNewSession={vi.fn()} onAddAccounts={vi.fn()} />
    </ConversationsProvider>,
  );
  await waitFor(() => expect(view.getAllByTestId("message-need-waiting")).toHaveLength(1));
  expect(view.getByTestId("home-heading").textContent).toBe("1 thing needs you");
  expect(view.getByTestId("home-needs").textContent).toContain("Planner wrote to you");
  expect(view.getByTestId("home-needs").textContent).not.toContain("Builder wrote to you");
});

/**
 * 0.1.5 review — time to first content on 10 000 sessions. History, as it
 * was, built and filtered every session in the window and drew them;
 * paged, it draws its frame and a skeleton in the first render, its
 * remembered rows at once when opened again, and its first page as soon
 * as the backend answers. Each prints a `[perf]` line (jsdom: slower
 * than the app's WebKit; the budgets are the app's: 100 ms to the
 * skeleton, 300 ms to remembered rows, 1 s to fresh ones).
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstanceInfo } from "../lib/bridge";
import type { Page, PageRequest } from "../lib/paged";
import type { SessionRowInfo, SessionsQuery } from "../lib/ipc/sessions";

let live = false;
vi.mock("../lib/bridge", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/bridge")>()),
  hasLiveBackend: () => live,
}));

const COUNT = 10_000;
const T = new Date(2026, 9, 1, 12).getTime();
const sessions: InstanceInfo[] = Array.from(
  { length: COUNT },
  (_, n) =>
    ({
      id: `s${n}`,
      status: "finished",
      provider: "claude-code",
      account_id: `acc${n % 5}`,
      account_label: `Account ${n % 5}`,
      title: `Session ${n}`,
      model: "opus",
      git_branch: "main",
      started_at_ms: T - n * 60_000,
      last_activity_ms: T - n * 60_000,
      project_id: `p${n % 20}`,
      quota: null,
      tokens: null,
      attention: null,
      needs_attention: null,
      machine_id: "this-mac",
      kind: "terminal",
      cwd: "/w",
    }) as InstanceInfo,
);
const rows: SessionRowInfo[] = sessions.map((session) => ({ key: `s/${session.id}`, session, conversation: null, snippet: null }));
let answer: (() => void) | null = null;

vi.mock("../lib/ipc/sessions", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/sessions")>()),
  sessionsPage: (_query: SessionsQuery, page: PageRequest): Promise<Page<SessionRowInfo>> =>
    new Promise((resolve) => {
      const start = page.cursor ? Number(page.cursor) : 0;
      const items = rows.slice(start, start + (page.limit ?? 50));
      answer = () => resolve({ items, next_cursor: String(start + items.length), total: COUNT });
    }),
  sessionsCounts: async () => ({ all: COUNT, open: 0, ended: COUNT, archived: 0 }),
}));

const { History } = await import("./History");
const { forgetPaged } = await import("../lib/paged");
const projects = Array.from({ length: 20 }, (_, n) => ({ id: `p${n}`, name: `project-${n}` })) as never[];

afterEach(() => {
  cleanup();
  forgetPaged();
  answer = null;
});

const timed = (what: string, run: () => void): number => {
  const started = performance.now();
  run();
  const took = performance.now() - started;
  console.info(`[perf] ${what}: ${took.toFixed(1)} ms`);
  return took;
};

describe("History on 10 000 sessions: time to first content", () => {
  it("before: every session built and filtered in the window", () => {
    live = false;
    let container: HTMLElement | null = null;
    timed("before: History's first render, every session in the window", () => {
      container = render(<History sessions={sessions} projects={projects} onChanged={() => {}} />).container;
    });
    expect(container!.querySelectorAll("[data-testid='page-session-item']").length).toBeGreaterThan(0);
  });

  it("now: the skeleton in the first render, remembered rows at once, fresh ones when the backend answers", async () => {
    live = true;
    let first: ReturnType<typeof render> | null = null;
    const skeleton = timed("now: History's frame and skeleton (first render)", () => {
      first = render(<History sessions={[]} projects={projects} onChanged={() => {}} />);
    });
    expect(first!.container.querySelector("[data-testid='page-sessions-list-skeleton']")).not.toBeNull();
    expect(skeleton).toBeLessThan(500);
    const asked = performance.now();
    answer!();
    await waitFor(() => expect(first!.container.querySelectorAll("[data-testid='page-session-item']").length).toBeGreaterThan(0));
    const fresh = performance.now() - asked;
    console.info(`[perf] now: History's first page drawn after the backend answered: ${fresh.toFixed(1)} ms`);
    expect(fresh).toBeLessThan(1000);
    first!.unmount();

    let again: ReturnType<typeof render> | null = null;
    const cached = timed("now: History opened again (remembered rows, first render)", () => {
      again = render(<History sessions={[]} projects={projects} onChanged={() => {}} />);
    });
    expect(again!.container.querySelector("[data-testid='page-sessions-list-skeleton']")).toBeNull();
    expect(again!.container.querySelectorAll("[data-testid='page-session-item']").length).toBeGreaterThan(0);
    expect(cached).toBeLessThan(1000);
  });
});

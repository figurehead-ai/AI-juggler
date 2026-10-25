/**
 * 0.1.5 review — History reads a page at a time from the backend: a
 * skeleton at once, the first 50 rows, the search sent to the backend
 * (its transcript words in context come back with the rows), and the
 * list shown at once from memory when History opens again.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstanceInfo } from "../lib/bridge";
import type { Page, PageRequest } from "../lib/paged";
import type { SessionRowInfo, SessionsQuery } from "../lib/ipc/sessions";

vi.mock("../lib/bridge", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/bridge")>()),
  hasLiveBackend: () => true,
}));

const asked: { query: SessionsQuery; page: PageRequest }[] = [];
const T = new Date(2026, 9, 1, 12).getTime();
const session = (n: number): InstanceInfo =>
  ({
    id: `s${n}`,
    status: "finished",
    provider: "claude-code",
    account_id: "acc",
    account_label: "Work",
    title: `Session ${n}`,
    started_at_ms: T - n * 60_000,
    last_activity_ms: T - n * 60_000,
    project_id: null,
    quota: null,
    tokens: null,
    attention: null,
    needs_attention: null,
    machine_id: "this-mac",
    kind: "terminal",
    cwd: "/w",
  }) as InstanceInfo;
const rows = Array.from({ length: 500 }, (_, n): SessionRowInfo => ({ key: `s/s${n}`, session: session(n), conversation: null, snippet: null }));

vi.mock("../lib/ipc/sessions", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/sessions")>()),
  sessionsPage: async (query: SessionsQuery, page: PageRequest): Promise<Page<SessionRowInfo>> => {
    asked.push({ query, page });
    if (query.text === "stripe") return { items: [{ ...rows[7]!, snippet: "…the flaky Stripe webhook…" }], next_cursor: null, total: 1 };
    const start = page.cursor ? Number(page.cursor) : 0;
    const items = rows.slice(start, start + (page.limit ?? 50));
    return { items, next_cursor: start + items.length < rows.length ? String(start + items.length) : null, total: rows.length };
  },
  sessionsCounts: async () => ({ all: 520, open: 0, ended: 500, archived: 20 }),
  sessionsGet: async (ids: string[]) => rows.filter((row) => ids.includes(row.session!.id)).map((row) => row.session!),
}));

const { History } = await import("./History");
const { forgetPaged } = await import("../lib/paged");

afterEach(() => {
  cleanup();
  forgetPaged();
  asked.length = 0;
});

const items = (container: HTMLElement) => container.querySelectorAll("[data-testid='page-session-item']");

describe("History, a page at a time", () => {
  it("shows a skeleton at once, then the first page, never every session", async () => {
    const { container } = render(<History sessions={[]} projects={[]} onChanged={() => {}} />);
    expect(container.querySelector("[data-testid='page-sessions-list-skeleton']")).not.toBeNull();
    await waitFor(() => expect(items(container).length).toBeGreaterThan(0));
    expect(asked[0]!.query.which).toBe("ended");
    expect(asked[0]!.page).toEqual({ cursor: null, limit: 50 });
    // 500 rows in the list; only those in view are drawn.
    expect(items(container).length).toBeLessThan(50);
    expect(container.querySelector("[data-testid='page-sessions-count']")!.textContent).toContain("500 sessions");
    await waitFor(() => expect(container.querySelector("[data-testid='history-tab-archived']")!.textContent).toBe("Archived (20)"));
  });

  it("sends the search to the backend once typing pauses, with its words in context", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { container } = render(<History sessions={[]} projects={[]} onChanged={() => {}} />);
      await waitFor(() => expect(items(container).length).toBeGreaterThan(0));
      fireEvent.change(container.querySelector("[data-testid='history-search']")!, { target: { value: "strip" } });
      fireEvent.change(container.querySelector("[data-testid='history-search']")!, { target: { value: "stripe" } });
      expect(asked.some((call) => call.query.text === "strip")).toBe(false);
      await act(async () => {
        vi.advanceTimersByTime(300);
      });
      await waitFor(() => expect(items(container)).toHaveLength(1));
      expect(asked.filter((call) => call.query.text?.startsWith("strip")).map((call) => call.query.text)).toEqual(["stripe"]);
      expect(container.querySelector("[data-testid='history-snippet']")!.textContent).toContain("Stripe webhook");
    } finally {
      vi.useRealTimers();
    }
  });

  it("opens again at once from memory, and reads it again behind it", async () => {
    const first = render(<History sessions={[]} projects={[]} onChanged={() => {}} />);
    await waitFor(() => expect(items(first.container).length).toBeGreaterThan(0));
    first.unmount();
    const before = asked.length;
    const { container } = render(<History sessions={[]} projects={[]} onChanged={() => {}} />);
    expect(container.querySelector("[data-testid='page-sessions-list-skeleton']")).toBeNull();
    expect(items(container).length).toBeGreaterThan(0);
    await waitFor(() => expect(asked.length).toBeGreaterThan(before));
  });
});

describe("a session past the working set, opened from History", () => {
  it("is read by its id and its page has it", async () => {
    const { ActionHostProvider } = await import("../components/ItemMenu");
    const { useWithSelected } = await import("../lib/session-lookup");
    const { useState } = await import("react");
    // The app's list holds only its working set: none of History's rows.
    function AppLike() {
      const [selected, setSelected] = useState<string | null>(null);
      const sessions = useWithSelected([], selected);
      const page = sessions.find((s) => s.id === selected);
      return (
        <ActionHostProvider host={{ openSessionById: (id) => setSelected(id) }}>
          <History sessions={[]} projects={[]} onChanged={() => {}} />
          <p data-testid="page-of">{page ? page.title : selected ? "loading" : "none"}</p>
        </ActionHostProvider>
      );
    }
    const { container, getByTestId } = render(<AppLike />);
    await waitFor(() => expect(items(container).length).toBeGreaterThan(0));
    fireEvent.click(container.querySelector<HTMLButtonElement>("[data-key='s/s3'] [data-testid='page-session-row']")!);
    await waitFor(() => expect(getByTestId("page-of").textContent).toBe("Session 3"));
  });
});

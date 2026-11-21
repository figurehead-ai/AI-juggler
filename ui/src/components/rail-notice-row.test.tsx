import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { NoticeInfo } from "../lib/ipc/notices";

/**
 * The rail's row for a notice (RailNoticeRow): how a hand-over names its
 * action, a pool member's question (subject first when its thread is
 * known), and a held message's reason under a two-line title. Moved out
 * of notifications.test.tsx (its line cap) unchanged.
 */

const T0 = Date.now();
const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
function notice(over: Partial<NoticeInfo>): NoticeInfo {
  return { id: 1, kind: "finished", needs: false, title: "", detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: T0, unread: false, subject: `n:${over.id ?? 1}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false, ...over };
}
const quota = notice({ id: 7, kind: "quota", needs: true, unread: true, title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", meter: 7, action: "hand-over", target: { ...target, account_id: "acct-work" }, facts: { ...facts, window: "5-hour", resets_at_ms: T0 + 3_600_000, sessions: 2, account: "Work Max" } });

afterEach(() => {
  cleanup();
});

it("walk A5: the rail names a hand-over as it will act, the switch with something open, its app without", async () => {
  const { RailNoticeRow } = await import("./RailNoticeRow");
  const { primaryWords } = await import("../lib/notice-words");
  expect(primaryWords(quota, true)).toBe("Continue in another app…");
  expect(primaryWords(quota, false)).toBe("Open its app");
  const view = render(<RailNoticeRow notice={quota} now={Date.now()} onOpen={() => undefined} canHandOver={false} />);
  expect(view.getByTestId("rail-notice").textContent).toContain("Open its app");
  expect(view.getByTestId("rail-notice").textContent).not.toContain("Continue in another app");
  view.unmount();
});

// D-118's mock: an agent's question in the rail reads "builder-1 asks
// you" (its amber mark, the words in ink) over "Release pool · «Export flow»", with no
// "Open" under it: the row itself opens it. Its pool is said once.
it("a pool member's question: the title is the mark, no bare Open, its pool once", async () => {
  const { RailNoticeRow } = await import("./RailNoticeRow");
  const asked = notice({ id: 10, kind: "message", needs: true, unread: true, title: "builder-1 asks you", detail: "Release pool · «Export flow»", action: "pool" });
  const view = render(<RailNoticeRow notice={asked} now={Date.now()} onOpen={() => undefined} canHandOver={false} />);
  // AJ-E3603 R5: the title in ink, the level's colour on its mark alone.
  const title = view.getByTestId("rail-notice-title");
  expect(title.className).toContain("text-ink");
  expect(title.className).not.toMatch(/text-(warning|danger)/);
  expect(title.querySelector(".level-mark[data-level='2']")).toBeTruthy();
  expect(view.queryByTestId("rail-notice-action")).toBeNull();
  expect(view.getByTestId("rail-notice-line").textContent).toBe("Release pool · «Export flow»");
  expect(view.getByRole("button").textContent).not.toMatch(/Open$/);
  view.unmount();
});

// D-118's mock (round 3): with what it's about known (its thread, its
// pool), the row is subject first, as a session's need row: "Export
// flow", then "builder-1 asks you" after its amber mark, then "Release pool".
it("a pool member's question, subject first: its thread, then who asks (its amber mark), then its pool", async () => {
  const { RailNoticeRow } = await import("./RailNoticeRow");
  const asked = notice({ id: 11, kind: "message", needs: true, unread: true, title: "builder-1 asks you", detail: "Release pool · «Export flow»", action: "pool" });
  const view = render(<RailNoticeRow notice={asked} now={Date.now()} onOpen={() => undefined} canHandOver={false} about={{ subject: "Export flow", context: "Release pool" }} />);
  expect(view.getByTestId("rail-notice-title").textContent).toBe("Export flow");
  expect(view.getByTestId("rail-notice-action").textContent).toBe("builder-1 asks you");
  expect(view.getByTestId("rail-notice-action").className).toContain("text-ink-2");
  expect(view.getByTestId("rail-notice-action").querySelector(".level-mark[data-level='2']")).toBeTruthy();
  expect(view.getByTestId("rail-notice-line").textContent).toBe("Release pool");
  // Its name says who asks first (the need), then about what and where.
  expect(view.getByRole("button").textContent).toContain("builder-1 asks you, Export flow, Release pool");
  view.unmount();
});

// The rc19 walk: a held message's row read "«Untitled session · r3-tools
// started 01:15 PM» wants t…", and its reason (Ask first) was out of
// sight. A title on two lines keeps the reason; only a bare "Open" gives
// way (the row itself opens it). A quota's long title still takes the
// detail's line (the 0.4 review).
it("rc19: a held message's row keeps its reason under a two-line title", async () => {
  const { RailNoticeRow } = await import("./RailNoticeRow");
  const tall = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ height: 36, width: 200, top: 0, left: 0, right: 200, bottom: 36, x: 0, y: 0, toJSON: () => ({}) });
  try {
    const held = notice({ id: 9, kind: "message", needs: true, unread: true, title: "«r3-tools» wants to message «web»", detail: "Ask first in «Team»", action: "pool" });
    const view = render(<RailNoticeRow notice={held} now={Date.now()} onOpen={() => undefined} canHandOver={false} />);
    expect(view.getByTestId("rail-notice-line").textContent).toBe("Ask first in «Team»");
    view.unmount();
    const long = render(<RailNoticeRow notice={quota} now={Date.now()} onOpen={() => undefined} canHandOver={false} />);
    expect(long.queryByTestId("rail-notice-line")).toBeNull();
    expect(long.getByTestId("rail-notice").textContent).toContain("Open its app");
    long.unmount();
  } finally {
    tall.mockRestore();
  }
});

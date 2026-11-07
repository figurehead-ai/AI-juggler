/**
 * AJ-E2405 R5: the cleanup history filters by project (the whole
 * history; a session's own tab is already that session's) and pages
 * back with "Show older", never all at once.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { HistoryView } from "./HistoryView";
import * as ipc from "../../lib/ipc/processes";
import type { CleanupEventInfo } from "../../lib/ipc/processes";
import { pick } from "../../test/pick";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const event = (id: number, project: string): CleanupEventInfo => ({
  id,
  at_ms: Date.now() - id * 60_000,
  actor: "user",
  automatic: false,
  pid: 1000 + id,
  name: "python3",
  session_id: `s-${id}`,
  session_title: `Session ${id}`,
  tool: "claude-code",
  account_id: null,
  project_id: project,
  outcome: "exited on TERM",
  ports_before: null,
  ports_after: null,
});

const PROJECTS = [
  { id: "p-shop", label: "shop" },
  { id: "p-docs", label: "docs" },
];

it("filters the whole history by project", async () => {
  const read = vi.spyOn(ipc, "processesHistory").mockImplementation((_, project) =>
    Promise.resolve({ items: project === "p-docs" ? [event(2, "p-docs")] : [event(1, "p-shop"), event(2, "p-docs")], next_cursor: null, total: null }),
  );
  render(<HistoryView projects={PROJECTS} />);
  await screen.findByTestId("history-1");
  pick(screen.getByTestId("cleanup-history-project"), "docs");
  await waitFor(() => expect(screen.queryByTestId("history-1")).toBeNull());
  expect(screen.getByTestId("history-2")).toBeTruthy();
  expect(read).toHaveBeenLastCalledWith(null, "p-docs", { cursor: null, limit: null });
});

it("a session's own history has no project filter", async () => {
  vi.spyOn(ipc, "processesHistory").mockResolvedValue({ items: [event(1, "p-shop")], next_cursor: null, total: null });
  render(<HistoryView sessionId="s-1" projects={PROJECTS} />);
  await screen.findByTestId("history-1");
  expect(screen.queryByTestId("cleanup-history-project")).toBeNull();
});

it("pages back with Show older, from where the last page ended", async () => {
  const read = vi.spyOn(ipc, "processesHistory").mockImplementation((_, __, page) =>
    Promise.resolve(page.cursor === "c1" ? { items: [event(3, "p-shop")], next_cursor: null, total: null } : { items: [event(1, "p-shop"), event(2, "p-shop")], next_cursor: "c1", total: null }),
  );
  render(<HistoryView projects={PROJECTS} />);
  await screen.findByTestId("history-2");
  expect(screen.queryByTestId("history-3")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Show older" }));
  await screen.findByTestId("history-3");
  expect(screen.getByTestId("history-1")).toBeTruthy();
  expect(read).toHaveBeenLastCalledWith(null, null, { cursor: "c1", limit: null });
  expect(screen.queryByRole("button", { name: "Show older" })).toBeNull();
});

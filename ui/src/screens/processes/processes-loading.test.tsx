/**
 * AJ-E1552 — the Processes page's skeletons draw only while their read
 * loads: the process list and Cleanup history each show a skeleton while
 * loading (Still loading after 3 s), their own empty words, and an error
 * with Retry that reads again.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/processes";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { HistoryView } from "./HistoryView";
import { ProcessesScreen } from "./ProcessesScreen";
import { forgetProcessRows } from "./useProcessRows";
import type { ProcessStatusInfo } from "../../types/generated/ProcessStatusInfo";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
  forgetProcessRows();
  try {
    localStorage.removeItem("aij.processes.view");
  } catch {
    // no storage: the default view
  }
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("the process list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "processesStatus").mockReturnValue(never());
    render(<ProcessesScreen />);
    await expectSlowSkeleton("processes-loading");
  });
  it("empty", async () => {
    const status = await ipc.processesStatus();
    vi.spyOn(ipc, "processesStatus").mockResolvedValue(status satisfies ProcessStatusInfo);
    vi.spyOn(ipc, "processesPage").mockResolvedValue({ items: [], next_cursor: null, total: 0 });
    render(<ProcessesScreen />);
    expect(await screen.findByText(/Nothing needs you: no process is left behind/)).toBeTruthy();
    expect(screen.queryByTestId("processes-loading")).toBeNull();
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "processesStatus").mockRejectedValue(new Error("down"));
    render(<ProcessesScreen />);
    await expectErrorRetry("the processes", read, "processes-loading");
  });
});

describe("Cleanup history", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "processesHistory").mockReturnValue(never());
    render(<HistoryView />);
    await expectSlowSkeleton("cleanup-history-loading");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "processesHistory").mockResolvedValue({ items: [], next_cursor: null, total: 0 });
    render(<HistoryView />);
    await expectEmpty("cleanup-history-empty", "Nothing was stopped in the last 7 days. What you or AI Juggler stops shows up here.", "cleanup-history-loading");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "processesHistory").mockRejectedValue(new Error("down"));
    render(<HistoryView />);
    await expectErrorRetry("the history", read, "cleanup-history-loading");
  });
});

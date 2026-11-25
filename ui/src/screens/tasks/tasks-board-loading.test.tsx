/**
 * AJ-E1552 — the Tasks screen's board columns and its plan draw a
 * skeleton only while their read loads (Still loading after 3 s), then
 * their own empty words, or an error with Retry that reads again.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, it, vi } from "vitest";

import { forgetPaged } from "../../lib/paged";
import * as ipc from "../../lib/ipc/tasks-across";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { Board, Plan } from "./TaskBoard";

const none = { items: [], next_cursor: null, total: 0 };
const shared = { holderApp: () => null, filter: {}, filterKey: "all", open: null, projectName: () => null, onOpen: () => {}, onOpenProject: () => {} };

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  forgetPaged();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const board = () => render(<Board counts={null} narrow={false} {...shared} />);
const plan = () => render(<Plan now={0} {...shared} />);

describe("a board column", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    // The Ready column never answers; the others are empty.
    vi.spyOn(ipc, "tasksAcross").mockImplementation((filter) => (filter.status === "ready" ? never() : Promise.resolve(none)));
    board();
    await expectSlowSkeleton("tasks-skeleton-ready");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "tasksAcross").mockResolvedValue(none);
    board();
    await expectEmpty("tasks-empty-ready", "Nothing ready to take.", "tasks-skeleton-ready");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "tasksAcross").mockImplementation((filter) => (filter.status === "ready" ? Promise.reject(new Error("down")) : Promise.resolve(none)));
    board();
    await expectErrorRetry("the ready tasks", read, "tasks-skeleton-ready");
    await screen.findByTestId("tasks-empty-blocked");
  });
});

describe("the plan", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "tasksAcross").mockReturnValue(never());
    plan();
    await expectSlowSkeleton("tasks-plan-skeleton");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "tasksAcross").mockResolvedValue(none);
    plan();
    await expectEmpty("tasks-plan-empty", "No epics or stories yet.", "tasks-plan-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "tasksAcross").mockRejectedValue(new Error("down"));
    plan();
    await expectErrorRetry("the plan", read, "tasks-plan-skeleton");
  });
});

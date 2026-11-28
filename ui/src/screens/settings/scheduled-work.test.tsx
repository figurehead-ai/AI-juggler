/**
 * AJ-E2027 R6 — Settings › Advanced › Scheduled work: the app's own
 * schedules, read-only, in plain words.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ScheduledWorkSection, whenOf } from "./ScheduledWorkSection";
import type { ScheduleInfo } from "../../types/generated/ScheduleInfo";

const NOW = Date.now();
const HOUR = 3_600_000;

function app(id: string, title: string, handler: string, trigger: ScheduleInfo["trigger"]): ScheduleInfo {
  return {
    id,
    title,
    handler,
    source: "ai_juggler",
    trigger,
    modifiers: { window: null, jitter_ms: 0, lifetime: { kind: "always" } },
    next_due_ms: NOW + HOUR,
    last_run_ms: null,
    last_outcome: null,
    paused: false,
    ended_ms: null,
    created_ms: NOW,
    created_by: "app",
    about: null,
  };
}

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_SCHEDULES__");
});

describe("AJ-E2027 R6 — Settings › Advanced › Scheduled work", () => {
  it("lists the app's own schedules read-only, in plain words", async () => {
    const backup = app("b", "Check whether a cloud backup is due", "cloud_backup", { kind: "every", interval_ms: HOUR, anchor_ms: 0 });
    const tidy = app("m", "Tidy what Bot remembers", "memory.maintain", { kind: "when_idle", max_wait_ms: HOUR, after_ms: HOUR });
    window.__AIJ_SCHEDULES__ = { app: [backup, tidy], calls: [] };
    render(<ScheduledWorkSection />);
    const rows = await screen.findAllByTestId("scheduled-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]!.textContent).toContain("Every hour");
    expect(rows[1]!.textContent).toContain("When this computer is quiet");
    expect(rows[0]!.querySelector("button")).toBeNull();
    expect(whenOf({ ...backup, paused: true }, NOW)).toContain("Paused");
  });

  /** 0.5.2 G30 (RW1 G1: "next 7:00 PM" still at 7:00:36): a run is
   * followed by a read, so the row says when it ran and what is next. */
  it("follows each run: next and last read again", async () => {
    const backup = app("b", "Check whether a cloud backup is due", "cloud_backup", { kind: "every", interval_ms: HOUR, anchor_ms: 0 });
    window.__AIJ_SCHEDULES__ = { app: [{ ...backup, next_due_ms: NOW + 60_000 }], calls: [] };
    render(<ScheduledWorkSection />);
    const [row] = await screen.findAllByTestId("scheduled-row");
    expect(row!.textContent).not.toContain("last ran");
    window.__AIJ_SCHEDULES__ = { app: [{ ...backup, next_due_ms: NOW + HOUR, last_run_ms: NOW, last_outcome: "ok" }], calls: [] };
    act(() => {
      window.dispatchEvent(new Event("schedules://changed"));
    });
    await waitFor(() => expect(screen.getByTestId("scheduled-row").textContent).toContain("last ran"));
  });

  it("a read that fails says so with Retry, never that nothing is scheduled", async () => {
    window.__AIJ_SCHEDULES__ = { app: [], calls: [], off: true };
    render(<ScheduledWorkSection />);
    expect(await screen.findByText("Couldn't load the scheduled work.")).toBeTruthy();
    expect(screen.queryByText("Nothing is scheduled.")).toBeNull();
    window.__AIJ_SCHEDULES__ = { app: [app("b", "Check whether a cloud backup is due", "cloud_backup", { kind: "every", interval_ms: HOUR, anchor_ms: 0 })], calls: [] };
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(await screen.findAllByTestId("scheduled-row")).toHaveLength(1);
  });

  it("an empty answer says nothing is scheduled", async () => {
    window.__AIJ_SCHEDULES__ = { app: [], calls: [] };
    render(<ScheduledWorkSection />);
    expect(await screen.findByText("Nothing is scheduled.")).toBeTruthy();
  });
});

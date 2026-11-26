/**
 * AJ-E2027 R3, R6 — the Reminders list beside the watches (hidden until
 * one is set; Change time, Pause, Remove with Undo, Pause all), Bot's
 * reminder card (only the user's click sets it, priming Notifications
 * first).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { RemindersSection } from "./RemindersSection";
import { BotReminderDraft } from "../bot/BotReminderDraft";
import { forgetPaged } from "../../lib/paged";
import type { ScheduleInfo } from "../../types/generated/ScheduleInfo";

const NOW = Date.now();
const HOUR = 3_600_000;

function reminder(id: string, at: number, extra: Partial<ScheduleInfo> = {}): ScheduleInfo {
  return {
    id,
    title: `Look at “${id}”`,
    handler: "reminder",
    source: "ai_juggler",
    trigger: { kind: "at", at_ms: at },
    modifiers: { window: null, jitter_ms: 0, lifetime: { kind: "once" } },
    next_due_ms: at,
    last_run_ms: null,
    last_outcome: null,
    paused: false,
    ended_ms: null,
    created_ms: NOW,
    created_by: "user",
    about: { kind: "session", id },
    ...extra,
  };
}

beforeEach(() => {
  forgetPaged();
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_SCHEDULES__");
});

describe("AJ-E2027 — Reminders beside the watches", () => {
  it("is hidden until a reminder is set (D-127)", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [], calls: [] };
    const { container } = render(<RemindersSection book={new Map()} now={NOW} />);
    await act(async () => undefined);
    expect(container.textContent).toBe("");
  });

  it("lists each reminder with its words, what it's about and when; pauses, moves and removes it by id", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [reminder("api", NOW + HOUR), reminder("web", NOW + 2 * HOUR)], calls: [] };
    render(<RemindersSection book={new Map()} now={NOW} />);
    const rows = await screen.findAllByTestId("reminder-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]!.textContent).toContain("Look at “api”");
    expect(screen.getByTestId("reminders-line").textContent).toBe("2 set");
    await act(async () => {
      fireEvent.click(screen.getAllByTestId("reminder-pause")[0]!);
    });
    expect(window.__AIJ_SCHEDULES__.calls).toContainEqual(["schedule_pause", { id: "api", paused: true }]);
    await waitFor(() => expect(screen.getByTestId("reminders-line").textContent).toBe("1 set · 1 paused"));
    fireEvent.click(screen.getAllByTestId("reminder-change")[1]!);
    await act(async () => {
      fireEvent.click(screen.getAllByTestId("reminder-time-choice")[0]!);
    });
    const moved = window.__AIJ_SCHEDULES__.calls!.find(([name]) => name === "schedule_update")!;
    expect(moved[1]).toMatchObject({ id: "web", title: "Look at “web”" });
    await act(async () => {
      fireEvent.click(screen.getAllByTestId("reminder-remove")[0]!);
    });
    expect(window.__AIJ_SCHEDULES__.calls).toContainEqual(["schedule_remove", { id: "api" }]);
  });

  it("Pause all pauses every reminder, then offers Resume all", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [reminder("api", NOW + HOUR), reminder("web", NOW + 2 * HOUR)], calls: [] };
    render(<RemindersSection book={new Map()} now={NOW} />);
    const pauseAll = await screen.findByTestId("reminders-pause-all");
    await act(async () => {
      fireEvent.click(pauseAll);
    });
    expect(window.__AIJ_SCHEDULES__.calls).toContainEqual(["schedule_pause_all", { paused: true }]);
    expect(await screen.findByTestId("reminders-resume-all")).toBeTruthy();
  });

  it("an ended reminder says it was told, and offers only Remove", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [reminder("api", NOW - HOUR, { ended_ms: NOW - HOUR + 1_000, last_run_ms: NOW - HOUR + 1_000, last_outcome: "ok", next_due_ms: null })], calls: [] };
    render(<RemindersSection book={new Map()} now={NOW} />);
    const row = await screen.findByTestId("reminder-row");
    expect(row.getAttribute("data-state")).toBe("ended");
    expect(row.textContent).toMatch(/Told /);
    expect(screen.queryByTestId("reminder-change")).toBeNull();
    expect(screen.getByTestId("reminder-remove")).toBeTruthy();
  });
});

describe("AJ-E2027 — Bot's reminder card", () => {
  it("sets nothing until the user clicks Set reminder; then sets it with the turn's id", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [], calls: [] };
    render(<BotReminderDraft draft={{ at_ms: NOW + HOUR, title: "Check the deploy", about: null, chat_turn: "t-1" }} line="I drafted the reminder." />);
    expect(screen.getByTestId("bot-reminder-title").textContent).toBe("Check the deploy");
    expect(window.__AIJ_SCHEDULES__.calls).toHaveLength(0);
    await act(async () => {
      fireEvent.click(screen.getByTestId("bot-reminder-set-button"));
    });
    expect(window.__AIJ_SCHEDULES__.calls).toEqual([["schedule_remind", { at_ms: NOW + HOUR, title: "Check the deploy", about: null, chat_turn: "t-1" }]]);
    expect(await screen.findByTestId("bot-reminder-set")).toBeTruthy();
  });

  it("with no time said, Set waits for a time to be picked", async () => {
    window.__AIJ_SCHEDULES__ = { yours: [], calls: [] };
    render(<BotReminderDraft draft={{ at_ms: null, title: "Your reminder", about: null, chat_turn: null }} line="Pick a time." />);
    expect(screen.getByTestId("bot-reminder-set-button")).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByTestId("bot-reminder-change"));
    fireEvent.click(screen.getAllByTestId("bot-reminder-choice")[1]!);
    expect(screen.getByTestId("bot-reminder-set-button")).toHaveProperty("disabled", false);
  });
});

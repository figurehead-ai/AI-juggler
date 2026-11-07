/**
 * AJ-E3104 R2: Needs you counts the same notices as the bell. A snoozed
 * cap, agent's message or task is out of Home's rows (and so its
 * heading's count) until its snooze ends, as the bell leaves it out.
 */
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { notificationsSnooze, resetNoticeStandIn } from "./ipc/notices";
import { notSnoozed, useSnoozedSubjects } from "./needs-you";
import { resetNotices } from "./notices-store";

beforeEach(() => {
  resetNoticeStandIn();
  resetNotices();
});

describe("Home's own rows leave out what's snoozed", () => {
  it("drops a snoozed cap, message and task, and keeps the rest", () => {
    const snoozed = new Set(["cap:2", "message:ask:ask-1", "task:7"]);
    const shown = notSnoozed(snoozed, {
      caps: [{ id: 1 }, { id: 2 }],
      messages: [{ key: "ask:ask-1" }, { key: "ask:ask-2" }],
      tasks: [{ id: 7 }, { id: 8 }],
    });
    expect(shown).toEqual({ caps: [{ id: 1 }], messages: [{ key: "ask:ask-2" }], tasks: [{ id: 8 }] });
    // Nothing snoozed: every row stays.
    expect(notSnoozed(new Set(), { caps: [{ id: 1 }], messages: [], tasks: [] }).caps).toHaveLength(1);
  });

  it("a snooze counts until it ends; one for good, always", async () => {
    const now = Date.now();
    await act(async () => {
      await notificationsSnooze("cap:2", now + 60_000);
      await notificationsSnooze("task:7", null, true);
    });
    const during = renderHook(() => useSnoozedSubjects(now));
    await waitFor(() => expect([...during.result.current].sort()).toEqual(["cap:2", "task:7"]));
    // Two minutes on, the cap's snooze has ended: it needs you again.
    const after = renderHook(() => useSnoozedSubjects(now + 120_000));
    await waitFor(() => expect([...after.result.current]).toEqual(["task:7"]));
  });
});

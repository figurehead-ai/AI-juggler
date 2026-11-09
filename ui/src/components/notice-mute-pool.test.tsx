/**
 * AJ-E3009 R2 (D-62): a notice names its pool, and its menu mutes that
 * pool. A notice about no pool offers no pool mute.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { NoticeInfo } from "../lib/ipc/notices";
import type { NoticeActions } from "../lib/notice-actions";
import { NoticeRow } from "./NoticeRow";

afterEach(cleanup);

const target = { session_id: "s-1", conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: "pool-team" };
const notice = (pool: string | null): NoticeInfo => ({ id: 7, kind: "message", needs: true, title: "Reviewer asks you", detail: "In «Team»", project_id: null, project: null, target: { ...target, pool_id: pool }, facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null }, action: null, meter: null, created_ms: 0, unread: true, subject: "m:7", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false });

function actions(mute: NoticeActions["mute"]): NoticeActions {
  return { primary: vi.fn(), canHandOver: () => false, handOverSubject: () => null, second: vi.fn(), snooze: vi.fn(async () => true), wake: vi.fn(), mute, quietSince: () => null };
}

describe("a notice's pool mute (AJ-E3009 R2)", () => {
  it("mutes the pool it names", () => {
    const mute = vi.fn();
    const about = notice("pool-team");
    render(<NoticeRow notice={about} actions={actions(mute)} now={0} />);
    fireEvent.click(screen.getByTestId("notice-more"));
    const item = screen.getByTestId("notice-mute-pool");
    expect(item.textContent).toContain("Mute this pool");
    fireEvent.click(item);
    expect(mute).toHaveBeenCalledWith(about, "pool");
  });

  it("offers no pool mute for a notice about no pool", () => {
    render(<NoticeRow notice={notice(null)} actions={actions(vi.fn())} now={0} />);
    fireEvent.click(screen.getByTestId("notice-more"));
    expect(screen.queryByTestId("notice-mute-pool")).toBeNull();
    expect(screen.getByTestId("notice-mute-kind")).toBeTruthy();
  });
});

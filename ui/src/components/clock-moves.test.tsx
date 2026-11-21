/**
 * 0.4.9 (AJ-E2312 item 5, SK48-6, the 0.4.8 soak's K3 lab): about 11 min
 * in, Home's rows still said "waiting 7 min" and the rail's "6 min ago"
 * while "Your apps" went from "last open 7 min ago" to "11 min ago": the
 * bell's panel kept the time it opened at, and "Your apps" read the
 * system's clock at each draw while everything else read the app's.
 * Every relative time reads the app's one clock now (`lib/clock`), one
 * shared tick: advance the clock, and the words move together.
 */
import type { JSX } from "react";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NotificationCentre } from "./NotificationCentre";
import { HomeApps } from "./HomeApps";
import { useNow } from "../shell/use-rail";
import { agoWords } from "../lib/time-words";
import { CLOCK_EVERY_MS } from "../lib/clock";
import { notificationsSummary, resetNoticeStandIn, type NoticeInfo } from "../lib/ipc/notices";
import { resetNotices } from "../lib/notices-store";
import { setSeam } from "../test/fixtures";
import type { KnownPlaceInfo } from "../lib/ipc/places";

const MIN = 60_000;

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setSeam("__AIJ_NOTICES__", undefined);
  setSeam("__AIJ_PLACES__", undefined);
  resetNoticeStandIn();
  resetNotices();
});

/** `minutes` of the app's clock pass (its tick included). */
async function pass(minutes: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(minutes * MIN + CLOCK_EVERY_MS);
  });
}

const target = { session_id: "api", conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };

describe("SK48-6: every relative time moves on the app's one clock", () => {
  it("the bell's rows keep pace while its panel is open", async () => {
    vi.useFakeTimers();
    const asks: NoticeInfo = {
      id: 7, kind: "asks", needs: true, title: "“api” needs your reply", detail: "", project_id: null, project: null, target, facts, action: "open", meter: null,
      created_ms: Date.now() - 7 * MIN, unread: true, subject: "s:api", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
    };
    setSeam("__AIJ_NOTICES__", { app: [asks] });
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText("waiting 7 min")).not.toBeNull();
    await pass(4);
    expect(screen.queryByText("waiting 7 min")).toBeNull();
    expect(screen.getByText("waiting 11 min")).not.toBeNull();
  });

  it("Your apps and a row read the same clock: both move on its tick, together", async () => {
    vi.useFakeTimers();
    const start = Date.now();
    const place: KnownPlaceInfo = {
      id: "app:claude:side", app_id: "claude", app: "Claude", instance: "Side", profile: "Side", label: "Claude · Side", app_pid: null, lost: false,
      keep_alive: false, paused: null, account_id: null, seen_ms: start - 7 * MIN, cannot_start: null, recent: [], found: false,
    };
    setSeam("__AIJ_PLACES__", { places: [place] });
    // A row as the shell hands it its now (the rail's and Home's).
    function Row(): JSX.Element {
      return <span data-testid="row-when">{agoWords(start - 6 * MIN, useNow())}</span>;
    }
    render(
      <>
        <Row />
        <HomeApps items={[]} runwayOf={() => null} />
      </>,
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    const tile = () => screen.getByTestId("home-app-closed").textContent ?? "";
    expect(screen.getByTestId("row-when").textContent).toBe("6 min ago");
    expect(tile()).toContain("last open 7 min ago");
    await pass(4);
    expect(screen.getByTestId("row-when").textContent).toBe("10 min ago");
    expect(tile()).toContain("last open 11 min ago");
  });
});

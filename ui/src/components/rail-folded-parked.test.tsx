/**
 * D-117 (AJ-E3602), CI run 43777785232's home-scrolled captures: the
 * folded icon rail lit FIVE Apps (three amber "!", two red ✕) while its
 * count and the bell said 4. The fixture is the regression gate's PARKED
 * one (`tests/visual/regression.spec.ts`): its limit-hit session parked,
 * the reset reached. The backend lists no parked session (its level is 0,
 * the shared table); the window still read its "Limit reached" and lit it.
 * Now both read one rule, and the count, the lit icons and the bell agree,
 * parked or not.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { needsFor } from "../test/needs-kit";
import { FoldedRail } from "./RailFolded";
import { NotificationBell } from "./NotificationBell";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { resetSeen } from "../lib/seen";
import { resetNoticeStandIn } from "../lib/ipc/notices";
import { resetNotices } from "../lib/notices-store";
import { instance, project } from "../test/fixtures";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** The gate's pinned clock (tests/visual/axe-gallery.ts GATE_NOW). */
const T0 = 1_728_800_000_000;
const NEEDS: Partial<Record<InstanceInfo["status"], InstanceInfo["needs_attention"]>> = { waiting_user: "waiting_user", stuck: "stuck", quota_blocked: "quota_blocked", failed: "failed" };
/** The gate's `allStatuses` (tests/visual/fixtures.ts), in brief: one
 * session per status, and three of a desktop app (running, ended, and
 * crashed, its failure in its status alone). */
const own = (n: number, status: InstanceInfo["status"], work: boolean): InstanceInfo =>
  instance({ id: `sess-${n}`, status, needs_attention: NEEDS[status] ?? null, account_id: work ? "claude-work" : "claude-personal", account_label: work ? "Work Max" : "Personal", project_id: work ? "proj-1" : "proj-2", started_at_ms: T0 - n * 3_600_000, last_activity_ms: T0 - n * 3_500_000, quota: status === "quota_blocked" ? { used_percentage: 100, resets_at_ms: T0 + 7_200_000, confidence: "measured" } : null });
const desk = (n: number, status: InstanceInfo["status"]): InstanceInfo => ({ ...own(n, status, false), id: `sess-desktop-${n - 9}`, provider: "fake-desktop", kind: "desktop", account_id: "fake-desk", account_label: "Fake Desk", needs_attention: null, project_id: "proj-3" });
const SESSIONS: InstanceInfo[] = [
  own(1, "starting", true), own(2, "running", true), own(3, "waiting_user", false), own(4, "stuck", true), own(5, "quota_blocked", false),
  own(6, "finished", true), own(7, "stopped", false), own(8, "failed", true), own(9, "handed_over", false),
  desk(10, "running"), desk(11, "finished"), desk(12, "failed"),
];
const PROJECTS = [project({ id: "proj-1", name: "ai-juggler" }), project({ id: "proj-2", name: "storefront" }), project({ id: "proj-3", name: "sacrificial" })];
/** The regression gate's PARKED: its limit-hit sessions parked, the reset reached. */
const PARKED = SESSIONS.map((session) => (session.status === "quota_blocked" ? { ...session, parked: "measured", parked_resets_at_ms: T0 - 60_000 } : session));

beforeEach(() => {
  resetSeen();
  resetRailOrderCache();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  resetNoticeStandIn();
  resetNotices();
});

/** The folded rail and the bell over `sessions`, once the count is in. */
async function glance(sessions: InstanceInfo[]) {
  Object.assign(window, { __AIJ_FIXTURE__: { conversations: [] } });
  needsFor({ sessions, conversations: [], projects: PROJECTS });
  const view = render(
    <ActionHostProvider host={{}}>
      <ConversationsProvider>
        <FoldedRail sessions={sessions} projects={PROJECTS} now={T0} selectedId={null} onSelect={() => {}} />
        <NotificationBell sessions={sessions} />
      </ConversationsProvider>
    </ActionHostProvider>,
  );
  const count = await view.findByTestId("rail-folded-needs-you");
  await waitFor(() => expect(view.getByTestId("bell-count")).toBeTruthy());
  const rail = view.getByTestId("rail-folded");
  const corners = [...rail.querySelectorAll<HTMLElement>("button[data-rail-item] [data-testid='rail-folded-needs']")];
  const lit = [...rail.querySelectorAll<HTMLElement>("button[data-rail-item]")].filter((row) => row.querySelector(".app-tone")?.classList.contains("is-lit"));
  return {
    view,
    count: Number(count.textContent?.replace(/\D/g, "")),
    corners: corners.map((corner) => corner.dataset.level ?? "").sort((a, b) => a.localeCompare(b)),
    lit: lit.length,
    bell: Number(view.getByTestId("bell-count").textContent),
  };
}

describe("the folded rail's count = its lit icons = the bell (D-117)", () => {
  it("parked, the reset reached (home-scrolled's fixture): 4 = 4 = 4, the parked one calm", async () => {
    const { view, count, corners, lit, bell } = await glance(PARKED);
    expect(corners).toHaveLength(count);
    expect(lit).toBe(count);
    expect(bell).toBe(count);
    expect(count).toBe(4);
    // Two amber (asks, stuck), two red (the two that stopped with an error).
    expect(corners).toEqual(["2", "2", "3", "3"]);
    // The parked one isn't counted, and (0.4.9, SK-20) with its tool
    // closed it isn't open, so the folded rail, which lists what's open,
    // leaves it out: never a lit icon for it.
    expect(view.container.querySelector("button[data-instance-id='sess-5']")).toBeNull();
  });

  it("not parked (home's fixture): 5 = 5 = 5, the limit one amber", async () => {
    const { count, corners, lit, bell } = await glance(SESSIONS);
    expect([corners.length, lit, bell]).toEqual([count, count, count]);
    expect(count).toBe(5);
    expect(corners).toEqual(["2", "2", "2", "3", "3"]);
  });
});

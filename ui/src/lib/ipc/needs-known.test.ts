/**
 * 0.5.1 F54 (D-103.6): the preview's Needs you is known only once the
 * needs list shows a page read after the stand-in's first tick. "Ticked"
 * alone said known while the shared list still showed its earlier, empty
 * page being read again: Home dropped Needs you and its Apps' need lines
 * for a few frames, then put them back (a 0.21 layout shift, twice, on a
 * loaded machine; layout-shift.spec "opening Home holds still").
 */
import { beforeEach, describe, expect, it } from "vitest";
import { notificationsPage, resetNoticeStandIn, standInNeeds, standInNeedsKnown, standInNeedsPage, type StandInNeed } from "./notices";

const need: StandInNeed = {
  key: "s:one",
  kind: "asks",
  title: "One",
  detail: "Asks a question",
  project_id: null,
  project: null,
  target: { session_id: "one", conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null },
  action: null,
};

describe("the preview's Needs you is known (F54)", () => {
  beforeEach(() => resetNoticeStandIn());

  it("not from the tick alone: the page shown before it is still the old one", async () => {
    const before = standInNeedsPage(50)!;
    expect(standInNeedsKnown(before.items)).toBe(false);
    standInNeeds([need]);
    // The list still shows its earlier (empty) page while it reads again.
    expect(standInNeedsKnown(before.items)).toBe(false);
    expect(standInNeedsKnown()).toBe(false);
    // Its answer lands: known, and for good.
    const after = await notificationsPage("needs", { cursor: null, limit: 50 });
    expect(after.items).toHaveLength(1);
    expect(standInNeedsKnown(after.items)).toBe(true);
    expect(standInNeedsKnown(before.items)).toBe(true);
  });

  it("at once for a list first drawn after the tick (its seed is the tick's)", () => {
    standInNeeds([need]);
    const seeded = standInNeedsPage(50)!;
    expect(seeded.items).toHaveLength(1);
    expect(standInNeedsKnown(seeded.items)).toBe(true);
  });
});

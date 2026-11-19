/**
 * F-rc3f-2 (0.4.6 rc3f, P2): on a pool's page with Tasks open, the bell's
 * panel opened under the Tasks panel (about 130 px of it showing, empty)
 * while the page around it was inert. Placed in the top bar (z-30), it
 * lost to the page's own panel (z-30 too, later in the page). It opens on
 * the body now, above every page's own panels, whole and usable.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { AppHeader } from "./AppHeader";
import { PoolPanel } from "../pages/pool/PoolPanel";
import { resetNoticeStandIn, standInNeeds } from "../lib/ipc/notices";
import { applySummary, resetNotices, setCentreOpen } from "../lib/notices-store";

const target = { session_id: "sess-1", conversation_id: null, account_id: null, project_id: "proj-docs", page: null, task_id: null, pool_id: null };

beforeEach(() => {
  resetNoticeStandIn();
  resetNotices();
});
afterEach(() => {
  setCentreOpen(false);
  cleanup();
});

/** The z-index a Tailwind class gives (`z-30`, `z-[60]`), else null. */
function zOf(element: Element): number | null {
  const found = /(?:^|\s)z-\[?(\d+)\]?(?:\s|$)/.exec(element.getAttribute("class") ?? "");
  return found ? Number(found[1]) : null;
}

/** Whether `a` paints over `b` where they meet: in the window, each is
 * stacked by its outermost ancestor that sets a z-index (each such one
 * starts its own stacking context); higher wins, a tie goes to the later
 * in the page. */
function paintsAbove(a: Element, b: Element): boolean {
  const layer = (element: Element): { at: Element; z: number } => {
    let out = { at: element, z: 0 };
    for (let at: Element | null = element; at && at !== document.body; at = at.parentElement) {
      const z = zOf(at);
      if (z !== null) out = { at, z };
    }
    return out;
  };
  const [one, two] = [layer(a), layer(b)];
  if (one.at === two.at || one.at.contains(two.at) || two.at.contains(one.at)) throw new Error("one stacking context: compare inside it");
  if (one.z !== two.z) return one.z > two.z;
  return (two.at.compareDocumentPosition(one.at) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

it("the bell's panel opens above a pool's Tasks panel, whole and usable", async () => {
  applySummary(standInNeeds([{ key: "s:1", kind: "asks", title: "“Docs bot” needs your reply", detail: "docs-site · Claude app", project_id: "proj-docs", project: "docs-site", target, action: "open" }]));
  render(
    <div className="flex min-w-0 flex-1 flex-col">
      <AppHeader title="Pools" global="pools" page="pool" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} />
      <main>
        <div className="relative min-w-0 flex-1">
          <PoolPanel panel="tasks" poolId="pool-team" taskKey={0} sessions={[]} projects={[]} members={[]} now={Date.now()} onClose={() => {}} />
        </div>
      </main>
    </div>,
  );
  const panel = screen.getByTestId("pool-panel-tasks-open");
  fireEvent.click(await screen.findByTestId("notification-bell"));
  const centre = await screen.findByTestId("notification-centre");
  expect(paintsAbove(centre, panel)).toBe(true);
  // The page under it is inert; the centre and its rows are not.
  expect(panel.closest("[inert]")).not.toBeNull();
  expect(centre.closest("[inert]")).toBeNull();
  expect(within(centre).getByTestId("notices-needs").textContent).toContain("Docs bot");
});

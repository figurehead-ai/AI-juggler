/**
 * The coordinator's critique of the D-126 canvas, on the gallery's own
 * pool (the Checkout team: six sessions, four need you): the page's two
 * views and the ways between them, as the mock-ups have them —
 *
 * - setting up: the checklist, "Setting up · n of 7", Add a member;
 * - running (a session has joined and setup is done): the calm overview
 *   (Now, Tasks, Lead, On their own, Messages, Edit setup), no palette;
 * - hidden by the user: running, kept for the pool;
 * - Edit setup: the checklist again, every step with its Change.
 */
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The gallery's seams, set before the preview's fixtures are read (as
// the gallery's page sets them before the app loads).
const T0 = await vi.hoisted(async () => {
  const at = Date.now();
  const { canvasSeed } = await import("../../../tests/visual/fixtures/pool-canvas");
  Object.assign(window, canvasSeed(at));
  return at;
});

import { canvasSeed } from "../../../tests/visual/fixtures/pool-canvas";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { primariesIn } from "../../components/primitives";
import { PoolPage } from "../PoolPage";
import { resetCanvasTests } from "./canvas.test-kit";

const page = () => (
  <ConversationsProvider want={null}>
    <ActionHostProvider host={{}}>
      <PoolPage poolId="pool-checkout" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} now={T0} />
    </ActionHostProvider>
  </ConversationsProvider>
);

async function open(bridge: Record<string, unknown> = { asked: true, on: true }) {
  Object.assign(window, { ...canvasSeed(T0), __AIJ_AGENT_BRIDGE__: bridge });
  resetPreviewAgentBridge();
  await act(async () => {
    await refreshPools();
  });
  return render(page());
}

beforeEach(() => {
  resetCanvasTests();
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  resetPreviewAgentBridge();
});

describe("the gallery's Checkout team, its views (D-126 §3)", () => {
  it("running: a session has joined and setup is done — the calm overview, no palette, no progress", async () => {
    const view = await open();
    const running = await view.findByTestId("pool-running");
    expect([...running.querySelectorAll("dt")].map((term) => term.textContent)).toEqual(["Now", "Tasks", "Lead", "On their own", "Messages this week"]);
    expect(within(running).getByText("Planner")).toBeTruthy();
    expect(view.getByRole("button", { name: "Edit setup" })).toBeTruthy();
    for (const old of ["pool-summary", "pool-ready", "pool-checklist", "pool-header-setup", "canvas-palette"]) expect(view.queryByTestId(old)).toBeNull();
  });

  it("setting up: the team tools not on — the checklist, the header's progress, Add a member, one primary", async () => {
    const view = await open({ asked: false, on: false });
    await view.findByTestId("pool-checklist");
    expect(view.getByTestId("pool-header-setup").textContent).toBe("Setting up · 6 of 7");
    expect(within(view.getByTestId("canvas-palette")).getByRole("button", { name: "Add a member" })).toBeTruthy();
    expect(view.queryByTestId("pool-running")).toBeNull();
    expect(primariesIn(view.container)).toHaveLength(1);
  });

  it("hidden by the user: running, kept for the pool when it's opened again", async () => {
    const view = await open({ asked: false, on: false });
    fireEvent.click(await view.findByRole("button", { name: "Hide setup" }));
    expect(await view.findByTestId("pool-running")).toBeTruthy();
    expect(view.getByTestId("pool-setup-hidden").textContent).toBe("Setup hidden: 6 of 7 done.");
    expect(view.queryByTestId("canvas-palette")).toBeNull();
    cleanup();
    const again = render(page());
    expect(await again.findByTestId("pool-running")).toBeTruthy();
  });

  it("Edit setup: the checklist again, every step with its Change; Hide setup goes back", async () => {
    const view = await open();
    fireEvent.click(await view.findByRole("button", { name: "Edit setup" }));
    const ready = await view.findByTestId("pool-ready");
    expect(within(ready).getAllByTestId("pool-checklist-step")).toHaveLength(7);
    // Every step done: it says so, never "Setting up · 7 of 7" (the rc3
    // baseline review).
    expect(view.getByTestId("pool-header-setup").textContent).toBe("Setup done");
    expect(within(view.getByTestId("canvas-palette")).getByRole("button", { name: "Add a member" })).toBeTruthy();
    fireEvent.click(view.getByRole("button", { name: "Hide setup" }));
    expect(await view.findByTestId("pool-running")).toBeTruthy();
    // A done pool with a session in it runs anyway: nothing is kept hidden.
    expect(view.queryByTestId("pool-setup-hidden")).toBeNull();
  });
});

/**
 * SK48-5 (the 0.4.8 soak, P6 at 1701 × 1348): with Add a member open, its
 * panel covered the left edge of the planned team's This Mac card. The
 * planned team stood at a fixed centre; it's fitted beside the open panel
 * now, as the members' diagram is, from a small window's canvas to a
 * large one's.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_ACCOUNTS, MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { forgetSharedReads } from "../../lib/shared-read";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { PoolPage } from "../PoolPage";
import { NODE_W } from "./layout";
import { resetCanvasTests, seams } from "./canvas.test-kit";

const POOL = "pool-mac";
const rule = (member: string, kind: string, role: string) => ({ member, kind, role, level: null, joined_at: 1, sessions: 0, session_ids: [] });
/** The open panel's right edge: 12 px in, 240 px wide (`w-60`). */
const PANEL_RIGHT = 12 + 240;

let canvasSize = { w: 0, h: 0 };
const sized = (element: Element, side: "w" | "h") => (element.getAttribute("aria-label")?.endsWith(": diagram") ? canvasSize[side] : 0);
const width = Object.getOwnPropertyDescriptor(Element.prototype, "clientWidth")!;
const height = Object.getOwnPropertyDescriptor(Element.prototype, "clientHeight")!;

async function openPlanned() {
  // Build Mac's planned team: no lead yet, This Mac and everyone in its project.
  seams().__AIJ_POOLS__ = { pools: [{ id: POOL, name: "build team", rules: [] }], members: [] };
  seams().__AIJ_POOL_RULES__ = { [POOL]: { members: [rule("mac:this", "mac", "member"), rule("all", "everyone", "member"), rule("app:cli:claude-work", "app", "member")], talk: [], lead_next: null } };
  await act(async () => {
    await refreshPools();
  });
  return render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId={POOL} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} now={Date.now()} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

beforeEach(() => {
  resetCanvasTests();
  resetPreviewPoolModel();
  forgetSharedReads();
  Object.defineProperty(Element.prototype, "clientWidth", { configurable: true, get(this: Element) { return sized(this, "w"); } });
  Object.defineProperty(Element.prototype, "clientHeight", { configurable: true, get(this: Element) { return sized(this, "h"); } });
});
afterEach(() => {
  cleanup();
  Object.defineProperty(Element.prototype, "clientWidth", width);
  Object.defineProperty(Element.prototype, "clientHeight", height);
  delete seams().__AIJ_POOL_RULES__;
  resetPreviewPoolModel();
});

/** The planned team's cards' left and right edges on screen. */
function edges(): { left: number; right: number } {
  const transform = screen.getByTestId("planned-team").style.transform;
  const [, x, , scale] = /translate\(([-\d.]+)px, ([-\d.]+)px\) scale\(([\d.]+)\)/.exec(transform)!;
  const lefts = screen.getAllByTestId("planned-card").map((card) => parseFloat(card.style.left));
  return { left: Number(x) + Math.min(...lefts) * Number(scale), right: Number(x) + (Math.max(...lefts) + NODE_W) * Number(scale) };
}

describe("the planned team beside the open Add a member (SK48-5)", () => {
  // The canvas in a 1440 × 900 window and in the walk's 1701 × 1348 (the
  // rail and the side panel open: about 1020 px of canvas).
  it.each([
    { w: 800, h: 620 },
    { w: 1020, h: 1180 },
  ])("on a $w × $h canvas, no card is under the panel", async (size) => {
    canvasSize = size;
    await openPlanned();
    await screen.findByTestId("planned-team");
    fireEvent.click(screen.getByTestId("canvas-add-member"));
    await screen.findByTestId("canvas-palette-body");
    const { left, right } = edges();
    expect(left).toBeGreaterThanOrEqual(PANEL_RIGHT);
    // And the whole row is in the canvas.
    expect(right).toBeLessThanOrEqual(size.w);
  });
});

/**
 * P5 step 4 (0.4.1): a pool whose only member just left still says who
 * left. The Live stream is read and shown for a pool with no members while
 * it has lines; an empty pool with none keeps its own teach state.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { PoolPage } from "../PoolPage";
import { resetCanvasTests, seams } from "./canvas.test-kit";

const LEFT = "«Untitled session · web» left: its account is now Other";

async function openEmpty() {
  seams().__AIJ_POOLS__ = { pools: [{ id: "pool-careful", name: "Careful", rules: [] }], members: [] };
  await act(async () => {
    await refreshPools();
  });
  return render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId="pool-careful" sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

beforeEach(resetCanvasTests);
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_POOL_CANVAS__");
});

describe("a pool's Live stream with no members (P5 step 4)", () => {
  it("shows who left, though no member is left to draw", async () => {
    seams().__AIJ_POOL_CANVAS__ = {
      "pool-careful": { stream: [{ at_ms: Date.now() - 30_000, words: LEFT, source: "member", session_id: null, other_session_id: null }] },
    };
    const view = await openEmpty();
    const live = await view.findByTestId("inspect-stream");
    await waitFor(() => expect(live.textContent).toContain(LEFT));
    // The same list the Live section always is, read out as lines arrive.
    expect(live.querySelector("ol[aria-live='polite']")).not.toBeNull();
    // Nothing to select on an empty canvas, so no hint to.
    expect(live.textContent).not.toContain("Select a member");
  });

  it("an empty pool with nothing said keeps its own empty state", async () => {
    const view = await openEmpty();
    await view.findByTestId("inspect-activity");
    expect(view.queryByTestId("inspect-stream")).toBeNull();
  });
});

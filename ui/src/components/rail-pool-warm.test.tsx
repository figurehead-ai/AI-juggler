/**
 * X2 (the rc26 walk): a 50-member pool took 120–140 ms from the click to
 * its canvas, its two reads made only once its page opened. A pool's rail
 * header (the Pool lens), pointed at or focused, reads it ahead as its
 * card on the Pools screen does: one path, `warmPool` → `prefetchPool`.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../preview/fixtures";
import { resetPreviewPools } from "../lib/ipc/pools";
import { forgetPaged } from "../lib/paged";
import { refreshPools, resetPools } from "../lib/pools";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { forgetCanvases, rememberedCanvas } from "../pages/pool/prefetch";
import { ActionHostProvider } from "./ItemMenu";
import { InstanceBoard } from "./WorkflowRail";

const [first, second] = MOCK_SESSIONS;

beforeEach(() => {
  resetPreviewPools();
  resetPools();
  forgetPaged();
  forgetCanvases();
  resetRailViewCache();
  setRailView(DEFAULT_VIEW);
  Reflect.set(window, "__AIJ_POOLS__", {
    pools: [{ id: "pool-checkout", name: "Checkout redesign" }],
    members: [
      { session_id: first!.id, pool_id: "pool-checkout", role: "lead" },
      { session_id: second!.id, pool_id: "pool-checkout", role: "member" },
    ],
  });
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  resetRailViewCache();
  Reflect.deleteProperty(window, "__AIJ_POOLS__");
});

describe("a pool's rail header reads its pool ahead", () => {
  it("when hovered, and when focused", async () => {
    await act(async () => {
      await refreshPools();
    });
    const view = render(
      <ConversationsProvider>
        <ActionHostProvider host={{ openPool: () => undefined }}>
          <InstanceBoard sessions={MOCK_SESSIONS} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="compact" freshness={{}} now={Date.now()} projects={MOCK_PROJECTS} snooze={{}} onSnooze={() => {}} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    const lens = view.getByRole("radiogroup", { name: "Group sessions by" });
    fireEvent.click(await within(lens).findByRole("radio", { name: "Pool" }));
    const header = (await view.findAllByTestId("place-header")).find((one) => one.textContent?.startsWith("Checkout redesign"))!;
    expect(rememberedCanvas("pool-checkout")).toBeNull();
    fireEvent.pointerEnter(header);
    await waitFor(() => expect(rememberedCanvas("pool-checkout")?.pool_id).toBe("pool-checkout"));
    forgetCanvases();
    fireEvent.focus(within(header).getByTestId("place-page"));
    await waitFor(() => expect(rememberedCanvas("pool-checkout")?.pool_id).toBe("pool-checkout"));
  });
});

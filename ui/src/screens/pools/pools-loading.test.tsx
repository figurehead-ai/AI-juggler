/**
 * AJ-E1552 — the Pools screen's cards draw a skeleton only while the
 * pools are read (Still loading after 3 s), then the screen's own empty
 * words, or an error with Retry that reads them again.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS } from "../../preview/fixtures";
import * as ipc from "../../lib/ipc/pools";
import { forgetPaged } from "../../lib/paged";
import { resetPools } from "../../lib/pools";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { PoolsScreen } from "./PoolsScreen";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
  ipc.resetPreviewPools();
  resetPools();
  forgetPaged();
});
afterEach(() => {
  cleanup();
  resetPools();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const pools = () => render(<PoolsScreen sessions={[]} projects={MOCK_PROJECTS} accounts={[]} />);

describe("the pools", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "poolsList").mockReturnValue(never());
    pools();
    await expectSlowSkeleton("pools-loading");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "poolsList").mockResolvedValue({ pools: [], placements: [] });
    pools();
    await expectEmpty("pools-empty", "No pool has anyone in it yet.", "pools-loading");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "poolsList").mockRejectedValue(new Error("down"));
    pools();
    await expectErrorRetry("the pools", read, "pools-loading");
    expect(screen.queryByText("down")).toBeNull();
  });
});

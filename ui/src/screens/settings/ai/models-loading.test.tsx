/**
 * AJ-E1552 — Settings › AI › Models draws its skeleton only while the
 * models list loads (Still loading after 3 s), and an error with Retry
 * that reads again when the read fails. The list is never empty (no pick
 * is the "nothing to recommend" card), so it has no empty state here.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, it, vi } from "vitest";
import * as ipc from "../../../lib/ipc/models";
import { expectErrorRetry, expectSlowSkeleton, never } from "../../../test/loading-kit";
import { ModelsPane } from "./ModelsPane";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("the models list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "aiModelsCatalog").mockReturnValue(never());
    render(<ModelsPane />);
    await expectSlowSkeleton("models-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "aiModelsCatalog").mockRejectedValue(new Error("down"));
    render(<ModelsPane />);
    await expectErrorRetry("the AI models", read, "models-skeleton");
  });
});

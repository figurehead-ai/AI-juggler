/**
 * AJ-E1511 vitest — "Start from a saved handover…": a pack is checked and
 * shown (where it came from, its fidelity, what carries and doesn't, what
 * was withheld) before anything starts; a refused pack says why; the
 * pack's own folder becomes where to work; starting needs a folder and
 * passes the pack checked at preview.
 */
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PackPicker, usePack, type PackChoice } from "./OpenPack";

const w = () => window as unknown as Record<string, unknown>;

afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_PACK_DIR__", "__AIJ_PACK__", "__AIJ_PACK_ERROR__", "__AIJ_PACK_STARTED__"]) delete w()[key];
});

let latest: PackChoice | null = null;
function Harness({ onFolder }: { onFolder: (folder: string) => void }) {
  const pack = usePack(onFolder);
  latest = pack;
  return <PackPicker pack={pack} />;
}

describe("AJ-E1511 — Start from a context pack", () => {
  it("shows what the pack holds before anything starts, and offers its folder", async () => {
    w().__AIJ_PACK_DIR__ = "/Users/dev/exports/pack-1";
    const onFolder = vi.fn();
    const { getByTestId } = render(<Harness onFolder={onFolder} />);
    await act(async () => {
      fireEvent.click(getByTestId("pack-choose"));
    });
    const summary = getByTestId("pack-summary");
    expect(summary.textContent).toContain("Claude Code on Work Max");
    expect(summary.textContent).toContain("Full story, fresh start · Beta");
    expect(summary.textContent).toContain("the conversation (24 turns), 1 rules file, the uncommitted changes");
    expect(summary.textContent).toContain("Subagents' work");
    expect(getByTestId("pack-withheld").textContent).toBe("2 API keys, when it was made. They stay out.");
    expect(within(summary).getByLabelText("The handover's brief (read-only)").textContent).toContain("Refunds");
    expect(onFolder).toHaveBeenCalledWith("/Users/you/Dev/payments-api");
    expect(w().__AIJ_PACK_STARTED__).toBeUndefined();
    fireEvent.click(getByTestId("pack-clear"));
    expect(getByTestId("pack-choose")).toBeTruthy();
  });

  it("says why a pack is refused, and starts nothing", async () => {
    w().__AIJ_PACK_DIR__ = "/Users/dev/exports/pack-2";
    w().__AIJ_PACK_ERROR__ = "this handover was changed after it was made: conversation.md doesn't match";
    const { getByTestId, queryByTestId } = render(<Harness onFolder={() => {}} />);
    await act(async () => {
      fireEvent.click(getByTestId("pack-choose"));
    });
    expect(getByTestId("pack-error").textContent).toContain("changed after it was made");
    expect(queryByTestId("pack-summary")).toBeNull();
  });

  it("starts only with a folder, with the pack checked at preview", async () => {
    w().__AIJ_PACK_DIR__ = "/Users/dev/exports/pack-1";
    const { getByTestId } = render(<Harness onFolder={() => {}} />);
    await act(async () => {
      fireEvent.click(getByTestId("pack-choose"));
    });
    await expect(latest!.start("p-1", null)).rejects.toContain("pick where the session works");
    await latest!.start("p-1", "/Users/you/Dev/payments-api");
    expect(w().__AIJ_PACK_STARTED__).toEqual({
      path: "/Users/dev/exports/pack-1",
      digest: "preview",
      profileId: "p-1",
      folder: "/Users/you/Dev/payments-api",
    });
  });
});

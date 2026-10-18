import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RecoverySummaryDialog } from "./RecoverySummaryDialog";
import type { RecoverySummaryData } from "../lib/bridge";

/**
 * AJ-E510 Req.3 evidence (the one-time summary): the crash narrative
 * renders the EXACT backend template (the drill asserts the same
 * string in Rust — one source of truth for the words), with modal
 * dialog semantics, focus moving into the dialog on open, and BOTH
 * dismiss paths (Esc, "Got it"). The shown-once rule itself is
 * bridge-side (`recovery_summary_consumed`), covered by the Rust
 * drill; here the seam must return the pending summary verbatim.
 */

afterEach(cleanup);

const SUMMARY: RecoverySummaryData = {
  summary:
    "2 Instances re-adopted, 1 helpers left behind, 1 processes could not be verified and were left alone.",
  adopted: 2,
  left_behind: 1,
  unverifiable: 1,
};

describe("RecoverySummaryDialog (AJ-E510)", () => {
  it("renders the exact backend template inside a labelled modal dialog", () => {
    const { getByTestId, getByRole } = render(
      <RecoverySummaryDialog summary={SUMMARY} onDismiss={() => {}} />,
    );
    const dialog = getByRole("dialog", { name: "Restart recovery summary" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(getByTestId("recovery-summary-text").textContent).toBe(SUMMARY.summary);
    // The suspension fact rides along — the user must know automation
    // paused itself for 24 h after a crash.
    expect(getByTestId("recovery-summary").textContent).toContain(
      "Automatic cleanup stays suspended for 24 hours",
    );
  });

  it("moves focus into the dialog on open", () => {
    const { getByTestId } = render(
      <RecoverySummaryDialog summary={SUMMARY} onDismiss={() => {}} />,
    );
    expect(document.activeElement).toBe(getByTestId("recovery-summary"));
  });

  it("dismisses via Got it", () => {
    const onDismiss = vi.fn();
    const { getByTestId } = render(
      <RecoverySummaryDialog summary={SUMMARY} onDismiss={onDismiss} />,
    );
    fireEvent.click(getByTestId("recovery-summary-dismiss"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("dismisses via Escape (and only Escape)", () => {
    const onDismiss = vi.fn();
    const { getByTestId } = render(
      <RecoverySummaryDialog summary={SUMMARY} onDismiss={onDismiss} />,
    );
    fireEvent.keyDown(getByTestId("recovery-summary"), { key: "Enter" });
    expect(onDismiss).not.toHaveBeenCalled();
    fireEvent.keyDown(getByTestId("recovery-summary"), { key: "Escape" });
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("returns focus to the board on dismissal (WCAG 2.4.3)", () => {
    const board = document.createElement("main");
    board.tabIndex = -1;
    document.body.appendChild(board);
    const { getByTestId, unmount } = render(
      <RecoverySummaryDialog summary={SUMMARY} onDismiss={() => {}} />,
    );
    expect(document.activeElement).toBe(getByTestId("recovery-summary"));
    fireEvent.click(getByTestId("recovery-summary-dismiss"));
    unmount();
    expect(document.activeElement).toBe(board);
    board.remove();
  });
});

describe("recovery bridge seam (AJ-E510)", () => {
  // jsdom has no backend, so the seam exercises the window fallback —
  // the same seam the visual harness drives. The Tauri invoke path is
  // the one-liner beside it; command registration is covered in Rust.
  it("returns the pending summary verbatim, null once absent", async () => {
    const bridge = await import("../lib/bridge");
    (window as unknown as Record<string, unknown>).__AIJ_RECOVERY__ = SUMMARY;
    try {
      expect(await bridge.recoverySummaryGet()).toEqual(SUMMARY);
    } finally {
      delete (window as unknown as Record<string, unknown>).__AIJ_RECOVERY__;
    }
    expect(await bridge.recoverySummaryGet()).toBeNull();
    await expect(bridge.recoverySummaryDismiss()).resolves.toBeUndefined();
  });
});

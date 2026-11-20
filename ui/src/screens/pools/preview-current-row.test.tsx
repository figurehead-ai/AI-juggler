/**
 * The visual review of 0.4.7 batch 1 (98-pool-setup): on a 1280×800
 * window New pool's side summary scrolls, and the step's own row (On
 * their own) sat below its edge, a 3 px sliver in view. Whenever the step
 * changes, its row is scrolled into view, in the rows' box only; and the
 * box says when it has more below (the kit's chip, AJ-E2108).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { NewPoolPreview, type PreviewRow } from "./NewPoolPreview";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const ids = ["where", "who", "lead", "everyone", "talk", "own"] as const;
const rows: PreviewRow[] = ids.map((id) => ({ id, label: id, value: id, pending: false, prefilled: false }));

/** A box 200 px tall showing rows 40 px each, scrolled by `scrollTop`. */
function geometry(): void {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const box = this.closest<HTMLElement>("[data-testid='new-pool-preview-scroll']");
    const top = 100 - (box?.scrollTop ?? 0);
    if (this.dataset.testid === "new-pool-preview-scroll") return DOMRect.fromRect({ x: 0, y: 100, width: 300, height: 200 });
    const at = ids.findIndex((id) => id === this.dataset.row);
    return DOMRect.fromRect({ x: 0, y: top + Math.max(at, 0) * 40 + 120, width: 300, height: 40 });
  });
}

describe("New pool's side summary keeps the step's own row in view", () => {
  it("scrolls the current row into view when the step changes, and back up for an earlier one", () => {
    geometry();
    const view = render(<NewPoolPreview name="web team" diagram={null} rows={rows} current="where" />);
    const box = screen.getByTestId("new-pool-preview-scroll");
    expect(box.scrollTop).toBe(0);
    // "own" is the sixth row: its foot at 100 + 120 + 6 × 40 = 460, the box's at 300.
    view.rerender(<NewPoolPreview name="web team" diagram={null} rows={rows} current="own" />);
    expect(box.scrollTop).toBe(160);
    // Back to Where: just enough that its top meets the box's (its row
    // sits 120 px down the list).
    view.rerender(<NewPoolPreview name="web team" diagram={null} rows={rows} current="where" />);
    expect(box.scrollTop).toBe(120);
  });
});

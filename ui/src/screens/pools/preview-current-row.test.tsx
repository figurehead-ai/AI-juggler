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
  vi.unstubAllGlobals();
});

const ids = ["where", "who", "lead", "everyone", "talk", "own"] as const;
const rows: PreviewRow[] = ids.map((id) => ({ id, label: id, value: id, pending: false, prefilled: false }));

/** A box 200 px tall showing rows 40 px each (`tall` rows 80), scrolled
 * by `scrollTop`. */
function geometry(tall: ReadonlySet<string> = new Set()): void {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const box = this.closest<HTMLElement>("[data-testid='new-pool-preview-scroll']");
    const top = 100 - (box?.scrollTop ?? 0);
    if (this.dataset.testid === "new-pool-preview-scroll") return DOMRect.fromRect({ x: 0, y: 100, width: 300, height: 200 });
    const at = Math.max(ids.findIndex((id) => id === this.dataset.row), 0);
    const above = ids.slice(0, at).reduce((sum, id) => sum + (tall.has(id) ? 80 : 40), 0);
    return DOMRect.fromRect({ x: 0, y: top + above + 120, width: 300, height: tall.has(ids[at]!) ? 80 : 40 });
  });
}

/** ResizeObserver's callbacks, run by hand. */
function observers(): { grow: () => void } {
  const seen: (() => void)[] = [];
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(run: () => void) {
        seen.push(run);
      }
      observe(): void {}
      disconnect(): void {}
    },
  );
  return { grow: () => seen.forEach((run) => run()) };
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

  it("scrolls again when the step's row grows after a choice, and the last row to the list's very end", () => {
    const resize = observers();
    geometry();
    const view = render(<NewPoolPreview name="web team" diagram={null} rows={rows} current="talk" />);
    const box = screen.getByTestId("new-pool-preview-scroll");
    // "talk", the fifth row: its foot at 100 + 120 + 5 × 40 = 420.
    expect(box.scrollTop).toBe(120);
    // A choice gives it a second line: 80 px tall, cut at the box's edge.
    vi.restoreAllMocks();
    geometry(new Set(["talk"]));
    resize.grow();
    expect(box.scrollTop).toBe(160);
    // The last row: to the very end (its foot needs 200; the end is 220).
    Object.defineProperty(box, "scrollHeight", { value: 420, configurable: true });
    Object.defineProperty(box, "clientHeight", { value: 200, configurable: true });
    view.rerender(<NewPoolPreview name="web team" diagram={null} rows={rows} current="own" />);
    expect(box.scrollTop).toBe(220);
  });

  it("says \"More below\" only when a row is hidden, never for the list's own bottom room", () => {
    const view = render(<NewPoolPreview name="web team" diagram={null} rows={rows} current="where" />);
    const box = screen.getByTestId("new-pool-preview-scroll");
    // The rows' bottom room is outside the box.
    expect(screen.getByTestId("new-pool-preview-rows").className).not.toMatch(/\bpb-/);
    expect(box.nextElementSibling?.getAttribute("data-testid")).toBe("new-pool-preview-rows-foot");
    Object.defineProperty(box, "clientHeight", { value: 200, configurable: true });
    Object.defineProperty(box, "scrollHeight", { value: 200, configurable: true });
    view.rerender(<NewPoolPreview name="web team" diagram={null} rows={rows} current="lead" />);
    expect(screen.queryByTestId("new-pool-preview-more")).toBeNull();
    Object.defineProperty(box, "scrollHeight", { value: 240, configurable: true });
    view.rerender(<NewPoolPreview name="web team" diagram={null} rows={rows} current="who" />);
    expect(screen.getByTestId("new-pool-preview-more")).toBeTruthy();
  });
});

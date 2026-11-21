/**
 * 0.5.0 (D-142, GAPS A1–A2, DLG-1 to DLG-4): the one dialog frame. Its
 * header and footer sit outside the body, which alone scrolls; each part
 * is marked for the layout check (tests/visual/dialog-fit.spec.ts, which
 * measures what jsdom can't); the footer's divider and the body's
 * `data-more` show while more is below; the overlay is drawn in the
 * page's body, so no panel clips it. `Modal` is a thin wrapper with a
 * fixed footer.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DialogActions, DialogFrame, DialogOverlay } from "./DialogFrame";
import { Modal } from "./form";
import { Button, primariesIn } from "./primitives";

afterEach(cleanup);

/** jsdom lays nothing out: give the body a size and a scroll height. */
function sized(body: HTMLElement, { scrollHeight, clientHeight, scrollTop = 0 }: { scrollHeight: number; clientHeight: number; scrollTop?: number }) {
  Object.defineProperty(body, "scrollHeight", { configurable: true, value: scrollHeight });
  Object.defineProperty(body, "clientHeight", { configurable: true, value: clientHeight });
  body.scrollTop = scrollTop;
  fireEvent.scroll(body);
}

describe("DialogFrame", () => {
  it("marks one header, one body and one footer, the footer outside the body", () => {
    const view = render(
      <DialogFrame aria-label="Test" data-testid="frame" header={<h2>Title</h2>} footer={<Button variant="primary">Go</Button>}>
        <p>Body</p>
      </DialogFrame>,
    );
    const dialog = view.getByTestId("frame");
    const parts = (name: string) => dialog.querySelectorAll(`[data-dialog-part='${name}']`);
    expect(parts("header")).toHaveLength(1);
    expect(parts("body")).toHaveLength(1);
    expect(parts("footer")).toHaveLength(1);
    const body = parts("body")[0]!;
    expect(body.className).toMatch(/overflow-y-auto/);
    expect(body.className).toMatch(/min-h-0 flex-1/);
    expect(body.contains(view.getByRole("button", { name: "Go" }))).toBe(false);
    expect(body.contains(view.getByRole("heading", { name: "Title" }))).toBe(false);
    // The cap: never taller than min(92vh, 760px) and the window less its gutter.
    expect(dialog.className).toContain("max-h-[min(92vh,760px,calc(100vh_-_2rem))]");
  });

  it("shows the footer's divider and marks the body while more is below, and its top once scrolled", () => {
    const view = render(
      <DialogFrame aria-label="Test" data-testid="frame" header={<h2>Title</h2>} footer={<Button variant="primary">Go</Button>}>
        <p>Body</p>
      </DialogFrame>,
    );
    const body = view.getByTestId("frame").querySelector<HTMLElement>("[data-dialog-part='body']")!;
    const footer = view.getByTestId("frame").querySelector<HTMLElement>("[data-dialog-part='footer']")!;
    expect(footer.className).toContain("border-transparent");
    act(() => sized(body, { scrollHeight: 900, clientHeight: 400 }));
    expect(body.hasAttribute("data-more")).toBe(true);
    expect(body.hasAttribute("data-scrolled")).toBe(false);
    expect(footer.className).toContain("border-hairline");
    act(() => sized(body, { scrollHeight: 900, clientHeight: 400, scrollTop: 500 }));
    expect(body.hasAttribute("data-more")).toBe(false);
    expect(body.hasAttribute("data-scrolled")).toBe(true);
  });

  it("takes a size class, and a caller's own cap in place of the default", () => {
    const view = render(
      <>
        <DialogFrame aria-label="m" size="M-cards" data-testid="m">
          <p />
        </DialogFrame>
        <DialogFrame aria-label="held" className="max-h-[76vh]" data-testid="held">
          <p />
        </DialogFrame>
      </>,
    );
    expect(view.getByTestId("m").className).toContain("max-w-[680px]");
    expect(view.getByTestId("held").className).not.toContain("max-h-[min(");
  });

  it("draws its overlay in the page's body, outside the panel that opened it", () => {
    const view = render(
      <section data-testid="panel">
        <DialogOverlay data-testid="overlay">
          <p>Inside</p>
        </DialogOverlay>
      </section>,
    );
    const overlay = view.getByTestId("overlay");
    expect(overlay.parentElement).toBe(document.body);
    expect(view.getByTestId("panel").contains(overlay)).toBe(false);
    // A 16 px gutter.
    expect(overlay.className).toContain("p-4");
  });

  it("keeps a flow's own actions at the body's foot", () => {
    const view = render(
      <DialogFrame aria-label="flow" data-testid="flow">
        <DialogActions>
          <Button>Back</Button>
        </DialogActions>
      </DialogFrame>,
    );
    const row = view.getByTestId("flow").querySelector("[data-dialog-actions]")!;
    expect(row.className).toMatch(/sticky bottom-0/);
  });
});

describe("Modal on DialogFrame", () => {
  it("puts its footer in the fixed footer, its one primary there, and closes on Escape", () => {
    const onClose = vi.fn();
    const view = render(
      <Modal
        title="Rename"
        onClose={onClose}
        footer={
          <>
            <Button variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button variant="primary" type="submit" form="rename-form">
              Save
            </Button>
          </>
        }
      >
        <form id="rename-form">
          <input aria-label="Name" />
        </form>
      </Modal>,
    );
    const dialog = view.getByRole("dialog", { name: "Rename" });
    const footer = dialog.querySelector("[data-dialog-part='footer']")!;
    expect(footer.contains(view.getByRole("button", { name: "Save" }))).toBe(true);
    expect(primariesIn(dialog)).toHaveLength(1);
    expect(view.getByRole("button", { name: "Save" }).getAttribute("form")).toBe("rename-form");
    // The title is on the type scale's section step.
    expect(view.getByRole("heading", { name: "Rename" }).className).toContain("type-section");
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});

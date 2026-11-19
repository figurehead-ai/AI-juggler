/**
 * 0.4.6 RC: closing a menu or popover handed focus back to its opener
 * while the opener was still inert (an inert element takes no focus), so
 * focus fell to the page. useInertOutside gives it back to the opener
 * once the page is let go. jsdom has no inert, so the lost focus is made
 * by hand (blur in the close), as the browser does it.
 */
import { act, cleanup, render } from "@testing-library/react";
import { useEffect, useRef, useState, type JSX } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { releaseAllInert, useInertOutside } from "./inert";

afterEach(() => {
  cleanup();
  releaseAllInert();
});

let close: () => void = () => {};

function Popover({ onClose }: { onClose: (close: () => void) => void }): JSX.Element {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement | null>(null);
  const panel = useRef<HTMLDialogElement | null>(null);
  useInertOutside(panel, open);
  useEffect(() => {
    onClose(() => {
      setOpen(false);
      // What the browser does to `button.focus()` while the button is inert.
      const active = document.activeElement;
      if (active instanceof HTMLElement) active.blur();
    });
  }, [onClose]);
  return (
    <div>
      <button ref={button} type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      <div>{open && (<dialog ref={panel} open aria-label="Panel">inside</dialog>)}</div>
    </div>
  );
}

describe("useInertOutside gives focus back to the opener", () => {
  it("when a close left focus on the page", () => {
    const view = render(<Popover onClose={(fn) => (close = fn)} />);
    const button = view.getByRole("button", { name: "Open" });
    button.focus();
    act(() => button.click());
    expect(view.getByRole("dialog")).toBeTruthy();
    act(() => close());
    expect(view.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(button);
  });
});

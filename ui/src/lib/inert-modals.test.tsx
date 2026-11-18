/**
 * The 0.4.6 rc2 walk (A8): a modal dialog only dimmed the window under it,
 * so an accessibility press on the New session sheet reached Home's
 * "Start" behind it. While a dialog shows, the window under it is inert;
 * when it goes, focus is back on what opened it. A dialog opened over
 * another stays usable and hands focus back to the one under it.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState, type JSX } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { releaseAllInert, useModalsInert } from "./inert";

afterEach(() => {
  cleanup();
  releaseAllInert();
});

const settle = () => act(async () => new Promise<void>((done) => queueMicrotask(done)));

function Shell(): JSX.Element {
  useModalsInert();
  const [first, setFirst] = useState(false);
  const [second, setSecond] = useState(false);
  return (
    <div>
      <main data-testid="page">
        <button type="button" onClick={() => setFirst(true)}>
          New session
        </button>
        <button type="button">Start</button>
      </main>
      {first && (
        <dialog open aria-modal="true" aria-label="New session">
          <button type="button" onClick={() => setSecond(true)}>
            Start it
          </button>
          <button type="button" onClick={() => setFirst(false)}>
            Cancel
          </button>
        </dialog>
      )}
      {second && (
        <dialog open aria-modal="true" aria-label="Busy folder">
          <button type="button" onClick={() => setSecond(false)}>
            Back
          </button>
        </dialog>
      )}
    </div>
  );
}

describe("a modal dialog's window is inert under it", () => {
  it("while it shows, and focus is back on its opener when it goes", async () => {
    render(<Shell />);
    const opener = screen.getByRole("button", { name: "New session" });
    opener.focus();
    fireEvent.click(opener);
    await settle();
    expect(screen.getByTestId("page").hasAttribute("inert")).toBe(true);
    expect(screen.getByRole("dialog", { name: "New session" }).hasAttribute("inert")).toBe(false);
    // Its close moves focus nowhere (the browser's lot while the opener is inert).
    const cancel = screen.getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.click(cancel);
    cancel.blur();
    await settle();
    expect(screen.getByTestId("page").hasAttribute("inert")).toBe(false);
    expect(document.activeElement).toBe(opener);
  });

  it("a dialog over a dialog is the one usable, then hands focus back", async () => {
    render(<Shell />);
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    await settle();
    fireEvent.click(screen.getByRole("button", { name: "Start it" }));
    await settle();
    expect(screen.getByRole("dialog", { name: "Busy folder", hidden: true }).closest("[inert]")).toBeNull();
    expect(screen.getByRole("dialog", { name: "New session", hidden: true }).hasAttribute("inert")).toBe(true);
    const back = screen.getByRole("button", { name: "Back", hidden: true });
    back.focus();
    fireEvent.click(back);
    back.blur();
    await settle();
    const under = screen.getByRole("dialog", { name: "New session" });
    expect(under.hasAttribute("inert")).toBe(false);
    expect(under.contains(document.activeElement)).toBe(true);
    expect(screen.getByTestId("page").hasAttribute("inert")).toBe(true);
  });

  it("gives focus to the opener drawn again while the dialog was open (Processes' Stop… sheet)", async () => {
    function List(): JSX.Element {
      useModalsInert();
      const [open, setOpen] = useState(false);
      const [draw, setDraw] = useState(0);
      return (
        <div>
          <main>
            <button key={draw} type="button" data-testid="row-8123" onClick={() => setOpen(true)}>
              Row
            </button>
          </main>
          {open && (
            <dialog open aria-modal="true" aria-label="Stop">
              <button
                type="button"
                onClick={() => {
                  setDraw((n) => n + 1); // the list draws its rows again
                  setOpen(false);
                }}
              >
                Cancel
              </button>
            </dialog>
          )}
        </div>
      );
    }
    render(<List />);
    const row = screen.getByTestId("row-8123");
    row.focus();
    fireEvent.click(row);
    await settle();
    const cancel = screen.getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.click(cancel);
    cancel.blur();
    await settle();
    expect(screen.getByTestId("row-8123")).not.toBe(row);
    expect(document.activeElement).toBe(screen.getByTestId("row-8123"));
  });

  it("never takes a dialog's own frame for its opener (a sheet that focuses the overlay around its box)", async () => {
    function Sheet(): JSX.Element {
      useModalsInert();
      const [open, setOpen] = useState(false);
      return (
        <div>
          <main>
            <button type="button" data-testid="row" onClick={() => setOpen(true)}>
              Row
            </button>
          </main>
          {open && (
            <div tabIndex={-1} data-testid="frame" ref={(frame) => frame?.focus()}>
              <dialog open aria-modal="true" aria-label="Stop">
                <button type="button" onClick={() => setOpen(false)}>
                  Cancel
                </button>
              </dialog>
            </div>
          )}
        </div>
      );
    }
    render(<Sheet />);
    const row = screen.getByTestId("row");
    row.focus();
    fireEvent.click(row);
    await settle();
    const cancel = screen.getByRole("button", { name: "Cancel" });
    fireEvent.click(cancel);
    await settle();
    expect(document.activeElement).toBe(screen.getByTestId("row"));
  });
});

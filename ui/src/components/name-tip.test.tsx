/**
 * 0.5.1 F61 — the folded rail's icons name themselves in a drawn tip, on
 * hover and on keyboard focus (WhyTip's pattern), never in a browser
 * `title`, which the app's web view shows only after a still hover and
 * never on focus.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TipButton } from "./NameTip";
import { RailHeader } from "./RailHeader";

afterEach(cleanup);

describe("a drawn name tip (F61)", () => {
  it("shows on hover and hides on leave; no title", () => {
    render(<TipButton tip="History" aria-label="History" data-testid="rail-history" onClick={vi.fn()}>H</TipButton>);
    const button = screen.getByTestId("rail-history");
    expect(button.getAttribute("title")).toBeNull();
    expect(screen.queryByRole("tooltip")).toBeNull();
    fireEvent.pointerEnter(button);
    expect(screen.getByTestId("rail-history-tip").textContent).toBe("History");
    expect(screen.getByRole("tooltip").textContent).toBe("History");
    fireEvent.pointerLeave(button);
    expect(screen.queryByTestId("rail-history-tip")).toBeNull();
  });

  it("shows on keyboard focus, and Escape closes it; a click's focus shows none", () => {
    // jsdom keeps no :focus-visible: here focus comes from the keyboard
    // until the test says it came from a click.
    let fromKeyboard = true;
    const matches: unknown = Object.getOwnPropertyDescriptor(Element.prototype, "matches")?.value;
    vi.spyOn(Element.prototype, "matches").mockImplementation(function (this: Element, selector: string) {
      if (selector === ":focus-visible") return fromKeyboard && this === document.activeElement;
      if (typeof matches !== "function") throw new Error("no Element.matches");
      return Boolean(Reflect.apply(matches, this, [selector]));
    });
    render(<TipButton tip="New project" aria-label="New project" data-testid="rail-new-project">P</TipButton>);
    const button = screen.getByTestId("rail-new-project");
    act(() => button.focus());
    expect(screen.getByTestId("rail-new-project-tip").textContent).toBe("New project");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("rail-new-project-tip")).toBeNull();
    act(() => button.blur());
    act(() => button.focus());
    expect(screen.getByTestId("rail-new-project-tip")).toBeTruthy();
    act(() => button.blur());
    expect(screen.queryByTestId("rail-new-project-tip")).toBeNull();
    fromKeyboard = false;
    act(() => button.focus());
    expect(screen.queryByTestId("rail-new-project-tip")).toBeNull();
    vi.restoreAllMocks();
  });

  it("with no tip it is the plain button", () => {
    render(<TipButton tip={null} data-testid="plain">New session</TipButton>);
    fireEvent.pointerEnter(screen.getByTestId("plain"));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("the folded rail's head names its toggle in a drawn tip", () => {
    render(<RailHeader minimal info={null} onToggle={vi.fn()} />);
    const toggle = screen.getByRole("button", { name: "Show the list" });
    expect(toggle.getAttribute("title")).toBeNull();
    fireEvent.pointerEnter(toggle);
    expect(screen.getByRole("tooltip").textContent).toMatch(/^Show the list \(.+\)$/);
  });
});

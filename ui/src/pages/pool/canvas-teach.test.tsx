/**
 * AJ-E3008 R1 (D-62 decision 18): the empty canvas teaches the order —
 * three numbered steps, the next one to do the only primary button, each
 * opening the control it needs, and "Use a template" a click away.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CanvasTeach, hasWhoIsIn } from "./CanvasTeach";

afterEach(cleanup);

const teach = (scopeChosen: boolean, membersAdded = false) => {
  const calls = { scope: vi.fn(), add: vi.fn(), template: vi.fn() };
  render(<CanvasTeach scope="payments-api" scopeChosen={scopeChosen} membersAdded={membersAdded} onScope={calls.scope} onAdd={calls.add} onTemplate={calls.template} inset={false} />);
  return calls;
};
const primaries = () => screen.getAllByRole("button").filter((button) => button.className.includes("bg-accent") || button.dataset.variant === "primary");

describe("the empty canvas teaches (AJ-E3008 R1)", () => {
  it("shows three steps; the first is the only primary until where it works is chosen", () => {
    const calls = teach(false);
    expect(screen.getAllByTestId("canvas-teach-step")).toHaveLength(3);
    expect(screen.getByTestId("canvas-empty").textContent).toContain("Three steps to a working pool");
    const first = screen.getByTestId("canvas-teach-scope");
    expect(primaries()).toEqual([first]);
    fireEvent.click(first);
    expect(calls.scope).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("canvas-teach-template"));
    expect(calls.template).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("canvas-teach-template").textContent).toBe("Use a template");
  });

  it("once where it works is chosen, adding is the one primary step", () => {
    const calls = teach(true);
    const adds = screen.getAllByTestId("canvas-teach-add");
    expect(primaries()).toEqual([adds[0]]);
    fireEvent.click(adds[0]!);
    expect(calls.add).toHaveBeenCalledTimes(1);
  });

  it("walk K1: a pool of 500 sessions, none drawn, has who's in it; its steps done, the card is gone", () => {
    // A project's pool: its sessions join through its project, no member of their own.
    expect(hasWhoIsIn(500, false)).toBe(true);
    expect(hasWhoIsIn(0, true)).toBe(true); // an App, Account, Tool or Mac brings them as they start
    expect(hasWhoIsIn(0, false)).toBe(false);
    teach(true, true);
    expect(screen.queryByTestId("canvas-empty")).toBeNull();
  });

  it("step 2 reads the pool: ticked once it has who's in it, while where it works isn't chosen", () => {
    teach(false, true);
    const marks = screen.getAllByTestId("canvas-teach-step").map((step) => step.querySelector("span")?.textContent);
    expect(marks).toEqual(["1", "✓", "3"]);
    expect(primaries()).toEqual([screen.getByTestId("canvas-teach-scope")]);
  });
});

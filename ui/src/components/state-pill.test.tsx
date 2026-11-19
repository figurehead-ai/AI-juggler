/**
 * D-126 — the fixed state pills and the one-primary rule: a state's
 * look is fixed by what it is (neutral, progress, done, asks, error),
 * the marks differ in shape; a primary Button marks itself so a view's
 * test can hold it to one.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { StatePill } from "./Pill";
import { Button, primariesIn } from "./primitives";

afterEach(cleanup);

describe("StatePill", () => {
  it("draws each state its one way", () => {
    render(
      <>
        <StatePill variant="neutral">web</StatePill>
        <StatePill variant="progress">Setting up · 5 of 6</StatePill>
        <StatePill variant="done">Done</StatePill>
        <StatePill variant="asks">Asks you</StatePill>
        <StatePill variant="error">Stopped with an error</StatePill>
      </>,
    );
    expect(screen.getByText("web").className).toContain("bg-surface-3");
    expect(screen.getByText("Setting up · 5 of 6").className).toContain("bg-accent-soft");
    expect(screen.getByText("Done").className).toContain("text-success");
    expect(screen.getByText("Asks you").className).toContain("text-warning");
    expect(screen.getByText("Stopped with an error").className).toContain("text-danger");
  });

  it("marks done and asks with a dot and an error with a stop mark, all hidden from screen readers", () => {
    render(
      <>
        <StatePill variant="done">Done</StatePill>
        <StatePill variant="error">Stopped with an error</StatePill>
        <StatePill variant="progress">Setting up</StatePill>
      </>,
    );
    expect(screen.getByText("Done").querySelector("span[aria-hidden='true']")).not.toBeNull();
    expect(screen.getByText("Stopped with an error").querySelector("svg[aria-hidden='true']")).not.toBeNull();
    expect(screen.getByText("Setting up").children).toHaveLength(0);
  });
});

describe("one filled primary per view", () => {
  it("a primary Button marks itself; the others don't", () => {
    const view = render(
      <div>
        <Button variant="ghost">Cancel</Button>
        <Button variant="secondary">Add, sign in later</Button>
        <Button variant="primary">Create and sign in</Button>
      </div>,
    );
    const primaries = primariesIn(view.container);
    expect(primaries).toHaveLength(1);
    expect(primaries[0]).toBe(screen.getByRole("button", { name: "Create and sign in" }));
  });
});

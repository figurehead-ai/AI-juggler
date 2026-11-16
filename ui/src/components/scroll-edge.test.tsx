/**
 * The pool inspector's body scrolled under its title and status pill with
 * nothing between (the review folder's connector-remove-refused shot):
 * it has the app's scroll edge now, shown only once it's scrolled. The
 * connector gallery checks the inspector's body has it, scrolled.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { ScrollEdge } from "./ScrollEdge";

afterEach(cleanup);

describe("ScrollEdge", () => {
  it("draws the app's edge only once its content scrolls under the head", () => {
    render(
      <ScrollEdge className="overflow-y-auto" data-testid="body">
        <p>Sections</p>
      </ScrollEdge>,
    );
    const body = screen.getByTestId("body");
    expect(body.classList.contains("scroll-edge")).toBe(true);
    expect(body.hasAttribute("data-scrolled")).toBe(false);
    body.scrollTop = 40;
    fireEvent.scroll(body);
    expect(body.hasAttribute("data-scrolled")).toBe(true);
    body.scrollTop = 0;
    fireEvent.scroll(body);
    expect(body.hasAttribute("data-scrolled")).toBe(false);
  });
});

/**
 * 0.4.1 (the maintainer's finding): the Floor's split button reads as one
 * control, its arrow inside it, apart from the avatar beside it. D-118's
 * mock (0.5): quiet chrome, a ghost with no border or fill at rest.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FloorButton } from "./FloorButton";

afterEach(cleanup);

describe("the Floor's split button", () => {
  it("is one ghost control with the arrow inside, lit as one, each half named and titled", () => {
    render(<FloorButton />);
    const split = screen.getByTestId("floor-split");
    const classes = split.className.split(" ");
    expect(classes).toEqual(expect.arrayContaining(["group/floor", "hover:bg-surface", "rounded-lg", "overflow-hidden"]));
    // Quiet at rest: no border, no fill.
    expect(classes.some((name) => /^(border|bg-)/.test(name))).toBe(false);
    const open = screen.getByTestId("floor-button");
    const menu = screen.getByTestId("floor-menu-button");
    expect(split.contains(open) && split.contains(menu)).toBe(true);
    expect(open.getAttribute("aria-label")).toBe("The Floor");
    expect(open.getAttribute("title")).toMatch(/^The Floor \(/);
    expect(menu.getAttribute("aria-label")).toBe("More of the Floor");
    expect(menu.getAttribute("title")).toBe("The Floor's menu: open it, its list, its settings");
    // Hit areas: the whole height of the control (36 px, the bell's), 28 + 24 px wide:
    // neither half under WCAG 2.5.8's 24 px target (axe target-size), the halves flush.
    expect(open.className).toContain("w-7");
    expect(menu.className).toContain("w-6");
  });

  it("sits apart from the avatar", async () => {
    const headerSource = (await import("./AppHeader.tsx?raw")).default;
    expect(headerSource).toMatch(/<FloorButton \/>\s*\{\/\*[^*]*\*\/\}\s*<span className="ml-2 flex[^"]*">\s*<UserMenu \/>/);
  });
});

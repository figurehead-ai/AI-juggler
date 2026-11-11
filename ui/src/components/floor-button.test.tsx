/**
 * 0.4.1 (the maintainer's finding): the Floor's split button reads as one
 * control, its arrow inside it, apart from the avatar beside it.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FloorButton } from "./FloorButton";

afterEach(cleanup);

describe("the Floor's split button", () => {
  it("is one bordered control with the arrow inside, each half named and titled", () => {
    render(<FloorButton />);
    const split = screen.getByTestId("floor-split");
    expect(split.className.split(" ")).toEqual(expect.arrayContaining(["border", "border-hairline", "rounded-lg", "overflow-hidden"]));
    const open = screen.getByTestId("floor-button");
    const menu = screen.getByTestId("floor-menu-button");
    expect(split.contains(open) && split.contains(menu)).toBe(true);
    expect(menu.className.split(" ")).toEqual(expect.arrayContaining(["border-l", "border-hairline"]));
    expect(open.getAttribute("aria-label")).toBe("The Floor");
    expect(open.getAttribute("title")).toMatch(/^The Floor \(/);
    expect(menu.getAttribute("aria-label")).toBe("More of the Floor");
    expect(menu.getAttribute("title")).toBe("The Floor's menu: open it, its list, its settings");
    // Hit areas: the whole height of the control, 36 + 24 px wide.
    expect(open.className).toContain("w-9");
    expect(menu.className).toContain("w-6");
  });

  it("sits apart from the avatar", async () => {
    const headerSource = (await import("./AppHeader.tsx?raw")).default;
    expect(headerSource).toMatch(/<FloorButton \/>\s*\{\/\*[^*]*\*\/\}\s*<span className="ml-2 flex">\s*<UserMenu \/>/);
  });
});

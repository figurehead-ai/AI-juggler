/** 0.3.2: a rail group header's right-click is its own menu, never the
 * web view's (the maintainer's 0.3.0 walk: Reload on the account header). */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PlaceHeader } from "./PlaceHeader";

afterEach(cleanup);

describe("0.3.2 a group header's menu", () => {
  it("right-click opens Open its page, Bring it forward and Fold, and each does its thing", () => {
    const onPage = vi.fn();
    const onOpen = vi.fn();
    const onToggle = vi.fn();
    render(<PlaceHeader place={null} label="Work" count={3} needs={0} folded={null} onToggle={onToggle} onOpen={onOpen} onPage={onPage} />);
    const header = screen.getByTestId("place-header");
    // fireEvent says false when the default (the web view's menu) was prevented.
    expect(fireEvent.contextMenu(header, { clientX: 20, clientY: 30 })).toBe(false);
    expect(screen.getByTestId("place-header-menu-open").textContent).toBe("Bring it forward");
    fireEvent.click(screen.getByTestId("place-header-menu-page"));
    expect(onPage).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("place-header-menu-page")).toBeNull();
    fireEvent.keyDown(header, { key: "F10", shiftKey: true });
    fireEvent.click(screen.getByTestId("place-header-menu-fold"));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });
});

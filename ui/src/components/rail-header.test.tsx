/**
 * 0.4.1 (the Floor session's walk): folded at 1024, the rail's head sat
 * under the window's traffic lights (the overlay title bar's top 28 px)
 * and the Dev mark landed on them. Folded, the logo and the mark start
 * below that band; unfolded, the head is as it was.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RailHeader } from "./RailHeader";
import type { AppInfo } from "../types/generated/AppInfo";

afterEach(cleanup);

const DEV: AppInfo = { version: "0.4.1", name: "AI Juggler Dev", edition: "dev", reserved_keys: [] };

describe("the rail's head", () => {
  it("folded, starts below the traffic lights, its Dev mark whole under the logo", () => {
    render(<RailHeader minimal info={DEV} onToggle={() => {}} />);
    const head = screen.getByTestId("rail-header").className.split(" ");
    expect(head).toEqual(expect.arrayContaining(["pt-7", "min-h-14", "flex-col"]));
    expect(screen.getByTestId("edition-badge").textContent).toBe("Dev");
  });

  it("unfolded, keeps its 56 px row", () => {
    render(<RailHeader minimal={false} info={DEV} onToggle={() => {}} />);
    const head = screen.getByTestId("rail-header").className.split(" ");
    expect(head).toContain("h-14");
    expect(head).not.toContain("pt-7");
  });

  it("unfolded, starts past the traffic lights; the name gives way before the Dev mark (0.5.1 F22)", () => {
    render(<RailHeader minimal={false} info={DEV} onToggle={() => {}} />);
    const head = screen.getByTestId("rail-header").className.split(" ");
    expect(head).toContain("pl-titlebar");
    expect(head).not.toContain("px-4");
    const name = screen.getByText("AI Juggler");
    expect(name.className.split(" ")).toEqual(expect.arrayContaining(["min-w-0", "truncate"]));
    expect(name.getAttribute("title")).toBe("AI Juggler");
    expect(screen.getByTestId("edition-badge").className.split(" ")).toContain("shrink-0");
  });
});

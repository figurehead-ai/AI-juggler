/**
 * 0.5.1 F64 (AJ-E1552, D-103.6, AJ-E1551) — Home's first frame is Home's
 * shape: its heading over D-118's two columns (Needs you now's need card
 * and the feed on the left; Your apps and Runway on the right), stacked
 * when narrow, cards by tone with no outlines (PAT-SURF-1).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { HomeLoading } from "./HomeLoading";
import { COLUMNS } from "../lib/home-layout";

afterEach(cleanup);

describe("Home's loading skeleton (F64)", () => {
  it("is Home's two columns, with the same grid", () => {
    render(<HomeLoading />);
    const root = screen.getByTestId("home-loading");
    expect(root.getAttribute("aria-label")).toBe("Loading Home");
    expect(root.getAttribute("data-layout")).toBe("columns");
    const columns = screen.getByTestId("home-loading-columns");
    expect(columns.className).toContain(COLUMNS);
    expect(columns.children).toHaveLength(2);
    expect(columns.children[0]!.contains(screen.getByTestId("home-loading-left"))).toBe(true);
    expect(columns.children[1]!.contains(screen.getByTestId("home-loading-right"))).toBe(true);
    expect(screen.getByTestId("home-loading-need")).toBeTruthy();
    expect(screen.getByTestId("home-loading-feed")).toBeTruthy();
  });

  it("draws its cards by tone: no outline anywhere", () => {
    render(<HomeLoading />);
    const outlined = [...screen.getByTestId("home-loading").querySelectorAll("*")].filter((node) => /(^|\s)border(\s|-)/.test(node.getAttribute("class") ?? ""));
    expect(outlined).toEqual([]);
  });
});

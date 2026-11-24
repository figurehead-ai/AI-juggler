/**
 * 0.5.1 F57 — the top bar keeps one row: the tabs that don't fit go
 * behind More, from the last, by the room the bar has (its fold point
 * counts the tabs it shows, Fleet's too).
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MainNav } from "./MainNav";
import { tabsThatFit } from "./use-tab-fold";
import { PRIMARY_NAV } from "../lib/nav";

afterEach(cleanup);

// Home · Projects · Pools · Tasks · Accounts · Usage · History · Fleet.
const WIDTHS = [48, 64, 46, 44, 70, 50, 56, 46];
const ALL = WIDTHS.reduce((sum, width) => sum + width, 0) + 2 * (WIDTHS.length - 1);

describe("how many tabs fit (F57)", () => {
  it("all of them when they fit, More not needed", () => {
    expect(tabsThatFit(WIDTHS, ALL, 2, 60)).toBe(8);
    expect(tabsThatFit(WIDTHS.slice(0, 7), ALL, 2, 60)).toBe(7);
  });

  it("Fleet's tab counts: the same room holds every tab without it, and folds one with it", () => {
    const room = ALL - 46;
    expect(tabsThatFit(WIDTHS.slice(0, 7), room, 2, 60)).toBe(7);
    // With Fleet, More (60) takes room too: Usage, History and Fleet go
    // behind it (More and six tabs need 394 px of the 392).
    expect(tabsThatFit(WIDTHS, room, 2, 60)).toBe(5);
  });

  it("the rest go behind More from the last, More counted, never fewer than keep", () => {
    // 48 + 2 + 64 + 2 + 46 = 162, with More 60 + 2: 224.
    expect(tabsThatFit(WIDTHS, 224, 2, 60)).toBe(3);
    expect(tabsThatFit(WIDTHS, 223, 2, 60)).toBe(2);
    expect(tabsThatFit(WIDTHS, 10, 2, 60, 2)).toBe(2);
  });
});

describe("More (F57)", () => {
  it("holds the folded tabs, each with its icon, lit when one of them is the page", () => {
    const onPick = vi.fn();
    render(<MainNav primary={PRIMARY_NAV} global={"history" as never} page="none" pairedCount={1} onPick={onPick} shown={5} />);
    const nav = screen.getByTestId("main-nav");
    expect(within(nav).getAllByRole("button").map((tab) => tab.textContent)).toEqual(["Home", "Projects", "Pools", "Tasks", "Accounts", "More"]);
    const more = screen.getByTestId("nav-more");
    expect(more.getAttribute("aria-current")).toBe("page");
    expect(screen.queryByTestId("nav-history")).toBeNull();
    fireEvent.click(more);
    const menu = screen.getByTestId("nav-more-menu");
    const lines = within(menu).getAllByRole("menuitem");
    expect(lines.map((line) => line.querySelector("[data-label]")?.textContent)).toEqual(["Usage", "History", "Fleet"]);
    expect(lines.every((line) => line.querySelector("svg[aria-hidden='true']") !== null)).toBe(true);
    fireEvent.click(screen.getByTestId("nav-more-fleet"));
    expect(onPick).toHaveBeenCalledWith("fleet");
    expect(screen.queryByTestId("nav-more-menu")).toBeNull();
  });

  it("no More while every tab fits", () => {
    render(<MainNav primary={PRIMARY_NAV} global={null} page="none" pairedCount={0} onPick={vi.fn()} />);
    expect(screen.queryByTestId("nav-more")).toBeNull();
    expect(within(screen.getByTestId("main-nav")).getAllByRole("button")).toHaveLength(7);
  });
});

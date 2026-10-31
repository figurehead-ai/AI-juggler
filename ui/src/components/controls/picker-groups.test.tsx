/**
 * 0.3.2 — the pickers rule (docs/plan/QUALITY.md "Finding things"): the
 * most used first, then All; empty ones last; search past 8; Enter picks
 * the first match; at most 60 lines drawn.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Picker, sections, type PickerOption } from "./Picker";

afterEach(cleanup);

const pool = (n: number, members: number): PickerOption => ({ value: `p${n}`, label: `Pool ${n}`, empty: members === 0 });
const OPTIONS = [pool(1, 0), pool(2, 3), pool(3, 0), pool(4, 1), pool(5, 0), pool(6, 0), pool(7, 0), pool(8, 0), pool(9, 2), pool(10, 0)];
const busy = new Set(["p2", "p4", "p9"]);
const options = OPTIONS.map((option) => ({ ...option, empty: !busy.has(option.value) }));

describe("0.3.2 the Picker: suggested, then All", () => {
  it("lists the group first, then All with the empty ones last", () => {
    const out = sections(options, [{ label: "With members", values: ["p2", "p9"] }], "", "All pools");
    expect(out.map((section) => section.label)).toEqual(["With members", "All pools"]);
    expect(out[0]!.options.map((option) => option.value)).toEqual(["p2", "p9"]);
    expect(out[1]!.options.slice(0, 3).map((option) => option.value)).toEqual(["p2", "p4", "p9"]);
    expect(out[1]!.options.at(-1)!.value).toBe("p10");
  });

  it("a search hides the groups and keeps what matches; no group means no All heading", () => {
    const out = sections(options, [{ label: "With members", values: ["p2"] }], "pool 1");
    expect(out).toHaveLength(1);
    expect(out[0]!.label).toBeNull();
    expect(out[0]!.options.map((option) => option.value)).toEqual(["p1", "p10"]);
  });

  it("a pinned option comes first; a short list shows no groups, all of it in view", () => {
    const short = [{ value: "all", label: "All", pinned: true }, ...options.slice(0, 3)];
    const out = sections(short, [{ label: "With members", values: ["p2"] }], "", "All pools");
    expect(out.map((section) => section.label)).toEqual([null, null]);
    expect(out[0]!.options.map((option) => option.value)).toEqual(["all"]);
    expect(out[1]!.options.map((option) => option.value)).not.toContain("all");
  });

  it("draws at most 60 lines and says how many more typing finds", () => {
    const many = Array.from({ length: 75 }, (_, n) => ({ value: `v${n}`, label: `Item ${n}` }));
    const out = sections(many, [], "");
    expect(out[0]!.options).toHaveLength(60);
    expect(out[0]!.more).toBe(15);
  });

  it("past 8 options the menu opens on a search field; typing filters, Enter picks the first match", () => {
    const onChange = vi.fn();
    render(<Picker ariaLabel="Pool" value={null} options={options} groups={[{ label: "With members", values: ["p2"] }]} searchLabel="Search pools" onChange={onChange} testId="pick" />);
    fireEvent.click(screen.getByTestId("pick"));
    const search = screen.getByTestId("pick-search");
    expect(document.activeElement).toBe(search);
    expect(screen.getByText("With members")).toBeTruthy();
    fireEvent.change(search, { target: { value: "pool 9" } });
    expect(screen.queryByText("With members")).toBeNull();
    fireEvent.keyDown(search, { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith("p9");
  });

  it("a short list has no search field", () => {
    render(<Picker ariaLabel="Pool" value={null} options={options.slice(0, 4)} onChange={() => {}} testId="pick" />);
    fireEvent.click(screen.getByTestId("pick"));
    expect(screen.queryByTestId("pick-search")).toBeNull();
  });
});

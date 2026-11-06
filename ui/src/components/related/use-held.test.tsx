/**
 * E31 (AJ-E3103) vitest — a related list read again while it's shown
 * keeps its rows still: what arrives above waits behind "N new", and
 * "N new" shows the list in order.
 */
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useHeld } from "./useHeld";
import type { PagedList } from "../../lib/paged";

type R = { key: string; band: "working" | "recent" };
const list = (items: R[], over: Partial<PagedList<R>> = {}): PagedList<R> => ({ items, total: items.length, loading: false, stale: false, loadingMore: false, hasMore: false, error: null, loadMore: () => {}, refresh: () => {}, ...over });
const keyOf = (row: R) => row.key;
const bandOf = (row: R) => row.band;

describe("AJ-E3103 useHeld", () => {
  it("shows the remembered rows, then the first fresh read as it is, then holds", () => {
    const remembered = list([{ key: "e1", band: "recent" }], { stale: true });
    const { result, rerender } = renderHook(({ paged }) => useHeld(paged, keyOf, bandOf, "k"), { initialProps: { paged: remembered } });
    expect(result.current.list.items.map(keyOf)).toEqual(["e1"]);
    // The first fresh read: shown as it is (nobody's reading yet).
    rerender({ paged: list([{ key: "w1", band: "working" }, { key: "e1", band: "recent" }]) });
    expect(result.current.list.items.map(keyOf)).toEqual(["w1", "e1"]);
    expect(result.current.waiting).toBeNull();
    // Read again with a new one on top: it waits.
    rerender({ paged: list([{ key: "w2", band: "working" }, { key: "w1", band: "working" }, { key: "e1", band: "recent" }]) });
    expect(result.current.list.items.map(keyOf)).toEqual(["w1", "e1"]);
    expect(result.current.waiting).toBe("1 new");
    expect(result.current.list.total).toBe(2);
    act(() => result.current.showAll());
    expect(result.current.list.items.map(keyOf)).toEqual(["w2", "w1", "e1"]);
    expect(result.current.waiting).toBeNull();
  });

  it("a new scope (a filter, a search) starts again", () => {
    const { result, rerender } = renderHook(({ paged, scope }) => useHeld(paged, keyOf, bandOf, scope), { initialProps: { paged: list([{ key: "e1", band: "recent" }]), scope: "a" } });
    rerender({ paged: list([{ key: "w9", band: "working" }, { key: "e1", band: "recent" }]), scope: "a" });
    expect(result.current.waiting).toBe("1 new");
    rerender({ paged: list([{ key: "w9", band: "working" }]), scope: "b" });
    expect(result.current.list.items.map(keyOf)).toEqual(["w9"]);
    expect(result.current.waiting).toBeNull();
  });
});

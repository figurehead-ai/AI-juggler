/**
 * Walk A3: Back and Forward through the pages the window opened, as the
 * shell keeps them (the pure steps: lib/nav-history.test.ts).
 */
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { useNavigation } from "./use-navigation";

// The hook reads the cleanup count and the paired Macs; unmounted here, a
// late answer can't set state after the file's jsdom is gone (CI's
// "window is not defined" after every file passed, run 42728019512).
afterEach(cleanup);

it("goes back to the session from its App page, then to Home, and forward again", () => {
  const { result } = renderHook(() => useNavigation());
  expect(result.current.canBack).toBe(false);
  act(() => result.current.selectSession("s-1"));
  act(() => result.current.openPage({ kind: "app", id: "cli:work" }));
  expect(result.current.selection).toEqual({ kind: "app", id: "cli:work" });
  expect(result.current.canBack).toBe(true);
  act(() => result.current.back());
  expect(result.current.selection.kind).toBe("session");
  expect(result.current.canForward).toBe(true);
  act(() => result.current.back());
  expect(result.current.selection).toEqual({ kind: "none" });
  expect(result.current.canBack).toBe(false);
  act(() => result.current.forward());
  act(() => result.current.forward());
  expect(result.current.selection).toEqual({ kind: "app", id: "cli:work" });
  // A screen from the top bar joins the history too.
  act(() => result.current.setGlobal("accounts"));
  act(() => result.current.back());
  expect(result.current.global).toBeNull();
  expect(result.current.selection).toEqual({ kind: "app", id: "cli:work" });
  expect(result.current.canForward).toBe(true);
});

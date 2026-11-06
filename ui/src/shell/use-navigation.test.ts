/**
 * Walk A3: Back and Forward through the pages the window opened, as the
 * shell keeps them (the pure steps: lib/nav-history.test.ts).
 */
import { act, renderHook } from "@testing-library/react";
import { expect, it } from "vitest";
import { useNavigation } from "./use-navigation";

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

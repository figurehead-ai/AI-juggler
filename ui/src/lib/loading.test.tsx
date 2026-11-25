/**
 * AJ-E1552 — nothing loads forever: the loading states, a read's
 * deadline (in useResource and in a shared read), and the one component
 * that draws a skeleton only while a read loads.
 */
import { act, cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LoadingState } from "../components/LoadingState";
import { DEADLINE_MS, loadStateOf, missedDeadlines, SLOW_MS, TookTooLong, withDeadline } from "./loading";
import { sharedRead, useSharedReadState } from "./shared-read";
import { useResource } from "./use-resource";

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const never = <T,>() => new Promise<T>(() => {});

describe("the loading states", () => {
  it("a skeleton's state only before any answer; empty and error are their own", () => {
    expect(loadStateOf({ hasData: false, loading: true, error: undefined, empty: false })).toBe("loading");
    expect(loadStateOf({ hasData: true, loading: false, error: undefined, empty: false })).toBe("ready");
    expect(loadStateOf({ hasData: true, loading: false, error: undefined, empty: true })).toBe("empty");
    expect(loadStateOf({ hasData: false, loading: false, error: new Error("x"), empty: false })).toBe("error");
    expect(loadStateOf({ hasData: true, loading: true, error: undefined, empty: false })).toBe("stale");
    expect(loadStateOf({ hasData: true, loading: false, error: new Error("x"), empty: false })).toBe("stale");
  });

  it("a read past its deadline fails with TookTooLong", async () => {
    const late = withDeadline("the runways", never<number>(), 1_000);
    const caught = late.catch((cause: unknown) => cause);
    await vi.advanceTimersByTimeAsync(1_000);
    const cause = await caught;
    expect(cause).toBeInstanceOf(TookTooLong);
    expect(cause instanceof Error ? cause.message : cause).toBe("the runways took longer than 1 s");
  });
});

describe("useResource", () => {
  it("loading, slow after 3 s, then an error at its deadline, logged; Retry loads again", async () => {
    const read = vi.fn(() => never<string[]>());
    const view = renderHook(() => useResource("switch-space:a", read));
    expect(view.result.current.state).toBe("loading");
    expect(view.result.current.slow).toBe(false);
    await act(() => vi.advanceTimersByTimeAsync(SLOW_MS));
    expect(view.result.current.slow).toBe(true);
    await act(() => vi.advanceTimersByTimeAsync(DEADLINE_MS - SLOW_MS));
    expect(view.result.current.state).toBe("error");
    expect(view.result.current.error).toBeInstanceOf(TookTooLong);
    expect(missedDeadlines()[0]?.what).toBe("switch-space:a");
    act(() => view.result.current.reload());
    expect(read).toHaveBeenCalledTimes(2);
  });

  it("an empty answer is empty, not loading", async () => {
    const view = renderHook(() => useResource("rows", async () => [] as string[]));
    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(view.result.current.state).toBe("empty");
  });
});

describe("a shared read", () => {
  it("one that never answers fails at its deadline, and the next load reads again", async () => {
    const read = vi.fn(() => never<string[]>());
    const source = sharedRead(read, "the places");
    const view = renderHook(() => useSharedReadState(source));
    expect(view.result.current.state).toBe("loading");
    await act(() => vi.advanceTimersByTimeAsync(DEADLINE_MS));
    expect(view.result.current.state).toBe("error");
    expect(missedDeadlines()[0]?.what).toBe("the places");
    await act(async () => void source.load());
    expect(read).toHaveBeenCalledTimes(2);
  });
});

describe("LoadingState", () => {
  const draw = (state: "loading" | "ready" | "empty" | "error" | "stale", slow = false) =>
    render(
      <LoadingState state={state} slow={slow} what="the apps" skeleton={<span data-testid="skeleton" />} empty={<p>None yet</p>} onRetry={() => undefined}>
        <p>Rows</p>
      </LoadingState>,
    );

  it("draws its skeleton only while loading", () => {
    for (const state of ["ready", "empty", "error", "stale"] as const) {
      draw(state);
      expect(screen.queryByTestId("skeleton"), state).toBeNull();
      cleanup();
    }
    draw("loading", true);
    expect(screen.getByTestId("skeleton")).toBeTruthy();
    expect(screen.getByTestId("loading-slow").textContent).toBe("Still loading…");
  });

  it("an error in plain words with Retry; empty its own words", () => {
    draw("error");
    expect(screen.getByRole("alert").textContent).toBe("Couldn't load the apps.Retry");
    cleanup();
    draw("empty");
    expect(screen.getByText("None yet")).toBeTruthy();
  });
});

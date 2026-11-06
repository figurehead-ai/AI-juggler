import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useResource } from "./use-resource";

/** A read the test answers by hand. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

describe("AJ-E2018 useResource", () => {
  it("reads on mount, and nothing for a null key", async () => {
    const read = vi.fn(async () => 7);
    const { result } = renderHook(() => useResource("a", read));
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.data).toBe(7));
    expect(result.current.loading).toBe(false);
    const none = renderHook(() => useResource(null, read));
    expect(none.result.current.loading).toBe(false);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("keeps the last answer while a new key reads, and drops a stale answer", async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const reads: Record<string, Promise<string>> = { a: first.promise, b: second.promise };
    const { result, rerender } = renderHook(({ key }) => useResource(key, () => reads[key]!), { initialProps: { key: "a" } });
    rerender({ key: "b" });
    await act(async () => second.resolve("B"));
    expect(result.current.data).toBe("B");
    await act(async () => first.resolve("A")); // the old key's answer, too late
    expect(result.current.data).toBe("B");
    expect(result.current.loading).toBe(false);
  });

  it("a failure keeps the last answer and says why; reload reads again", async () => {
    let answer: Promise<number> = Promise.resolve(1);
    const { result } = renderHook(() => useResource("k", () => answer));
    await waitFor(() => expect(result.current.data).toBe(1));
    answer = Promise.reject(new Error("offline"));
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.error).toBeInstanceOf(Error));
    expect(result.current.data).toBe(1);
    answer = Promise.resolve(2);
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.data).toBe(2));
    expect(result.current.error).toBeUndefined();
  });

  it("polls while mounted", async () => {
    vi.useFakeTimers();
    try {
      const read = vi.fn(async () => 1);
      const { unmount } = renderHook(() => useResource("p", read, { pollMs: 1000 }));
      await act(async () => vi.advanceTimersByTimeAsync(3000));
      expect(read).toHaveBeenCalledTimes(4);
      unmount();
      await act(async () => vi.advanceTimersByTimeAsync(3000));
      expect(read).toHaveBeenCalledTimes(4);
    } finally {
      vi.useRealTimers();
    }
  });
});

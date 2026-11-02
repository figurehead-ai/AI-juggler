import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { forgetSharedReads, sharedRead, sharedReads, useSharedRead, type SharedRead } from "./shared-read";

/**
 * 0.3.4, the page-switching report — a read several screens show keeps
 * its last answer: a screen opened again paints it in its first frame and
 * reads again behind it; reads at once share one call; the screens that
 * keep it current share one timer; a failed read keeps the last answer.
 */
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function Shows({ source, pollMs, maxAgeMs, wanted }: { source: SharedRead<string[]>; pollMs?: number; maxAgeMs?: number; wanted?: boolean }) {
  const rows = useSharedRead(source, { pollMs, maxAgeMs, wanted });
  return <p data-testid="rows">{rows === undefined ? "loading" : rows.join(",")}</p>;
}

describe("sharedRead", () => {
  it("paints the last answer in a screen's first frame, then reads again behind it", async () => {
    let answer = ["a"];
    const read = vi.fn(async () => answer);
    const source = sharedRead(read);
    const first = render(<Shows source={source} />);
    expect(first.getByTestId("rows").textContent).toBe("loading");
    await waitFor(() => expect(first.getByTestId("rows").textContent).toBe("a"));
    first.unmount();

    answer = ["a", "b"];
    const again = render(<Shows source={source} />);
    // The last answer at once: no loading frame on a return.
    expect(again.getByTestId("rows").textContent).toBe("a");
    await waitFor(() => expect(again.getByTestId("rows").textContent).toBe("a,b"));
    expect(read).toHaveBeenCalledTimes(2);
  });

  it("shares one read between the screens that open at once", async () => {
    const read = vi.fn(async () => ["x"]);
    const source = sharedRead(read);
    const view = render(
      <>
        <Shows source={source} />
        <Shows source={source} />
        <Shows source={source} />
      </>,
    );
    await waitFor(() => expect(view.getAllByTestId("rows").map((row) => row.textContent)).toEqual(["x", "x", "x"]));
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("doesn't read again while the answer is younger than maxAgeMs", async () => {
    const read = vi.fn(async () => ["x"]);
    const source = sharedRead(read);
    const first = render(<Shows source={source} />);
    await waitFor(() => expect(first.getByTestId("rows").textContent).toBe("x"));
    first.unmount();
    render(<Shows source={source} maxAgeMs={30_000} />);
    await act(async () => undefined);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("keeps one timer for every screen that polls, at the shortest interval, and stops with the last", async () => {
    vi.useFakeTimers();
    const read = vi.fn(async () => ["x"]);
    const source = sharedRead(read);
    const view = render(
      <>
        <Shows source={source} pollMs={30_000} />
        <Shows source={source} pollMs={60_000} />
      </>,
    );
    await act(async () => undefined);
    expect(read).toHaveBeenCalledTimes(1); // the opening read, shared
    await act(async () => void (await vi.advanceTimersByTimeAsync(30_000)));
    expect(read).toHaveBeenCalledTimes(2); // one tick, not one per screen
    await act(async () => void (await vi.advanceTimersByTimeAsync(30_000)));
    expect(read).toHaveBeenCalledTimes(3);
    view.unmount();
    await act(async () => void (await vi.advanceTimersByTimeAsync(120_000)));
    expect(read).toHaveBeenCalledTimes(3);
  });

  it("reads nothing for a view that doesn't show it", async () => {
    const read = vi.fn(async () => ["x"]);
    render(<Shows source={sharedRead(read)} wanted={false} />);
    await act(async () => undefined);
    expect(read).not.toHaveBeenCalled();
  });

  it("keeps the last answer when a read fails", async () => {
    let fail = false;
    const source = sharedRead(async () => {
      if (fail) throw new Error("backend gone");
      return ["kept"];
    });
    await source.load();
    fail = true;
    await expect(source.refresh()).resolves.toEqual(["kept"]);
    expect(source.peek()).toEqual(["kept"]);
  });

  it("lets a refresh after a change win over a read that began before it", async () => {
    const answers: ((rows: string[]) => void)[] = [];
    const source = sharedRead(() => new Promise<string[]>((resolve) => answers.push(resolve)));
    const before = source.load();
    const after = source.refresh();
    answers[1]!(["new"]);
    answers[0]!(["old"]);
    await Promise.all([before, after]);
    expect(source.peek()).toEqual(["new"]);
  });

  it("is forgotten by forgetSharedReads", async () => {
    const source = sharedRead(async () => ["x"]);
    await source.load();
    forgetSharedReads();
    expect(source.peek()).toBeUndefined();
  });
});

describe("sharedReads", () => {
  it("keeps one answer per key, dropping the least recently asked beyond its bound", async () => {
    const read = vi.fn(async (key: string) => `v-${key}`);
    const of = sharedReads(read, 2);
    await of("a").load();
    await of("b").load();
    await of("c").load(); // "a" goes
    expect(of("b").peek()).toBe("v-b");
    expect(of("c").peek()).toBe("v-c");
    expect(of("a").peek()).toBeUndefined();
  });

  it("never drops an answer a screen shows", async () => {
    const of = sharedReads(async (key: string) => `v-${key}`, 1);
    function One({ id }: { id: string }) {
      return <p data-testid={id}>{useSharedRead(of(id)) ?? "…"}</p>;
    }
    const view = render(<One id="shown" />);
    await waitFor(() => expect(view.getByTestId("shown").textContent).toBe("v-shown"));
    await of("other").load();
    expect(of("shown").peek()).toBe("v-shown");
  });
});

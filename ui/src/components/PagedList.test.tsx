/**
 * AJ-E1552 — a paged list draws its skeleton only while its first page is
 * read ("Still loading…" after 3 s), its own empty state when it has no
 * rows, and an error in plain words with Retry when the read failed.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PagedList as Paged } from "../lib/paged";
import { SLOW_MS } from "../lib/loading";
import { PagedList } from "./PagedList";

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function list(over: Partial<Paged<string>>): Paged<string> {
  return {
    items: [],
    total: null,
    loading: false,
    stale: false,
    loadingMore: false,
    hasMore: false,
    error: null,
    loadMore: () => {},
    refresh: () => {},
    ...over,
  };
}

function draw(paged: Paged<string>) {
  return render(<PagedList list={paged} keyOf={(row) => row} renderRow={(row) => <span>{row}</span>} label="The sessions" empty={<p>No sessions yet.</p>} testId="sessions" />);
}

describe("PagedList's states", () => {
  it("loading: the skeleton, then Still loading after 3 s", async () => {
    draw(list({ loading: true }));
    expect(screen.getByTestId("sessions-skeleton")).toBeTruthy();
    expect(screen.queryByTestId("loading-slow")).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SLOW_MS);
    });
    expect(screen.getByTestId("loading-slow").textContent).toBe("Still loading…");
  });

  it("empty: the list's own empty state, never a skeleton", () => {
    draw(list({}));
    expect(screen.getByTestId("sessions-empty").textContent).toBe("No sessions yet.");
    expect(screen.queryByTestId("sessions-skeleton")).toBeNull();
  });

  it("error: says it couldn't load, and Retry reads again", () => {
    const refresh = vi.fn();
    draw(list({ error: new Error("gone"), refresh }));
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load the sessions.");
    expect(screen.queryByTestId("sessions-skeleton")).toBeNull();
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it("ready: its rows", () => {
    draw(list({ items: ["a", "b"] }));
    expect(screen.getByTestId("sessions")).toBeTruthy();
    expect(screen.queryByTestId("loading-state")).toBeNull();
  });
});

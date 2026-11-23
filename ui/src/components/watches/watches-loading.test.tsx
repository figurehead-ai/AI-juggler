/**
 * AJ-E1552 — the Watches skeletons draw only while their read loads:
 * the watch list, the Ended list and the form's conditions each show a
 * skeleton while loading ("Still loading…" after 3 s), their own empty
 * words, and an error with Retry that reads again.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { forgetPaged } from "../../lib/paged";
import { SLOW_MS } from "../../lib/loading";
import * as view from "../../lib/ipc/watch-view";
import { WatchesPage } from "./WatchesPage";
import { EndedSection } from "./WatchesLists";
import { WatchForm } from "./WatchForm";
import { stateFrom } from "./watch-form-state";
import type { TargetSources } from "./watches-view";

const SOURCES: TargetSources = { sessions: [], apps: [], projects: [], pools: [], accounts: [], now: 0 };
const never = <T,>(): Promise<T> => new Promise<T>(() => {});
const EMPTY = { items: [], next_cursor: null, total: 0 };

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
  vi.spyOn(view, "watchesSummary").mockReturnValue(never());
});
afterEach(() => {
  cleanup();
  forgetPaged();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const page = () => render(<WatchesPage sources={SOURCES} onGoTo={() => undefined} onAskBot={null} onQuietHours={() => undefined} />);
const ended = () => render(<EndedSection count={2} open onToggle={() => undefined} book={new Map()} now={0} appOf={() => null} actions={{ forward: () => undefined, remove: () => undefined }} />);
const form = () =>
  render(
    <WatchForm
      mode="new"
      initial={stateFrom({ targets: [{ kind: "session", id: "s-1" }], matching: "each", conditions: [], lifetime: { kind: "unsaid" }, evenInQuietHours: false }, 0)}
      editing={null}
      book={new Map()}
      quiet={null}
      count={0}
      limit={50}
      now={0}
      onCancel={() => undefined}
      onSave={async () => {}}
    />,
  );

describe("the watch list", () => {
  it("loading: its skeleton, Still loading after 3 s", async () => {
    vi.useFakeTimers();
    vi.spyOn(view, "watchesPage").mockReturnValue(never());
    page();
    expect(screen.getByTestId("watches-skeleton")).toBeTruthy();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SLOW_MS);
    });
    expect(screen.getByTestId("loading-slow").textContent).toBe("Still loading…");
  });

  it("empty: its starters, no skeleton", async () => {
    vi.spyOn(view, "watchesPage").mockResolvedValue(EMPTY);
    page();
    await screen.findByTestId("watches-empty");
    expect(screen.queryByTestId("watches-skeleton")).toBeNull();
  });

  it("error: plain words, Retry reads again", async () => {
    const read = vi.spyOn(view, "watchesPage").mockRejectedValue(new Error("down"));
    page();
    expect((await screen.findByRole("alert")).textContent).toContain("Couldn't read the watches.");
    const before = read.mock.calls.length;
    fireEvent.click(screen.getByTestId("loading-retry"));
    await waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
  });
});

describe("the Ended list", () => {
  it("loading: its skeleton", () => {
    vi.spyOn(view, "watchesPage").mockReturnValue(never());
    ended();
    expect(screen.getByTestId("watches-ended-skeleton")).toBeTruthy();
  });

  it("empty: says none ended", async () => {
    vi.spyOn(view, "watchesPage").mockResolvedValue(EMPTY);
    ended();
    expect((await screen.findByTestId("watches-ended-empty")).textContent).toBe("No ended watches in the last 30 days.");
  });

  it("error: Retry reads again", async () => {
    const read = vi.spyOn(view, "watchesPage").mockRejectedValue(new Error("down"));
    ended();
    expect((await screen.findByRole("alert")).textContent).toContain("Couldn't load the ended watches.");
    const before = read.mock.calls.length;
    fireEvent.click(screen.getByTestId("loading-retry"));
    await waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
  });
});

describe("the form's conditions", () => {
  beforeEach(() => {
    vi.spyOn(view, "watchDraftWords").mockReturnValue(never());
  });

  it("loading: their skeleton", () => {
    vi.spyOn(view, "watchCatalogue").mockReturnValue(never());
    form();
    expect(screen.getByTestId("watch-conditions-skeleton")).toBeTruthy();
  });

  it("empty: nothing to watch for, in words", async () => {
    vi.spyOn(view, "watchCatalogue").mockResolvedValue([]);
    form();
    await screen.findByTestId("watch-nothing-offered");
    expect(screen.queryByTestId("watch-conditions-skeleton")).toBeNull();
  });

  it("error: Retry reads again", async () => {
    const read = vi.spyOn(view, "watchCatalogue").mockRejectedValue(new Error("down"));
    form();
    expect((await screen.findByRole("alert")).textContent).toContain("Couldn't load the conditions.");
    fireEvent.click(screen.getByTestId("loading-retry"));
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
  });
});

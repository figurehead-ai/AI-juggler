/**
 * AJ-E1552 vitest — the switch's check (SwitchHeader's Checking) on
 * LoadingState: its skeleton only while the check reads, "Still loading…"
 * after 3 s, an error in plain words with Retry, its own empty words, and
 * no skeleton once answered.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Checking } from "./SwitchHeader";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton } from "../../test/loading-kit";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("the switch's check", () => {
  it("draws its skeleton while it reads, and says Still loading after 3 s", async () => {
    vi.useFakeTimers();
    render(<Checking target="Personal" />);
    expect(screen.getByTestId("switch-preflighting").textContent).toBe("Checking the switch to Personal…");
    await expectSlowSkeleton("switch-checking");
  });

  it("a failed check says so with Retry, and no skeleton", async () => {
    const retry = vi.fn();
    render(<Checking target="Personal" state="error" onRetry={retry} />);
    await expectErrorRetry("the check of the switch to Personal", retry, "switch-checking");
  });

  it("nothing to check has its own words, and no skeleton", async () => {
    render(<Checking target="Personal" state="empty" />);
    await expectEmpty("switch-check-empty", "Nothing to check for the switch to Personal.", "switch-checking");
  });

  it("no skeleton once the check answered", () => {
    render(<Checking target="Personal" state="ready" />);
    expect(screen.queryByTestId("switch-checking")).toBeNull();
  });
});

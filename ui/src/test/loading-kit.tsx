/**
 * AJ-E1552 — the checks every skeleton site's state test makes: its
 * skeleton while loading with "Still loading…" after 3 s, no skeleton
 * once answered, and an error in plain words whose Retry reads again.
 */
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { expect, vi } from "vitest";
import { SLOW_MS } from "../lib/loading";

/** A read that never answers. */
export const never = <T,>(): Promise<T> => new Promise<T>(() => {});

/** The skeleton `testId` is drawn, and says "Still loading…" after 3 s
 * (fake timers on). */
export async function expectSlowSkeleton(testId: string): Promise<void> {
  expect(screen.getByTestId(testId)).toBeTruthy();
  expect(screen.queryByTestId("loading-slow")).toBeNull();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(SLOW_MS);
  });
  expect(screen.getByTestId("loading-slow").textContent).toBe("Still loading…");
}

/** "Couldn't load <what>." (or `words`) with Retry, the skeleton gone;
 * Retry calls `read` again. */
export async function expectErrorRetry(what: string, read: { mock: { calls: unknown[] } }, skeleton?: string, words = `Couldn't load ${what}.`): Promise<void> {
  const alert = (await screen.findAllByRole("alert")).find((node) => node.textContent?.includes(words));
  expect(alert, `an error saying "${words}"`).toBeTruthy();
  if (skeleton) expect(screen.queryByTestId(skeleton)).toBeNull();
  const before = read.mock.calls.length;
  fireEvent.click(within(alert!).getByTestId("loading-retry"));
  await waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
}

/** The empty words `testId` (containing `words`), and no skeleton. */
export async function expectEmpty(testId: string, words: string | RegExp, skeleton?: string): Promise<void> {
  expect((await screen.findByTestId(testId)).textContent).toMatch(words);
  if (skeleton) expect(screen.queryByTestId(skeleton)).toBeNull();
}

/**
 * How a vendor's limit reading reads (QA, 2026-10-21, on 30 days of the
 * real Codex account's readings): a measured percentage says when it was
 * read; once its window has reset with no newer reading, the window says
 * so — never 0%, never the old number; credits say who reported them.
 */
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { creditsWords } from "../components/AccountRunway";
import type { CreditsDisplay } from "./bridge";
import { asOfWords, windowView } from "./quotaWindow";
import { quotaDisplay, windowDisplay } from "./wire";

const NOW = new Date(2026, 9, 21, 15, 30).getTime();
const MIN = 60_000;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});
afterEach(() => {
  vi.useRealTimers();
});

it("as of: minutes under an hour, a time under a day, then a date", () => {
  expect(asOfWords(NOW - 20_000, NOW)).toBe("as of just now");
  expect(asOfWords(NOW - 12 * MIN, NOW)).toBe("as of 12 min ago");
  const earlier = new Date(2026, 9, 21, 9, 5).getTime();
  expect(asOfWords(earlier, NOW)).toBe(`as of ${new Date(earlier).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}`);
  const days = new Date(2026, 9, 18, 9, 5).getTime();
  expect(asOfWords(days, NOW)).toBe(`as of ${new Date(days).toLocaleDateString([], { month: "short", day: "numeric" })}`);
});

it("a measured window says when it was read", () => {
  const view = windowView(windowDisplay("tracking", { percentage: 64, badge: "measured", measured_at_ms: NOW - 3 * MIN }));
  expect(view.left).toBe(36);
  expect(view.words).toBe("36% left · as of 3 min ago");
});

it("after the reset with no new reading: not measured, never 0% or the old number", () => {
  const view = windowView(windowDisplay("reset", { measured_at_ms: NOW - 3 * 86_400_000 }));
  expect(view.left).toBeNull();
  expect(view.tone).toBe("none");
  expect(view.words).toBe("Not measured since the reset");
  expect(view.resets).toBeNull();
});

it("credits say how much, who reported them and when", () => {
  const display = (credits: CreditsDisplay | null) => quotaDisplay("codex-1", { credits });
  expect(creditsWords(display({ has_credits: true, unlimited: false, balance: "16.25", as_of_ms: NOW - 5 * MIN, source: "Codex" }))).toBe(
    "Credits: 16.25 · reported by Codex, as of 5 min ago",
  );
  expect(creditsWords(display({ has_credits: true, unlimited: true, balance: null, as_of_ms: NOW - 5 * MIN, source: "Codex" }))).toBe(
    "Credits: unlimited · reported by Codex, as of 5 min ago",
  );
  expect(creditsWords(display(null))).toBeNull();
});

it("after a reset, a session's old reading doesn't bring '% left' back (Home, the cards)", async () => {
  const { runwayOf, runwayLines } = await import("../components/AccountRunway");
  const session = (resets: number) =>
    ({
      id: "s",
      status: "running",
      started_at_ms: NOW - 3_600_000,
      last_activity_ms: NOW - 60_000,
      quota: { used_percentage: 36, resets_at_ms: resets, confidence: "measured" },
    }) as unknown as import("./bridge").InstanceInfo;
  const reset = quotaDisplay("a", { weekly: windowDisplay("reset", { measured_at_ms: NOW - 2 * 86_400_000 }) });
  // A window reset with no new reading: the session's reading is the old window's.
  expect(runwayOf(reset, [session(NOW + 3_600_000)]).words).toBe("Not measured since the reset");
  expect(runwayLines(reset, [session(NOW + 3_600_000)]).some((line) => /% left/.test(line.words))).toBe(false);
  // A session reading whose own reset has passed says nothing either.
  const none = quotaDisplay("a");
  expect(runwayOf(none, [session(NOW - MIN)]).left).toBeNull();
  // A live session reading still helps when nothing else is known.
  expect(runwayOf(none, [session(NOW + 3_600_000)]).words).toBe("64% left, from its latest session");
});

it("New session says what Home says for the same account (AJ-E1535)", async () => {
  const { render } = await import("@testing-library/react");
  const { createElement } = await import("react");
  const { RunwayWords, runwayOf } = await import("../components/AccountRunway");
  const session = {
    id: "s",
    status: "running",
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    quota: { used_percentage: 42, resets_at_ms: NOW + 3_600_000, confidence: "measured" },
  } as unknown as import("./bridge").InstanceInfo;
  const none = quotaDisplay("a");
  const view = render(createElement(RunwayWords, { display: none, sessions: [session] }));
  expect(view.getByTestId("launch-runway").textContent).toBe(runwayOf(none, [session]).words);
  expect(view.getByTestId("launch-runway").textContent).toBe("58% left, from its latest session");
});

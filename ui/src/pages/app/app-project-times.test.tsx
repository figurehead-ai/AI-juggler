/**
 * The 0.4.6 rc2 walk: a Claude app profile's page listed "AIJuggler · 11
 * sessions · 20773 d ago" — 1970, a missing time read as 0. The project
 * was only ever worked in outside AI Juggler, so its own sessions' time is
 * empty; when it was last worked in is its history's. And a time that
 * isn't known says nothing, never days since 1970.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ProjectRows } from "./AppOverview";
import { withProjectCounts } from "../../lib/app-items";
import { ago, agoWords, knownMs, minutes, since } from "../../lib/time-words";
import { project } from "../../test/fixtures";

const NOW = new Date(2026, 10, 16, 17, 50).getTime();
const SIX_MIN_AGO = NOW - 6 * 60_000;

afterEach(cleanup);

describe("a project's last use on an App's page", () => {
  it("is when it was last worked in, outside AI Juggler too", () => {
    const outside = project({ id: "p-aij", name: "AIJuggler", last_activity_ms: null, history_last_used_ms: SIX_MIN_AGO, history_sessions: 27 });
    const rows = withProjectCounts([], new Map([["p-aij", 11]]), [outside]);
    expect(rows).toEqual([{ id: "p-aij", name: "AIJuggler", sessions: 11, lastMs: SIX_MIN_AGO }]);
  });

  it("says nothing when it isn't known", () => {
    render(<ProjectRows projects={[{ id: "p-aij", name: "AIJuggler", sessions: 11, lastMs: 0 }]} now={NOW} onOpenProject={() => {}} />);
    const row = screen.getByTestId("app-project");
    expect(row.textContent).toBe("AIJuggler11 sessions");
    expect(row.textContent).not.toMatch(/ago|\bd\b/);
  });

  it("says how long ago when it is", () => {
    render(<ProjectRows projects={[{ id: "p-aij", name: "AIJuggler", sessions: 11, lastMs: SIX_MIN_AGO }]} now={NOW} onOpenProject={() => {}} />);
    expect(screen.getByTestId("app-project").textContent).toBe("AIJuggler11 sessions · 6 min ago");
  });
});

describe("how long ago, for a time that isn't known", () => {
  it.each([0, -1, Number.NaN, Number.POSITIVE_INFINITY])("%s says nothing in every form", (ms) => {
    expect(knownMs(ms)).toBe(false);
    expect(ago(ms, NOW)).toBe("");
    expect(agoWords(ms, NOW)).toBe("");
    expect(minutes(ms, NOW)).toBe("");
    expect(since(ms, NOW)).toBe("");
  });

  it("a known one is still said", () => {
    expect(knownMs(SIX_MIN_AGO)).toBe(true);
    expect(agoWords(SIX_MIN_AGO, NOW)).toBe("6 min ago");
    expect(agoWords(NOW, NOW)).toBe("just now");
    expect(ago(NOW - 3 * 86_400_000, NOW)).toBe("3 d");
  });
});

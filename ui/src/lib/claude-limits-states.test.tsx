/**
 * The maintainer's third report: Claude accounts showed "Not measured yet"
 * for every window, and "Its sessions run outside AI Juggler" was a dead
 * end. A window now names where its reading came from and how old it is;
 * an old reading is history, never the state now; a limit hit says when
 * and when it resets; the unmeasured note says where readings come from.
 * D-38's "ask Claude Code" shows only once the backend offers it.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { WindowDisplay } from "./bridge";
import { agoWords, windowView } from "./quotaWindow";
import { runwayOf } from "../components/AccountRunway";
import { unmeasuredWords } from "../screens/accounts/AccountCard";
import { AccountProbe } from "../pages/AccountProbe";
import { setToolsForTests } from "./tools";
import { PREVIEW_TOOLS } from "../preview/tools";

const NOW = Date.now();
const MIN = 60_000;
const HOUR = 60 * MIN;

function win(over: Partial<WindowDisplay>): WindowDisplay {
  return {
    state: "stale",
    percentage: 40,
    badge: "measured",
    stale_note_ms: null,
    resets_at_ms: null,
    reset_basis: null,
    measured_at_ms: NOW - 55 * MIN,
    burned_tokens: 1000,
    burned_credits: null,
    headroom_tokens: null,
    ...over,
  };
}

const w = () => window as unknown as Record<string, unknown>;
afterEach(() => {
  cleanup();
  delete w().__AIJ_PROBE__;
});

describe("Claude limits from its own records", () => {
  it("a measured window says its source and age", () => {
    const view = windowView(win({ source: "desktop_app", source_words: "the Claude app" }));
    expect(view.left).toBe(60);
    expect(view.words).toBe("60% left · from the Claude app, 55 min ago");
  });

  it("a reading without a source keeps the old as-of words", () => {
    expect(windowView(win({ measured_at_ms: NOW - 12 * MIN })).words).toBe("60% left · as of 12 min ago");
  });

  it("an old reading is history, never the state now", () => {
    const view = windowView(
      win({ state: "reset", percentage: null, measured_at_ms: NOW - 13 * HOUR, last_used_pct: 40, source_words: "the Claude app" }),
    );
    expect(view.left).toBeNull();
    expect(view.tone).toBe("none");
    expect(view.words).toBe("Last reading 13 h ago (the Claude app): 40% used; that window has reset since");
    // Home's line says the same.
    const line = runwayOf({
      account: "a",
      five_hour: win({ state: "reset", percentage: null, measured_at_ms: NOW - 13 * HOUR, last_used_pct: 40 }),
      five_hour_omitted_reason: null,
      weekly: null,
      weekly_omitted_reason: "Claude Code reports no weekly limit for this account",
      renewal: { state: "none", label: "", entered_by_user: false },
      forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
      unavailable_reason: null,
      credits: null,
    });
    expect(line.left).toBeNull();
    expect(line.words).toMatch(/^Last reading 13 h ago: 40% used/);
  });

  it("a limit hit says when, and when it resets", () => {
    const resets = NOW + 3 * HOUR;
    const view = windowView(
      win({ state: "exhausted", percentage: 100, source: "api_retry", measured_at_ms: NOW - HOUR, resets_at_ms: resets, reset_basis: "measured" }),
    );
    expect(view.left).toBe(0);
    expect(view.tone).toBe("out");
    expect(view.words).toMatch(/^Hit its limit at .+ · resets /);
  });

  it("the unmeasured note says where readings come from, not that they can't be had", () => {
    setToolsForTests(PREVIEW_TOOLS);
    const words = unmeasuredWords("claude-code");
    expect(words).not.toMatch(/outside AI Juggler/);
    expect(words).toMatch(/app's meter/);
  });

  it("ages read in plain words", () => {
    expect(agoWords(NOW - 30_000, NOW)).toBe("just now");
    expect(agoWords(NOW - 3 * 86_400_000, NOW)).toBe("3 days ago");
  });
});

describe("D-38: ask Claude Code (off until ruled)", () => {
  it("isn't shown while the backend doesn't offer it", async () => {
    const view = render(<AccountProbe accountId="a1" />);
    await waitFor(() => expect(view.queryByTestId("account-probe")).toBeNull());
  });

  it("once offered, the sheet says what runs and what it costs before anything runs", async () => {
    w().__AIJ_PROBE__ = { a1: { offered: true, state: "off", last_run_ms: null, last_outcome: null } };
    const view = render(<AccountProbe accountId="a1" />);
    const open = await view.findByTestId("account-probe-open");
    await act(async () => {
      fireEvent.click(open);
    });
    const sheet = view.getByTestId("account-probe-sheet").textContent!;
    expect(sheet).toContain("costs no quota");
    expect(sheet).toContain("AI Juggler never sees it");
    expect(sheet).toContain("AI Juggler sends nothing");
    await act(async () => {
      fireEvent.click(view.getByTestId("account-probe-on"));
    });
    await waitFor(() => expect(view.getByTestId("account-probe-words").textContent).toContain("On:"));
    expect(view.getByTestId("account-probe-run")).toBeTruthy();
    await act(async () => {
      fireEvent.click(view.getByTestId("account-probe-off"));
    });
    await waitFor(() => expect(view.getByTestId("account-probe-open")).toBeTruthy());
  });
});

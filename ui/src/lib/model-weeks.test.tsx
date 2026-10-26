/**
 * The maintainer's report on 0.1.6: "Claude runway 5 hour seems to be
 * showing, but not the 7 day one". A team seat's only week is on one model
 * ("Fable only"); it read as "Claude Code reports no weekly limit". A
 * model's own week is a real limit: every surface that shows the windows
 * shows it, beside the all-models week when there is one. It limits that
 * model alone, so it never sets the account's room; a low one is said.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { QuotaDisplayData, WindowDisplay } from "./bridge";
import { limitWindows } from "./quotaWindow";
import { runwayLines, runwayOf } from "../components/AccountRunway";
import { RunwayPanel } from "../components/usage/Runway";
import { BurnDowns } from "../components/BurnDown";
import { Runway as SessionRunway } from "../pages/session/Runway";

const NOW = Date.now();
const MIN = 60_000;
const DAY = 86_400_000;

function win(over: Partial<WindowDisplay>): WindowDisplay {
  return {
    state: "tracking",
    percentage: 30,
    badge: "measured",
    stale_note_ms: null,
    resets_at_ms: NOW + 2 * 3_600_000,
    reset_basis: "measured",
    measured_at_ms: NOW - 5 * MIN,
    burned_tokens: 0,
    burned_credits: null,
    headroom_tokens: null,
    source: "usage_cache",
    source_words: "Claude Code's last usage check",
    ...over,
  };
}

function display(over: Partial<QuotaDisplayData>): QuotaDisplayData {
  return {
    account: "team-a",
    five_hour: win({}),
    five_hour_omitted_reason: null,
    weekly: null,
    weekly_omitted_reason: "Claude Code reports a weekly limit only for Fable",
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
    unavailable_reason: null,
    credits: null,
    model_weeks: [{ model: "Fable", window: win({ percentage: 0, resets_at_ms: NOW + 3 * DAY }) }],
    ...over,
  };
}

afterEach(cleanup);

describe("a week on one model only", () => {
  it("stands in the list of limits, and alone when there is no all-models week", () => {
    expect(limitWindows(display({})).map((entry) => entry.label)).toEqual(["5-hour window", "This week · Fable only"]);
    // A personal plan: the whole week, and the model's week beside it.
    const personal = display({ weekly: win({ percentage: 40, resets_at_ms: NOW + 3 * DAY }), weekly_omitted_reason: null });
    expect(limitWindows(personal).map((entry) => entry.label)).toEqual(["5-hour window", "This week", "This week · Fable only"]);
    // No model's week: the week's line says why it's absent, as before.
    const none = display({ model_weeks: [], weekly_omitted_reason: "Claude Code reports no weekly limit for this account" });
    expect(limitWindows(none).map((entry) => entry.omitted)).toEqual([null, "Claude Code reports no weekly limit for this account"]);
    // An older backend that doesn't send them.
    const older = { ...none, model_weeks: undefined };
    expect(limitWindows(older)).toHaveLength(2);
  });

  it("reads as a real limit, never as 'no weekly limit'", () => {
    const lines = runwayLines(display({}));
    expect(lines.map((line) => line.label)).toEqual(["5-hour window", "This week · Fable only"]);
    expect(lines[1]!.left).toBe(100);
    expect(lines[1]!.words).toMatch(/^100% left · resets .+ · from Claude Code's last usage check, 5 min ago$/);
    expect(lines.some((line) => /no weekly limit/.test(line.words))).toBe(false);
  });

  it("never sets the account's room, and a low one is said", () => {
    // 70% left in the 5 hours: the account's room, whatever Fable's week says.
    expect(runwayOf(display({})).left).toBe(70);
    expect(runwayOf(display({})).words).toBe("70% left in the 5-hour window");
    const low = display({ model_weeks: [{ model: "Fable", window: win({ percentage: 92 }) }] });
    expect(runwayOf(low).left).toBe(70);
    expect(runwayOf(low).words).toBe("70% left in the 5-hour window · Fable: 8% left this week");
    // Not measured, or reset since: nothing is said for it.
    const old = display({ model_weeks: [{ model: "Fable", window: win({ state: "reset", percentage: null, last_used_pct: 95 }) }] });
    expect(runwayOf(old).words).toBe("70% left in the 5-hour window");
  });

  it("has its own gauge on the account and usage cards", () => {
    const { getAllByTestId } = render(
      <RunwayPanel display={display({})} limitsProfile={null} usage={null} usageTotal={null} color="#888" name="Team A" now={NOW} />,
    );
    const gauges = getAllByTestId("window-gauge").map((gauge) => gauge.textContent ?? "");
    expect(gauges).toHaveLength(2);
    expect(gauges[1]).toContain("This week · Fable only");
    expect(gauges[1]).toContain("100");
    expect(gauges.join(" ")).not.toContain("no weekly limit");
  });

  it("has its own line on a session's Runway card", () => {
    const { getByTestId } = render(<SessionRunway display={display({})} title={null} />);
    const card = getByTestId("account-runway").textContent ?? "";
    expect(card).toContain("This week · Fable only");
    expect(card).toContain("100% left");
  });

  it("has its own burn-down when it has a runway", () => {
    const runway = { series: [[NOW - 60 * MIN, 10], [NOW - 5 * MIN, 12]] as [number, number][], starts_ms: NOW - 4 * DAY, resets_at_ms: NOW + 3 * DAY, used_pct: 12, pct_per_hour: null, exhausts_at_ms: null, session_pct: null, sessions_left: null, sessions_counted: 0, badge: "measured" };
    const shown = display({ five_hour: win({ runway: null }), model_weeks: [{ model: "Fable", window: win({ percentage: 12, runway }) }] });
    const { getByTestId } = render(<BurnDowns display={shown} />);
    expect(getByTestId("burn-downs").textContent).toContain("This week · Fable only");
  });
});

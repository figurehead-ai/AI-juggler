/**
 * AJ-E1512 vitest — sessions left in plain words with the forecast's
 * confidence word beside them; the burn-down's projection only when the
 * pace is known, ending at the exhaustion before the reset (or at the
 * reset); renders a full window in under 50 ms.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { BurnDownChart, BurnDowns, paceWords, sessionsLeftWords } from "./BurnDown";
import { quotaDisplay, windowDisplay } from "../lib/wire";
import type { RunwayDisplay } from "../lib/ipc/quota";

afterEach(cleanup);

const NOW = 1_792_094_400_000;
const H = 3_600_000;

function runway(over: Partial<RunwayDisplay> = {}): RunwayDisplay {
  return {
    series: Array.from({ length: 13 }, (_, i) => [NOW - 2 * H + i * 600_000, (i * 40) / 12] as [number, number]),
    starts_ms: NOW - 2 * H,
    resets_at_ms: NOW + 3 * H,
    used_pct: 40,
    pct_per_hour: 20,
    exhausts_at_ms: NOW + 3 * H - 30 * 60_000,
    session_pct: 10,
    sessions_left: 6,
    sessions_counted: 5,
    badge: "computed",
    ...over,
  };
}

describe("AJ-E1512 — sessions left and the burn-down", () => {
  it("says sessions left from the usual session, or why not yet", () => {
    expect(sessionsLeftWords(runway())).toBe("About 6 usual sessions left (each about 10%)");
    expect(sessionsLeftWords(runway({ sessions_left: 1 }))).toBe("About 1 usual session left (each about 10%)");
    expect(sessionsLeftWords(runway({ sessions_left: 0 }))).toBe("Not enough left for a usual session");
    expect(sessionsLeftWords(runway({ sessions_left: 31 }))).toBe("20+ usual sessions left");
    expect(sessionsLeftWords(runway({ sessions_left: null, session_pct: null, sessions_counted: 2 }))).toBe(
      "Sessions left show after a few sessions on this account",
    );
    expect(sessionsLeftWords(runway({ sessions_left: null, session_pct: null }))).toBe(
      "Sessions left show once this window has moved a little",
    );
  });

  it("carries the confidence word beside the number", () => {
    const display = quotaDisplay("acc", { five_hour: windowDisplay("tracking", { percentage: 40, runway: runway({ badge: "estimated" }) }) });
    const { getByTestId } = render(<BurnDowns display={display} />);
    expect(getByTestId("sessions-left-tier").textContent).toBe("estimate");
    expect(getByTestId("burn-down-chart").getAttribute("aria-label")).toContain("5-hour window: 40% used. About 6 usual sessions left");
  });

  it("projects to the exhaustion before the reset, past it, or not at all", () => {
    const { container, rerender } = render(<BurnDownChart runway={runway()} label="x" />);
    expect(container.querySelector('[data-testid="burn-down-projection"]')).not.toBeNull();
    expect(paceWords(runway())).toContain("runs out about");
    rerender(<BurnDownChart runway={runway({ exhausts_at_ms: null })} label="x" />);
    expect(container.querySelector('[data-testid="burn-down-projection"]')).not.toBeNull();
    expect(paceWords(runway({ exhausts_at_ms: null }))).toContain("lasts past the reset");
    rerender(<BurnDownChart runway={runway({ pct_per_hour: null, exhausts_at_ms: null })} label="x" />);
    expect(container.querySelector('[data-testid="burn-down-projection"]')).toBeNull();
    expect(paceWords(runway({ pct_per_hour: null, exhausts_at_ms: null }))).toMatch(/^Resets /);
  });

  it("shows nothing for an account without a runway", () => {
    const { container } = render(<BurnDowns display={quotaDisplay("acc")} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders a full window in under 50 ms", () => {
    const full = runway({ series: Array.from({ length: 120 }, (_, i) => [NOW - 5 * H + i * 150_000, i / 1.2] as [number, number]) });
    const display = quotaDisplay("acc", {
      five_hour: windowDisplay("tracking", { percentage: 40, runway: full }),
      weekly: windowDisplay("tracking", { percentage: 20, runway: full }),
    });
    render(<BurnDowns display={display} />); // warm-up
    cleanup();
    const began = performance.now();
    render(<BurnDowns display={display} />);
    expect(performance.now() - began).toBeLessThan(50);
  });
});

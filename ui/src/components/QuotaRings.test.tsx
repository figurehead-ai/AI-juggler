/**
 * AJ-E610 evidence: the typed state matrix on real renders — every
 * quota number carries its badge adjacent (grep-gated), stale Measured
 * demotes with "last measured N ago", blocked shows the reset, omitted
 * windows render their reason, unavailable-with-reason for Gemini, the
 * two clocks never merge, and D-12 holds (no date ⇒ unknown copy,
 * zero inferred dates).
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ForecastPill, QuotaChip, RenewalLine, TwoClocks, WindowRing } from "./QuotaRings";
import type { QuotaDisplayData, WindowDisplay } from "../lib/bridge";

const window = (over: Partial<WindowDisplay> = {}): WindowDisplay => ({
  state: "tracking",
  percentage: 82,
  badge: "measured",
  burned_tokens: 12_345,
  resets_at_ms: 1_792_096_400_000,
  ...over,
});

const display = (over: Partial<QuotaDisplayData> = {}): QuotaDisplayData => ({
  account: "a1",
  five_hour: window(),
  weekly: window({ percentage: 41, badge: "measured", resets_at_ms: 1_792_120_000_000 }),
  renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
  forecast: { state: "wont-exhaust", badge: "measured" },
  ...over,
});

afterEach(cleanup);

describe("WindowRing typed states (Req.4)", () => {
  it("measured: % text + badge adjacent + reset countdown (never colour-only)", () => {
    const screen = render(<WindowRing window={window()} kind="5h" />);
    expect(screen.getByText("82%")).toBeTruthy();
    expect(screen.getByText(/5-hour window · Measured/)).toBeTruthy();
    expect(screen.container.textContent).toMatch(/resets \d{2}:\d{2}/);
    // role=img with the full text equivalent (Req.6)
    const img = screen.container.querySelector('[role="img"]');
    expect(img?.getAttribute("aria-label")).toContain("82%");
    expect(img?.getAttribute("aria-label")).toContain("Measured");
  });

  it("no-data: 'Start a session to measure', no number, never 0%", () => {
    const screen = render(<WindowRing window={window({ state: "unknown", percentage: undefined })} kind="5h" />);
    expect(screen.getByText("Start a session to measure")).toBeTruthy();
    expect(screen.container.textContent).not.toContain("%");
    expect(screen.container.textContent).not.toContain("0%");
  });

  it("learning: the exact copy, no number", () => {
    const screen = render(<WindowRing window={window({ state: "learning", percentage: undefined })} kind="5h" />);
    expect(screen.getByText("learning — no forecast yet")).toBeTruthy();
    expect(screen.container.textContent).not.toMatch(/\d+%/);
  });

  it("stale Measured: 'last measured N ago' and visual demotion (Q-1)", () => {
    const screen = render(
      <WindowRing
        window={window({ stale_note_ms: 22 * 60 * 1000 })}
        kind="5h"
      />,
    );
    expect(screen.getByText(/last measured 22m ago/)).toBeTruthy();
    expect(screen.container.querySelector("svg")?.getAttribute("class")).toContain("opacity-60");
  });

  it("blocked: exhausted ring + reset CTA copy", () => {
    const screen = render(
      <WindowRing window={window({ state: "exhausted", percentage: 100 })} kind="5h" />,
    );
    expect(screen.getByText(/blocked — resets/)).toBeTruthy();
  });

  it("omitted window: its reason renders, never an empty ring", () => {
    const screen = render(
      <WindowRing kind="5h" omittedReason="Pro plans have no five-hour limit — weekly only" />,
    );
    expect(screen.getByTestId("ring-5h-omitted").textContent).toContain("no five-hour limit");
    expect(screen.container.querySelector("svg")).toBeNull();
  });

  it("estimated badge shows Estimate adjacent to the number", () => {
    const screen = render(<WindowRing window={window({ badge: "estimated" })} kind="weekly" />);
    expect(screen.getByText(/weekly window · Estimate/)).toBeTruthy();
    expect(screen.getByText("82%")).toBeTruthy();
  });
});

describe("TwoClocks separation + unavailability (Req.2/4)", () => {
  it("the subscription clock is a calendar line, not a merged countdown", () => {
    const screen = render(<TwoClocks display={display()} />);
    expect(screen.getByTestId("renewal-line").textContent).toContain("Renewal: unknown");
    // The two clocks never merge: the renewal line carries no %, and
    // no ring element mentions renewal (AJ-E606 Req.3).
    expect(screen.getByTestId("renewal-line").textContent).not.toContain("%");
    const rings = screen.container.querySelectorAll('[data-testid^="ring-"]');
    [...rings].forEach((ring) => expect(ring.textContent).not.toContain("Renew"));
  });

  it("unavailable-with-reason (Gemini pre-verification)", () => {
    const screen = render(
      <TwoClocks display={display({ five_hour: undefined, weekly: undefined, unavailable_reason: "No verified quota source for this Provider yet" })} />,
    );
    expect(screen.getByTestId("quota-unavailable").textContent).toContain("No verified quota source");
  });
});

describe("RenewalLine states (Req.3, D-12)", () => {
  it("none: unknown copy + add-date affordance, entered_by_user false", () => {
    const screen = render(
      <RenewalLine renewal={{ state: "none", label: "Renewal: unknown — add date", entered_by_user: false }} onAddDate={() => {}} />,
    );
    expect(screen.getByText(/Renewal: unknown — add date/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "add date" })).toBeTruthy();
  });

  it("set / approaching / overdue carry distinct copy and tones", () => {
    const set = render(<RenewalLine renewal={{ state: "set", label: "Renews 2026-10-23", entered_by_user: true }} />);
    expect(set.getByTestId("renewal-line").dataset.state).toBe("set");
    set.unmount();
    const approaching = render(
      <RenewalLine renewal={{ state: "approaching", label: "Renews in 6 days — check your plan", entered_by_user: true }} />,
    );
    expect(approaching.getByTestId("renewal-line").className).toContain("text-amber-500");
    approaching.unmount();
    const overdue = render(
      <RenewalLine renewal={{ state: "overdue", label: "Renewal date passed", entered_by_user: true }} />,
    );
    expect(overdue.getByTestId("renewal-line").className).toContain("text-danger");
  });
});

describe("ForecastPill states (deep/10 §1)", () => {
  it("learning copy exact; Estimated exhausts shows the RANGE", () => {
    const learning = render(<ForecastPill forecast={{ state: "learning", badge: "estimated" }} />);
    expect(learning.getByText("learning — no forecast yet")).toBeTruthy();
    learning.unmount();
    const estimated = render(
      <ForecastPill forecast={{ state: "exhausts", tte_min: 55, range_min: 40, range_max: 70, badge: "estimated" }} />,
    );
    const text = estimated.getByTestId("forecast-pill").textContent ?? "";
    expect(text).toContain("≈55 min");
    expect(text).toContain("range 40–70 min");
    expect(text).toContain("Estimate");
  });

  it("wont-exhaust and no-headroom states", () => {
    const steady = render(<ForecastPill forecast={{ state: "wont-exhaust", badge: "measured" }} />);
    expect(steady.getByText(/won't exhaust this window/)).toBeTruthy();
    const noCap = render(<ForecastPill forecast={{ state: "no-headroom", badge: "computed" }} />);
    expect(noCap.getByText(/set your plan cap/)).toBeTruthy();
  });
});

describe("QuotaChip (NewSession rows)", () => {
  it("percentage + badge adjacent; typed copies for unknown/learning/unavailable", () => {
    const pct = render(<QuotaChip display={display()} />);
    expect(pct.getByText("82%")).toBeTruthy();
    expect(pct.getByText("Measured")).toBeTruthy();
    const none = render(<QuotaChip display={display({ five_hour: window({ state: "unknown", percentage: undefined }) })} />);
    expect(none.getByText("no quota data")).toBeTruthy();
    const learning = render(<QuotaChip display={display({ five_hour: window({ state: "learning", percentage: undefined }) })} />);
    expect(learning.getByText("learning")).toBeTruthy();
    const gemini = render(<QuotaChip display={display({ unavailable_reason: "No verified quota source for this Provider yet" })} />);
    expect(gemini.getByText("quota unavailable")).toBeTruthy();
  });
});

describe("AJ-E610 grep gates", () => {
  it("no percentage renders without an adjacent badge in this feature's source", async () => {
    const raw = (await import("./QuotaRings.tsx?raw")).default as string;
    // The ring is the only % renderer; its badge line must sit in the
    // same component as the number (adjacency asserted by the state
    // tests above; this pins the source shape).
    expect(raw).toContain("badgeText(window.badge)");
    expect(raw).not.toContain("percent={0}");
    expect(raw).not.toContain('>0%<');
    const newSessionRaw = (await import("../screens/NewSession.tsx?raw")).default as string;
    expect(newSessionRaw).toContain("QuotaChip");
    expect(newSessionRaw).not.toContain("percent={0}");
  });

  it("D-12: no-date accounts carry unknown copy and no inferred dates", () => {
    const fresh = display(); // no renewal_date anywhere
    const screen = render(<TwoClocks display={fresh} />);
    expect(screen.getByTestId("renewal-line").textContent).toContain("unknown");
    expect(screen.getByTestId("renewal-line").dataset.state).toBe("none");
  });
});

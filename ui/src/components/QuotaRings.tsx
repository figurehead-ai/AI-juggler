/**
 * AJ-E610 — the two clocks, rendered honestly:
 *
 * - The WINDOW clock: ring + `%` text (never colour-only, deep/10
 *   line 28) + reset countdown + confidence badge adjacent to the
 *   number. Typed states: no-data / learning / measured / stale /
 *   blocked / unavailable-with-reason — a missing number is never
 *   rendered as 0% (prior F2.4 defect class).
 * - The SUBSCRIPTION clock: a calendar-style date line — the two
 *   clocks never share a merged "time left" element (AJ-E606 Req.3).
 *
 * A11y: rings are `role=img` with a full text equivalent; badge text
 * is visible without hover; grayscale keeps every number + badge
 * readable (WCAG 2.2 AA 1.4.1/1.4.3).
 */
import type { QuotaDisplayData, WindowDisplay } from "../lib/bridge";

const badgeText = (badge?: string): string =>
  badge === "measured" ? "Measured" : badge === "computed" ? "Computed" : "Estimate";

const clock = (ms: number): string => {
  const date = new Date(ms);
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
};

const hoursAgo = (ms: number): string => {
  const minutes = Math.max(1, Math.round(ms / 60_000));
  return minutes < 60 ? `${minutes}m` : `${Math.round(minutes / 60)}h`;
};

/** The window clock: ring + text, every typed state. */
export function WindowRing({
  window,
  kind,
  omittedReason,
}: {
  window?: WindowDisplay;
  kind: "5h" | "weekly";
  omittedReason?: string;
}): JSX.Element {
  if (omittedReason) {
    return (
      <p className="text-xs text-muted" data-testid={`ring-${kind}-omitted`}>
        {omittedReason}
      </p>
    );
  }
  if (!window) return <></>;
  const label = kind === "5h" ? "5-hour window" : "weekly window";

  // Typed empty/unavailable states — copy exactly (deep/10 lines 29).
  if (window.state === "unknown") {
    return (
      <div data-testid={`ring-${kind}`} role="img" aria-label={`${label}: no data yet`}>
        <p className="text-xs text-muted">Start a session to measure</p>
      </div>
    );
  }
  if (window.state === "learning") {
    return (
      <div data-testid={`ring-${kind}`} role="img" aria-label={`${label}: learning`}>
        <p className="text-xs text-muted">learning — no forecast yet</p>
      </div>
    );
  }

  const pct = window.percentage;
  const used = pct ?? 100; // exhausted ⇒ full ring
  const exhausted = window.state === "exhausted";
  const ringText =
    pct !== undefined ? `${Math.round(pct)}%` : exhausted ? "100%" : "—";
  const reset =
    window.resets_at_ms !== undefined ? `resets ${clock(window.resets_at_ms)}` : "";
  const stale =
    window.stale_note_ms !== undefined
      ? `last measured ${hoursAgo(window.stale_note_ms)} ago`
      : "";
  const aria = `${label}: ${exhausted ? "exhausted" : `${ringText} used`}${
    pct !== undefined ? `, ${badgeText(window.badge)}` : ""
  }${reset ? `, ${reset}` : ""}${stale ? `, ${stale}` : ""}`;

  // Ring geometry: a 36px donut; the arc length encodes use (colour is
  // NEVER the only carrier — the % text sits inside).
  const radius = 15.5;
  const circumference = 2 * Math.PI * radius;
  const dash = (Math.min(100, Math.max(0, used)) / 100) * circumference;

  return (
    <div
      className="flex items-center gap-2"
      data-testid={`ring-${kind}`}
      data-state={window.state}
    >
      <svg
        width="36"
        height="36"
        viewBox="0 0 36 36"
        role="img"
        aria-label={aria}
        className={stale ? "opacity-60" : undefined}
      >
        <circle cx="18" cy="18" r={radius} fill="none" strokeWidth="3" className="stroke-hairline" />
        <circle
          cx="18"
          cy="18"
          r={radius}
          fill="none"
          strokeWidth="3"
          strokeDasharray={`${dash} ${circumference - dash}`}
          transform="rotate(-90 18 18)"
          className={exhausted ? "stroke-danger" : "stroke-accent"}
        />
        <text
          x="18"
          y="21"
          textAnchor="middle"
          className="fill-ink text-[9px] font-medium"
        >
          {ringText}
        </text>
      </svg>
      <div className="flex flex-col">
        <span className="text-xs">
          {label} · {badgeText(window.badge)}
        </span>
        <span className="text-[10px] text-muted">
          {[reset, stale].filter(Boolean).join(" · ") || `${window.burned_tokens.toLocaleString()} tokens`}
        </span>
        {exhausted && (
          <span className="text-[10px] text-danger">blocked — resets {reset || "unknown"}</span>
        )}
      </div>
    </div>
  );
}

/** The subscription clock: a calendar-style line, separate by type. */
export function RenewalLine({
  renewal,
  onAddDate,
}: {
  renewal: QuotaDisplayData["renewal"];
  onAddDate?: () => void;
}): JSX.Element {
  const tone =
    renewal.state === "overdue"
      ? "text-danger"
      : renewal.state === "approaching"
        ? "text-amber-500"
        : "text-muted";
  return (
    <p className={`text-xs ${tone}`} data-testid="renewal-line" data-state={renewal.state}>
      <span aria-hidden="true">📅 </span>
      {renewal.label}
      {renewal.state === "none" && onAddDate && (
        <button type="button" className="ml-2 underline focus:outline-none focus:ring-2 focus:ring-accent" onClick={onAddDate}>
          add date
        </button>
      )}
    </p>
  );
}

/** Both clocks for one account — siblings, never merged (Req.2). */
export function TwoClocks({ display }: { display: QuotaDisplayData }): JSX.Element {
  if (display.unavailable_reason) {
    return (
      <p className="text-xs text-muted" data-testid="quota-unavailable">
        {display.unavailable_reason}
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-2" data-testid="two-clocks">
      <div className="flex flex-wrap gap-4">
        <WindowRing window={display.five_hour} kind="5h" omittedReason={display.five_hour_omitted_reason} />
        <WindowRing window={display.weekly} kind="weekly" omittedReason={display.weekly_omitted_reason} />
      </div>
      <RenewalLine renewal={display.renewal} />
      <ForecastPill forecast={display.forecast} />
    </div>
  );
}

/** The forecast pill (deep/10 §1): Estimated renders the RANGE. */
export function ForecastPill({
  forecast,
}: {
  forecast: QuotaDisplayData["forecast"];
}): JSX.Element {
  const badge = badgeText(forecast.badge);
  const text =
    forecast.state === "learning"
      ? "learning — no forecast yet"
      : forecast.state === "wont-exhaust"
        ? "→ steady · won't exhaust this window"
        : forecast.state === "no-headroom"
          ? "set your plan cap to see a forecast"
          : forecast.tte_min !== undefined &&
              forecast.range_min !== undefined &&
              forecast.range_max !== undefined
            ? `↑ fast · exhausts ≈${Math.round(forecast.tte_min)} min` +
              (forecast.range_max - forecast.range_min > 1
                ? ` (range ${Math.round(forecast.range_min)}–${Math.round(forecast.range_max)} min, ${badge})`
                : `, ${badge}`)
            : "exhausts soon";
  return (
    <p className="text-[11px] text-muted" data-testid="forecast-pill" data-state={forecast.state}>
      {text}
    </p>
  );
}

/** AJ-E610: the compact row chip (NewSession rows) — percentage +
 * badge adjacent, or the typed state's copy; never a bare number. */
export function QuotaChip({ display }: { display?: QuotaDisplayData }): JSX.Element | null {
  if (!display) return null;
  if (display.unavailable_reason) {
    return <span className="text-[10px] text-faint">quota unavailable</span>;
  }
  const window = display.five_hour ?? display.weekly;
  if (!window || window.state === "unknown") {
    return <span className="text-[10px] text-faint">no quota data</span>;
  }
  if (window.state === "learning") {
    return <span className="text-[10px] text-faint">learning</span>;
  }
  if (window.percentage !== undefined) {
    return (
      <span className="text-[10px]">
        <span className="tabular-nums">{Math.round(window.percentage)}%</span>{" "}
        <span className="text-faint">{badgeText(window.badge)}</span>
      </span>
    );
  }
  return <span className="text-[10px] text-danger">blocked</span>;
}

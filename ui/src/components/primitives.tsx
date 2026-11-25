import { forwardRef } from "react";
import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from "react";
import { SELECTED_CARD } from "./controls/selected";
import { RUNWAY_FILL, runwayState } from "../lib/runway-look";

/**
 * Base UI primitives, shadcn-style: hand-vendored, token-driven, no
 * runtime dependencies. Screen-level components compose from these.
 *
 * AJ-E1550 (PAT-SURF): calm surfaces. Separation by tone, never bright
 * lines: a card is its surface step (no border, shadow or sheen); buttons
 * and tags are fills (the secondary a neutral control fill, the accent a
 * tint), their edge transparent so sizes hold; the accent's lines are
 * keyboard focus (styles.css) and rare true attention only. Increase
 * Contrast brings the edges back (`--color-frame`).
 */

type ButtonVariant = "primary" | "accent" | "secondary" | "ghost" | "danger" | "chosen";
type ButtonSize = "sm" | "md";

/** The primary's fill (0.3.2, D-62 decision 23: one solid colour), for
 * the few primaries drawn without `Button` (the switch dialogs). */
export const PRIMARY_FILL = "border border-transparent bg-accent-fill text-white hover:brightness-110";

const buttonVariants: Record<ButtonVariant, string> = {
  primary: `${PRIMARY_FILL} active:translate-y-px shadow-glow-accent`,
  // An app-wide action that's always there (the top bar's New session, as
  // the rail's): tinted, so a page's own primary is the one filled button
  // (principle 9).
  accent: "border border-frame bg-accent-soft text-accent-hi hover:bg-accent/15 active:translate-y-px",
  // AJ-E1550: a fill (the neutral control tone), not a stroke; the ring
  // only on keyboard focus.
  secondary: "bg-control text-ink border border-frame hover:bg-control-hi active:translate-y-px",
  ghost: "text-muted hover:text-ink hover:bg-surface-2",
  // At /15 its red stood 4.48:1 on surface-2 and 4.2:1 on surface-3 in
  // dark (axe, the connector card's Remove anyway, 0.4.6); /10 keeps AA on
  // every surface it sits on.
  danger: "bg-danger/10 text-danger border border-frame hover:bg-danger/20",
  // D-117 (selection): a choice that is picked (an ask's answer, a toggle
  // among options): the one selected look, semibold (below), never the
  // primary's fill — selection is not an action, and never loud.
  chosen: `border ${SELECTED_CARD} text-selected-ink active:translate-y-px`,
};

const buttonSizes: Record<ButtonSize, string> = {
  sm: "h-7 px-2.5 text-xs gap-1.5 rounded-lg",
  md: "h-9 px-3.5 text-sm gap-2 rounded-xl",
};

/**
 * D-126 — one filled primary per view. A `primary` Button marks itself
 * (`data-primary`); `primariesIn(view)` finds them, so a view's test
 * holds it to one. Every other action is `secondary` (or `ghost`, or
 * `accent` for an app-wide one); FlowLayout's foot takes one primary.
 */
const PRIMARY_MARK = "data-primary";

/** The filled primaries in `view` (a view shows one). */
export function primariesIn(view: ParentNode): HTMLElement[] {
  return [...view.querySelectorAll<HTMLElement>(`[${PRIMARY_MARK}]`)];
}

/** Ref-forwarding (AJ-E301: the wizard focuses its primary button on
 * every step change — focus management needs the ref). */
export const Button = forwardRef<HTMLButtonElement, ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
}>(function Button(
  {
    variant = "secondary",
    size = "md",
    className = "",
    type,
    ...props
  },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type ?? "button"}
      {...(variant === "primary" ? { [PRIMARY_MARK]: "" } : variant === "danger" ? { "data-danger": "" } : {})}
      className={`inline-flex items-center justify-center ${variant === "chosen" ? "font-semibold" : "font-medium"}
        transition-all duration-150 disabled:opacity-40 disabled:pointer-events-none
        cursor-pointer disabled:cursor-default select-none ${buttonVariants[variant]} ${buttonSizes[size]} ${className}`}
      {...props}
    />
  );
});

export const Card = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(function Card({ className = "", children, ...props }, ref) {
  return (
    <div
      ref={ref}
      className={`rounded-card border border-frame bg-surface ${className}`}
      {...props}
    >
      {children}
    </div>
  );
});

export type ChipTone =
  | "neutral"
  | "success"
  | "warning"
  | "danger"
  | "live"
  | "accent";

/** 0.5.1 F76 (PAT-COLOR-2, COL-5): amber is never a fill behind words.
 * A warning chip is the neutral chip's tone with its words, and the mark
 * they carry (⚠, ▲, its dot), in amber. Every chip, pill and tag that
 * says a warning takes its look from here: Chip, Pill, the session
 * header's state, the room chip, a figure's change. */
export const WARNING_CHIP = "bg-surface-2 text-warning";

const chipTones: Record<ChipTone, string> = {
  neutral: "bg-surface-2 text-muted border-frame",
  success: "bg-success-soft text-success border-frame",
  warning: `${WARNING_CHIP} border-frame`,
  danger: "bg-danger-soft text-danger border-frame",
  live: "bg-live-soft text-live border-frame",
  accent: "bg-accent-soft text-accent-hi border-frame",
};

export function Chip({
  tone = "neutral",
  dot = false,
  breathing = false,
  children,
}: {
  tone?: ChipTone;
  dot?: boolean;
  breathing?: boolean;
  children: ReactNode;
}) {
  return (
    <span
      className={`inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border px-2 py-0.5
        text-label font-medium tracking-wide ${chipTones[tone]}`}
    >
      {dot && (
        <span
          className={`size-1.5 rounded-full bg-current ${breathing ? "dot-live" : ""}`}
          style={
            breathing
              ? ({ "--pulse-color": "currentColor" } as React.CSSProperties)
              : undefined
          }
        />
      )}
      {children}
    </span>
  );
}

export function StatCard({
  label,
  value,
  hint,
  tone = "neutral",
  icon,
}: {
  label: string;
  value: string;
  hint?: string;
  tone?: ChipTone;
  icon?: ReactNode;
}) {
  const valueClass =
    tone === "live"
      ? "text-live"
      : tone === "success"
        ? "text-success"
        : tone === "warning"
          ? "text-warning"
          : "numeral";
  const iconTone =
    tone === "live"
      ? "text-live bg-live-soft"
      : tone === "success"
        ? "text-success bg-success-soft"
        : "text-accent-hi bg-accent-soft";
  return (
    <Card className="group p-4 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-label font-semibold text-muted">
          {label}
        </span>
        {icon && (
          <span
            className={`flex size-7 items-center justify-center rounded-lg ${iconTone}`}
          >
            {icon}
          </span>
        )}
      </div>
      <span className={`text-stat leading-8 font-semibold tabular-nums ${valueClass}`}>
        {value}
      </span>
      {hint && <span className="text-xs text-muted">{hint}</span>}
    </Card>
  );
}

/** I-05 quota tier word — always adjacent to the number (WCAG/NN-g:
 * recognition over recall). SPEC §6's tiers in plain words (AJ-E1535:
 * never "MEAS"/"COMP"/"EST"). */
export function quotaTierTag(confidence: string): string {
  if (confidence === "measured") return "measured";
  if (confidence === "computed") return "worked out";
  return "estimate";
}

/** I-05: the quota bar for a row/card — filled by `% left`, reset
 * countdown when known, tier tag always adjacent, and the exact honest
 * empty state: dashed track + `Not measured yet` (never a fabricated 0). */
export function QuotaBar({
  quota,
  className = "",
}: {
  quota: { used_percentage: number; resets_at_ms: number | null; confidence: string } | null;
  className?: string;
}) {
  if (!quota) {
    return (
      <span
        className={`inline-flex min-w-0 items-center gap-1.5 ${className}`}
        title="Quota: not measured yet"
      >
        <span className="inline-block h-1.5 w-14 rounded-full border border-dashed border-faint/70" />
        <span className="whitespace-nowrap text-label text-muted">Not measured yet</span>
      </span>
    );
  }
  const left = Math.max(0, 100 - quota.used_percentage);
  // PAT-COLOR: the room's colour (lib/runway-look), as every runway bar.
  const tone = RUNWAY_FILL[runwayState(Math.round(left))];
  const reset = quota.resets_at_ms !== null ? ` · resets ${formatCountdown(quota.resets_at_ms)}` : "";
  return (
    <span
      className={`inline-flex min-w-0 items-center gap-1.5 ${className}`}
      title={`Quota: ${Math.round(left)}% left${reset} · ${quotaTierTag(quota.confidence)}`}
    >
      <span className="relative inline-block h-1.5 w-14 overflow-hidden rounded-full bg-surface-3">
        <span
          className={`absolute inset-y-0 left-0 rounded-full ${tone}`}
          style={{ width: `${Math.max(2, quota.used_percentage)}%` }}
        />
      </span>
      <span className="whitespace-nowrap text-label tabular-nums text-muted">
        {Math.round(left)}% left
      </span>
      <span className="shrink-0 whitespace-nowrap text-label text-muted">
        {quotaTierTag(quota.confidence)}
      </span>
    </span>
  );
}

/** A reset countdown in words: "in 45 min", "in 2 h 10 min", "in 1 d 3 h"
 * (the review of 0.1.4: "reset: 2h0m"). */
export function formatCountdown(resetsAtMs: number, nowMs: number = Date.now()): string {
  const ms = resetsAtMs - nowMs;
  if (ms <= 0) return "now";
  const minutes = Math.round(ms / 60_000);
  if (minutes < 60) return `in ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return minutes % 60 === 0 ? `in ${hours} h` : `in ${hours} h ${minutes % 60} min`;
  return hours % 24 === 0 ? `in ${Math.floor(hours / 24)} d` : `in ${Math.floor(hours / 24)} d ${hours % 24} h`;
}

/**
 * I-07: trailing dual mini sparkline — RAM solid, CPU% outline (legend
 * contract, AJ-E403). `samples` is the honest measured history (oldest →
 * newest); fewer than two samples renders the placeholder dash — never
 * an invented flat line.
 */
export function Sparkline({
  series,
  width = 46,
  height = 14,
  title,
}: {
  series: { cpu: number[]; ram: number[] };
  width?: number;
  height?: number;
  title?: string;
}) {
  if (series.cpu.length < 2 || series.ram.length < 2) {
    return (
      <span className="text-label leading-none text-muted" title={title ?? "resources unavailable"}>
        —
      </span>
    );
  }
  const line = (values: number[], scaleMax: number) => {
    const max = Math.max(...values, scaleMax, 1);
    const step = width / (values.length - 1);
    return values
      .map((value, i) => `${(i * step).toFixed(1)},${(height - (value / max) * (height - 2) - 1).toFixed(1)}`)
      .join(" ");
  };
  return (
    <svg
      width={width}
      height={height}
      className="shrink-0"
      role="img"
      aria-label={title ?? "resource history"}
      focusable="false"
    >
      {/* solid = RAM */}
      <polyline points={line(series.ram, 1)} fill="none" stroke="var(--color-accent-hi)" strokeWidth={1.6} strokeLinejoin="round" />
      {/* outline = CPU% */}
      <polyline points={line(series.cpu, 100)} fill="none" stroke="var(--color-live)" strokeWidth={1.1} strokeDasharray="2 1.6" strokeLinejoin="round" />
    </svg>
  );
}

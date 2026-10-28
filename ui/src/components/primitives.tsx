import { forwardRef } from "react";
import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from "react";

/**
 * Base UI primitives, shadcn-style: hand-vendored, token-driven, no
 * runtime dependencies. Screen-level components compose from these.
 *
 * Iteration 2: layered depth (sheen edge, card shadow), primary buttons
 * carry a quiet accent glow, live elements breathe.
 */

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md";

const buttonVariants: Record<ButtonVariant, string> = {
  primary:
    "bg-gradient-to-b from-accent-hi to-accent text-white hover:brightness-110 \
     active:translate-y-px shadow-glow-accent border border-accent-hi/40",
  secondary:
    "bg-surface-2 text-ink border border-hairline hover:bg-surface-3 \
     hover:border-muted/30 active:translate-y-px",
  ghost: "text-muted hover:text-ink hover:bg-surface-2",
  danger:
    "bg-danger/15 text-danger border border-danger/30 hover:bg-danger/25",
};

const buttonSizes: Record<ButtonSize, string> = {
  sm: "h-7 px-2.5 text-xs gap-1.5 rounded-lg",
  md: "h-9 px-3.5 text-sm gap-2 rounded-xl",
};

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
      className={`inline-flex items-center justify-center font-medium
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
      className={`sheen rounded-card border border-hairline bg-surface
        shadow-card transition-all duration-150
        hover:border-muted/25 ${className}`}
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

const chipTones: Record<ChipTone, string> = {
  neutral: "bg-surface-2 text-muted border-hairline",
  success: "bg-success-soft text-success border-success/25",
  warning: "bg-warning-soft text-warning border-warning/25",
  danger: "bg-danger-soft text-danger border-danger/25",
  live: "bg-live-soft text-live border-live/25",
  accent: "bg-accent-soft text-accent-hi border-accent/25",
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
      ? "text-live bg-live-soft border-live/20"
      : tone === "success"
        ? "text-success bg-success-soft border-success/20"
        : "text-accent-hi bg-accent-soft border-accent/20";
  return (
    <Card className="group p-4 flex flex-col gap-2 hover:-translate-y-0.5">
      <div className="flex items-center justify-between">
        <span className="text-label font-semibold text-muted">
          {label}
        </span>
        {icon && (
          <span
            className={`flex size-7 items-center justify-center rounded-lg border ${iconTone}`}
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
  const tone =
    left <= 10 ? "bg-danger" : left <= 30 ? "bg-warning" : "bg-accent";
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

/**
 * ADR-0021 — a limit window at a glance: a ring of what's left (the
 * fill turns amber, then red, as it runs out), the window's name, and
 * when it resets. Not measured is said in words, never drawn as empty.
 */
import { READING_SHORT } from "../../lib/history-reading";
import type { JSX } from "react";

/** `omitted`: the plan has no such limit; its note says so, alone. */
export type GaugeState = "measured" | "unmeasured" | "measuring" | "reading" | "stale" | "omitted";

/** What's left decides the ring's tone: plenty, getting low, nearly out. */
export function gaugeTone(left: number): string {
  if (left <= 10) return "var(--color-danger)";
  if (left <= 30) return "var(--color-warning)";
  return "var(--color-accent)";
}

export function Ring({ left, size = 60, color }: { left: number | null; size?: number; color?: string }): JSX.Element {
  const stroke = 7;
  const r = size / 2 - stroke / 2 - 1;
  const c = size / 2;
  const length = 2 * Math.PI * r;
  const share = left === null ? 0 : Math.max(0, Math.min(100, left)) / 100;
  return (
    <svg width={size} height={size} aria-hidden="true" className="shrink-0">
      <circle cx={c} cy={c} r={r} fill="none" stroke={left === 0 ? "color-mix(in oklab, var(--color-danger) 45%, var(--color-surface-3))" : "var(--color-surface-3)"} strokeWidth={stroke} />
      {left !== null && share > 0 && (
        <circle
          cx={c}
          cy={c}
          r={r}
          fill="none"
          stroke={color ?? gaugeTone(left)}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={`${length * share} ${length}`}
          transform={`rotate(-90 ${c} ${c})`}
        />
      )}
      <text x={c} y={c + 5} textAnchor="middle" className="fill-ink text-[14px] font-bold tabular-nums">
        {left === null ? "—" : `${Math.round(left)}%`}
      </text>
    </svg>
  );
}

/** One window: its ring, its name and its reset. */
export function Gauge({
  name,
  left,
  resets,
  resetsIn,
  state = "measured",
  note,
  testId,
}: {
  name: string;
  /** Percent left; null when not measured. */
  left: number | null;
  /** The reset in words ("16:20", "Fri 09:00"). */
  resets?: string | null;
  /** How long until then ("in 40 min"). */
  resetsIn?: string | null;
  state?: GaugeState;
  /** A line under it (the reading's age, an estimate's basis). */
  note?: string | null;
  testId?: string;
}): JSX.Element {
  // The 0.2 walk (F18): "Not measured yet · Codex reports no five-hour
  // limit" said two things; a limit the plan hasn't is said once.
  const omitted = state === "omitted" && !!note;
  const status = omitted ? note : state === "measuring" ? "Measuring…" : state === "reading" ? READING_SHORT : state === "stale" ? "Not read since its reset" : left === null ? "Not measured yet" : null;
  const said =
    status !== null
      ? `${name}: ${status}${note && !omitted ? `. ${note}` : ""}`
      : `${name}: ${Math.round(left!)}% left${resets ? `, resets ${resets}` : ""}${resetsIn ? `, ${resetsIn}` : ""}`;
  return (
    <div className="flex min-w-0 items-center gap-3" role="img" aria-label={said} data-testid={testId ?? "gauge"} data-state={state}>
      <Ring left={status === null ? left : null} />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-body font-semibold text-ink">{name}</span>
        {status !== null ? (
          <span className="text-label text-muted">{status}</span>
        ) : (
          <>
            <span className="text-label text-ink">
              {Math.round(left!)}% left{resets && <span className="text-muted"> · resets {resets}</span>}
            </span>
            {resetsIn && <span className="text-label text-muted">{resetsIn}</span>}
          </>
        )}
        {note && !omitted && <span className="text-label text-faint">{note}</span>}
      </span>
    </div>
  );
}

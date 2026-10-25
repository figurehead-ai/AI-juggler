/**
 * ADR-0021 — the chart kit's maths: one linear scale with "nice" ticks
 * (so every axis label names a value the chart reaches), the stacked
 * layout, and the words numbers are drawn in. Every chart places marks,
 * ticks and labels with these, never by hand.
 */
import { tokenWords } from "../../lib/quotaWindow";

/** A clean step near `raw`: 1, 2, 2.5 or 5 times a power of ten. */
export function niceStep(raw: number): number {
  if (!(raw > 0) || !Number.isFinite(raw)) return 1;
  const power = 10 ** Math.floor(Math.log10(raw));
  const n = raw / power;
  return (n <= 1 ? 1 : n <= 2 ? 2 : n <= 2.5 ? 2.5 : n <= 5 ? 5 : 10) * power;
}

/** An axis for values up to `max`: about `count` steps, its top a step. */
export function niceAxis(max: number, count = 4): { top: number; ticks: number[] } {
  if (!(max > 0)) return { top: 1, ticks: [0, 1] };
  const step = niceStep(max / count);
  const top = Math.ceil(max / step - 1e-9) * step;
  const ticks: number[] = [];
  for (let value = 0; value <= top + step / 2; value += step) ticks.push(Number(value.toPrecision(12)));
  return { top, ticks };
}

/** A linear map from `[0, top]` to `[from, to]` (pixels; `to` may be the smaller). */
export function linear(top: number, from: number, to: number): (value: number) => number {
  const span = top > 0 ? top : 1;
  return (value) => from + ((to - from) * Math.max(0, value)) / span;
}

/** One series of a stacked chart: its key and a value per bucket. */
export type Series = { key: string; label: string; values: number[]; color: string };

/** Each bucket's stack: segments bottom-up in the series' order, zeros left out. */
export function stackOf(series: Series[], bucket: number): { key: string; from: number; to: number }[] {
  let at = 0;
  const out: { key: string; from: number; to: number }[] = [];
  for (const entry of series) {
    const value = entry.values[bucket] ?? 0;
    if (value <= 0) continue;
    out.push({ key: entry.key, from: at, to: at + value });
    at += value;
  }
  return out;
}

/** Each bucket's total across the series. */
export function totalsOf(series: Series[], buckets: number): number[] {
  const totals = new Array<number>(buckets).fill(0);
  for (const entry of series) for (let i = 0; i < buckets; i += 1) totals[i]! += Math.max(0, entry.values[i] ?? 0);
  return totals;
}

/** What a chart's values are. */
export type Unit = "tokens" | "usd" | "count" | "hours" | "percent";

/** A value, short, in its unit: "8.0M", "$490", "12", "42 h", "62%". */
export function short(value: number, unit: Unit): string {
  switch (unit) {
    case "tokens":
      return tokenWords(Math.round(value));
    case "usd":
      return usdWords(value);
    case "hours":
      return `${Math.round(value)} h`;
    case "percent":
      return `${Math.round(value)}%`;
    default:
      return value >= 10_000 ? tokenWords(Math.round(value)) : String(Math.round(value));
  }
}

/** A value in full, for hover: "8,012,345 tokens", "$490.12". */
export function exact(value: number, unit: Unit): string {
  switch (unit) {
    case "tokens":
      return `${Math.round(value).toLocaleString("en-US")} tokens`;
    case "usd":
      return `$${value.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
    case "hours":
      return `${Math.round(value)} hours`;
    case "percent":
      return `${value.toFixed(1)}%`;
    default:
      return Math.round(value).toLocaleString("en-US");
  }
}

/** Dollars, short: "$0.42", "$12", "$1,284", "$12.3k". */
export function usdWords(value: number): string {
  if (!Number.isFinite(value)) return "—";
  const abs = Math.abs(value);
  if (abs >= 100_000) return `$${(value / 1000).toFixed(0)}k`;
  if (abs >= 10_000) return `$${(value / 1000).toFixed(1)}k`;
  if (abs >= 100) return `$${Math.round(value).toLocaleString("en-US")}`;
  if (abs >= 10) return `$${value.toFixed(0)}`;
  if (abs === 0) return "$0";
  return `$${value.toFixed(2)}`;
}

/** A share in whole percent, "<1%" for a sliver, of a total. */
export function percentOf(value: number, total: number): string {
  if (!(total > 0) || value <= 0) return "0%";
  const pct = (value / total) * 100;
  return pct < 1 ? "<1%" : `${Math.round(pct)}%`;
}

/** The change from `before` to `now`, as the tiles say it. */
export function change(now: number, before: number): { dir: "up" | "down" | "same" | "new"; words: string } {
  if (!(before > 0)) return now > 0 ? { dir: "new", words: "new" } : { dir: "same", words: "same" };
  const pct = ((now - before) / before) * 100;
  if (Math.abs(pct) < 1) return { dir: "same", words: "same" };
  return { dir: pct > 0 ? "up" : "down", words: `${Math.round(Math.abs(pct))}%` };
}

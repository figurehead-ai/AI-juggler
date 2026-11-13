/**
 * AJ-E1503 Req.4 — while a list loads it shows placeholders shaped like
 * its rows, never blank space and never a false "nothing here". Each
 * shape follows the real row it stands in for: the rail's session row,
 * Home's row, an account card. Quiet bars on the surface tokens; the
 * shimmer is a pulse that stops under reduced motion.
 */
import type { JSX } from "react";

export type PlaceholderShape = "rail" | "home" | "card";

function Bar({ className }: { className: string }): JSX.Element {
  return <span className={`block rounded bg-skeleton ${className}`} />;
}

function RailRow({ index }: { index: number }): JSX.Element {
  return (
    <div className="flex items-center gap-2.5 rounded-lg py-2 pl-2.5 pr-2">
      <span className="flex w-4 shrink-0 justify-center">
        <span className="size-2.5 rounded-full bg-skeleton" />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-1.5">
        <Bar className={`h-2.5 ${index % 2 === 0 ? "w-3/4" : "w-2/3"}`} />
        <Bar className={`h-2 ${index % 2 === 0 ? "w-1/2" : "w-2/5"}`} />
      </span>
    </div>
  );
}

function HomeRow({ index }: { index: number }): JSX.Element {
  return (
    <div className="flex items-center gap-4 rounded-card border border-hairline bg-surface px-4 py-3.5">
      <span className="flex min-w-0 flex-1 flex-col gap-2">
        <Bar className={`h-3 ${index % 2 === 0 ? "w-1/3" : "w-2/5"}`} />
        <Bar className={`h-2.5 ${index % 2 === 0 ? "w-2/3" : "w-1/2"}`} />
      </span>
      <Bar className="h-7 w-20 rounded-lg" />
    </div>
  );
}

function CardRow({ index }: { index: number }): JSX.Element {
  return (
    <div className="flex flex-col gap-3 rounded-card border border-hairline bg-surface p-4">
      <span className="flex items-center gap-2">
        <span className="size-7 rounded-lg bg-skeleton" />
        <span className="flex flex-1 flex-col gap-1.5">
          <Bar className={`h-3 ${index % 2 === 0 ? "w-1/3" : "w-1/4"}`} />
          <Bar className="h-2 w-1/5" />
        </span>
      </span>
      <Bar className="h-1.5 w-1/2 rounded-full" />
    </div>
  );
}

const ROWS: Record<PlaceholderShape, (props: { index: number }) => JSX.Element> = { rail: RailRow, home: HomeRow, card: CardRow };

/** `rows` placeholders of one shape, announced once as loading. */
export function RowPlaceholders({
  shape,
  rows = 3,
  label,
}: {
  shape: PlaceholderShape;
  rows?: number;
  /** What is loading ("Loading sessions"). */
  label: string;
}): JSX.Element {
  const Row = ROWS[shape];
  return (
    <div
      role="status"
      aria-label={label}
      aria-busy="true"
      data-testid={`placeholder-${shape}`}
      className={"flex flex-col motion-safe:animate-pulse " + (shape === "rail" ? "gap-0.5 px-1.5" : "gap-2.5")}
    >
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} aria-hidden="true">
          <Row index={index} />
        </div>
      ))}
    </div>
  );
}

/** A text line's box (`1lh` of its type), so a placeholder line is as
 * tall as the line it stands in for. */
function Line({ text, className }: { text: string; className: string }): JSX.Element {
  return (
    <span className={`flex h-[1lh] items-center ${text}`}>
      <Bar className={className} />
    </span>
  );
}

/** D-103.6: Home's Needs you while it loads, as large as the card: its
 * first need (mark, title, why, what it asks; its buttons), the rest one
 * line each. `height`: the card's last height, so the same needs arriving
 * move nothing. */
export function NeedsPlaceholder({ rows, height, label }: { rows: number; height: number | null; label: string }): JSX.Element {
  return (
    <output aria-label={label} aria-busy="true" data-testid="placeholder-need" data-rows={rows} className="flex flex-col motion-safe:animate-pulse">
      <div aria-hidden="true" className="flex flex-col gap-3 overflow-hidden rounded-xl border border-hairline bg-surface p-4" style={height ? { height } : undefined}>
        <span className="flex items-start gap-3">
          <span className="size-8 shrink-0 rounded-lg bg-skeleton" />
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <Line text="text-section" className="h-3.5 w-1/2" />
            <Line text="text-xs" className="h-2.5 w-2/3" />
            <Line text="text-meta" className="h-2.5 w-2/5" />
          </span>
        </span>
        <span className="flex items-center gap-2">
          <Bar className="h-7 w-32 rounded-lg" />
          <Bar className="h-7 w-44 rounded-lg" />
        </span>
        {rows > 1 && (
          <span className="flex flex-col border-t border-hairline/60 pt-1">
            {Array.from({ length: rows - 1 }, (_, index) => (
              <span key={index} className="flex items-center gap-2.5 py-2">
                <span className="size-6 shrink-0 rounded-md bg-skeleton" />
                <span className="flex min-w-0 flex-1 flex-col">
                  <Line text="text-meta" className={`h-2.5 ${index % 2 === 0 ? "w-1/3" : "w-2/5"}`} />
                  <Line text="text-label" className="h-2 w-2/3" />
                </span>
                <Bar className="h-7 w-24 rounded-lg" />
              </span>
            ))}
          </span>
        )}
      </div>
    </output>
  );
}

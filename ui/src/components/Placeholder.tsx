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
  const Row = shape === "rail" ? RailRow : shape === "home" ? HomeRow : CardRow;
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

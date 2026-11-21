/** An update's download: how far, once there's a figure (no figure is
 * no figure, never "0%"). */
import type { JSX } from "react";

export function UpdateProgress({ progress }: { progress: number | null | undefined }): JSX.Element {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-ink">Downloading and verifying…{progress != null ? ` ${progress}%` : ""}</span>
      <ProgressBar label="Update download" percent={progress} />
    </div>
  );
}

/** The one thin bar for how far something long is (a percent; none yet
 * is an empty track, never "0%"). An update's download, a move's backup. */
export function ProgressBar({ label, percent, testId }: { label: string; percent: number | null | undefined; testId?: string }): JSX.Element {
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent ?? undefined}
      data-testid={testId}
      className="h-1.5 w-full overflow-hidden rounded-full bg-surface-2"
    >
      {percent != null && <div className="h-full rounded-full bg-accent" style={{ width: `${percent}%` }} />}
    </div>
  );
}

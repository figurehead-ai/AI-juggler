/** An update's download: how far, once there's a figure (no figure is
 * no figure, never "0%"). */
import type { JSX } from "react";

export function UpdateProgress({ progress }: { progress: number | null | undefined }): JSX.Element {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-ink">Downloading and verifying…{progress != null ? ` ${progress}%` : ""}</span>
      <div
        role="progressbar"
        aria-label="Update download"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={progress ?? undefined}
        className="h-1.5 w-full overflow-hidden rounded-full bg-surface-2"
      >
        {progress != null && <div className="h-full rounded-full bg-accent" style={{ width: `${progress}%` }} />}
      </div>
    </div>
  );
}

/**
 * The rail footer's word on this Mac: the same state This Mac shows on
 * Home (`machineStatus`), never a fixed "healthy" (QA, P19: the footer
 * said "this-mac · healthy" while macOS reported memory pressure). The
 * collapsed rail shows its dot, titled with the same words. With
 * `onOpen` it opens the This Mac page (AJ-E1528).
 */
import { useContext, type JSX } from "react";
import { ThisMacContext, machineStatus } from "../lib/machine";

export function MachineStatusLine({ minimal, onOpen }: { minimal: boolean; onOpen?: () => void }): JSX.Element {
  const status = machineStatus(useContext(ThisMacContext));
  // Before the first sample: the Mac's name only, a neutral dot.
  const words = status ? `This Mac · ${status.words}` : "This Mac";
  const dot = status ? status.dot : "bg-faint";
  if (minimal) {
    return (
      <div className="flex justify-center py-2" title={words} data-testid="machine-status-dot" data-level={status?.level ?? "unsampled"}>
        {onOpen ? (
          <button type="button" onClick={onOpen} aria-label={words} className="flex size-6 items-center justify-center rounded-md outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60">
            <span aria-hidden="true" className={`dot-live size-1.5 rounded-full ${dot}`} />
          </button>
        ) : (
          <span className={`dot-live size-1.5 rounded-full ${dot}`} aria-label={words} role="img" />
        )}
      </div>
    );
  }
  const line = (
    <>
      <span aria-hidden="true" className={`dot-live size-1.5 shrink-0 rounded-full ${dot}`} />
      <span className="min-w-0 shrink truncate text-[11px]">{words}</span>
    </>
  );
  return onOpen ? (
    <button type="button" onClick={onOpen} className="flex min-w-0 shrink items-center gap-2 rounded-md text-left outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="machine-status" data-level={status?.level ?? "unsampled"}>
      {line}
    </button>
  ) : (
    <span className="flex min-w-0 shrink items-center gap-2" data-testid="machine-status" data-level={status?.level ?? "unsampled"}>
      {line}
    </span>
  );
}

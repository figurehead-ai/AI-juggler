/**
 * The rail footer's word on this Mac: the same state This Mac shows on
 * Home (`machineStatus`), never a fixed "healthy" (QA, P19: the footer
 * said "this-mac · healthy" while macOS reported memory pressure). The
 * collapsed rail shows its dot, titled with the same words.
 */
import { useContext, type JSX } from "react";
import { ThisMacContext, machineStatus } from "../lib/machine";

export function MachineStatusLine({ minimal }: { minimal: boolean }): JSX.Element {
  const status = machineStatus(useContext(ThisMacContext));
  // Before the first sample: the Mac's name only, a neutral dot.
  const words = status ? `This Mac · ${status.words}` : "This Mac";
  const dot = status ? status.dot : "bg-faint";
  if (minimal) {
    return (
      <div className="flex justify-center py-2" title={words} data-testid="machine-status-dot" data-level={status?.level ?? "unsampled"}>
        <span className={`dot-live size-1.5 rounded-full ${dot}`} aria-label={words} role="img" />
      </div>
    );
  }
  return (
    <span className="flex min-w-0 shrink items-center gap-2" data-testid="machine-status" data-level={status?.level ?? "unsampled"}>
      <span aria-hidden="true" className={`dot-live size-1.5 shrink-0 rounded-full ${dot}`} />
      <span className="min-w-0 shrink truncate text-[11px]">{words}</span>
    </span>
  );
}

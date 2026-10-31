/**
 * The rail footer's word on this Mac: the same state This Mac shows on
 * Home (`machineStatus`), never a fixed "healthy" (QA, P19: the footer
 * said "this-mac · healthy" while macOS reported memory pressure). The
 * collapsed rail shows its dot, titled with the same words. With
 * `onOpen` it opens the This Mac page (AJ-E1528).
 */
import { useContext, type JSX } from "react";
import { ThisMacContext, machineStatus } from "../lib/machine";
import { PRESSURE_SHORT, underPressure } from "../lib/machine-memory";

/** The top bar's shorter words, so the bar stays one row at 1280 px with
 * the list open (0.3.2: "Under memory pressure" wrapped it); the title
 * and This Mac's page keep the full words. */
const SHORT_WORDS: Record<string, string> = { "Under memory pressure": "Under pressure" };

export function MachineStatusLine({ minimal, onOpen, short = false }: { minimal: boolean; onOpen?: () => void; short?: boolean }): JSX.Element {
  const info = useContext(ThisMacContext);
  const status = machineStatus(info);
  // Memory pressure, in one line, where the words alone don't say it.
  const why = status && info && underPressure(info.machine) ? PRESSURE_SHORT : undefined;
  // Before the first sample: the Mac's name only, a neutral dot.
  const name = "This Mac";
  const words = status ? `${name} · ${status.words}` : name;
  const shown = status && short && SHORT_WORDS[status.words] ? `${name} · ${SHORT_WORDS[status.words]}` : words;
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
      <span aria-hidden="true" className={`dot-live size-2 shrink-0 rounded-full ${dot}`} />
      {/* The readability bar: meta 13 px, in ink at 80 % (7:1 or better). */}
      <span className="min-w-0 shrink truncate text-meta">{shown}</span>
    </>
  );
  return onOpen ? (
    <button type="button" onClick={onOpen} title={why ?? (shown !== words ? words : undefined)} className="flex min-h-6 min-w-0 shrink items-center gap-2 rounded-md text-left text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="machine-status" data-level={status?.level ?? "unsampled"}>
      {line}
    </button>
  ) : (
    <span className="flex min-w-0 shrink items-center gap-2 text-muted" title={why} data-testid="machine-status" data-level={status?.level ?? "unsampled"}>
      {line}
    </span>
  );
}

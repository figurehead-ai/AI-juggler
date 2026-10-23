/**
 * AJ-E1007 — replay an ended or archived Instance, read-only: its
 * transcript in the terminal view (input disabled), its history (status
 * changes and lifecycle steps), its switch lineage and its process
 * record. The header says Replay · read-only, and the transcript's
 * sha256 proves it is the stored file.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { lazyNamed } from "../lib/lazy";

// xterm loads with the first replay, not at start.
const ReplayTerminal = lazyNamed(() => import("./ReplayTerminal"), "ReplayTerminal");
import { errorText, instanceReplay, type ReplayInfo } from "../lib/bridge";

function when(ms: number): string {
  return new Date(ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function ReplayDialog({ sessionId, onClose }: { sessionId: string; onClose: () => void }): JSX.Element {
  const [replay, setReplay] = useState<ReplayInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const closeRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    instanceReplay(sessionId)
      .then(setReplay)
      .catch((cause) => setError(errorText(cause)));
  }, [sessionId]);
  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="fixed inset-0 z-50 flex items-stretch justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="replay-title"
        data-testid="replay-dialog"
        className="flex w-full max-w-6xl flex-col gap-3 rounded-xl border border-hairline bg-surface p-4 shadow-card"
      >
        <div className="flex flex-wrap items-center gap-2">
          <h3 id="replay-title" className="text-sm font-semibold">
            Replay · read-only
          </h3>
          {replay && (
            <span className="min-w-0 truncate text-[11px] text-muted" data-testid="replay-proof">
              {replay.transcript_path
                ? `${size(replay.bytes)} · sha256 ${replay.sha256?.slice(0, 12) ?? "unavailable"}…`
                : "No transcript recorded"}
              {replay.truncated ? " · showing the last 16 MB" : ""}
            </span>
          )}
          <span className="flex-1" />
          <button
            ref={closeRef}
            type="button"
            onClick={onClose}
            className="rounded-lg border border-hairline px-3 py-1 text-xs text-ink"
            data-testid="replay-close"
          >
            Close
          </button>
        </div>
        {error && (
          <p role="alert" className="text-[11px] text-ink">
            {error}
          </p>
        )}
        {replay && (
          <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 lg:grid-cols-[1fr_260px]">
            <div className="min-h-[240px] overflow-hidden rounded-lg border border-hairline bg-[#05070b] p-2" data-testid="replay-transcript">
              {replay.entries.length === 0 ? (
                <p className="p-3 text-[11px] text-[#cdd5e0]">Nothing to replay — the transcript is empty or missing.</p>
              ) : (
                <ReplayTerminal entries={replay.entries} />
              )}
            </div>
            <div className="flex min-h-0 flex-col gap-3 overflow-y-auto text-[11px]">
              <section aria-labelledby="replay-history">
                <h4 id="replay-history" className="mb-1 font-semibold">
                  History
                </h4>
                <ol className="flex flex-col gap-0.5" data-testid="replay-history-list">
                  {replay.history.map((event, index) => (
                    <li key={index} className="flex justify-between gap-2">
                      <span className="text-ink">{event.kind === "lifecycle" ? `· ${event.what}` : event.what}</span>
                      <span className="text-muted">{when(event.at_ms)}</span>
                    </li>
                  ))}
                </ol>
              </section>
              <section aria-labelledby="replay-lineage">
                <h4 id="replay-lineage" className="mb-1 font-semibold">
                  Lineage
                </h4>
                {replay.lineage.length === 0 ? (
                  <p className="text-muted">Never switched.</p>
                ) : (
                  <ul className="flex flex-col gap-0.5" data-testid="replay-lineage-list">
                    {replay.lineage.map((step, index) => (
                      <li key={index} className="text-ink">
                        {step.direction === "in" ? "Arrived from" : "Moved to"} {step.other_account} ({step.other_provider})
                        <span className="text-muted">
                          {" "}
                          · {step.state}
                          {step.fidelity ? ` · ${step.fidelity}` : ""}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
              <section aria-labelledby="replay-processes">
                <h4 id="replay-processes" className="mb-1 font-semibold">
                  Processes
                </h4>
                {replay.processes.length === 0 ? (
                  <p className="text-muted">No process record — the ledger keeps one for 7 days after a process exits.</p>
                ) : (
                  <ul className="flex flex-col gap-0.5" data-testid="replay-process-list">
                    {replay.processes.map((process, index) => (
                      <li key={index} className="text-ink">
                        {process.role} {process.pid ?? "—"} · {process.state}
                        {process.exit_kind ? ` · ${process.exit_kind}` : ""}
                        <span className="text-muted"> · {when(process.first_seen_ms)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

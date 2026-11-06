/**
 * A switch that didn't complete (0.3.9): a short title, then the
 * failure's own words (what happened and what to do next, said once:
 * `SwitchFailure::words`), and Copy details: what the program said,
 * where it stopped and when, for a bug report, shown before it's copied
 * (the home folder written `~`), and Report a bug… (AJ-E2911) with the
 * switch's steps. Retry shows only when
 * trying again can succeed (`retryable`).
 */
import { useState, type JSX } from "react";

import { SwitchSetupResults } from "../SwitchSetupList";
import { openReportBug } from "../../lib/report-bug";
import type { AppliedInfo } from "../../lib/ipc/switchSetup";

/** What a failure carries beyond its words. */
export function failureOf(cause: unknown): { detail: string | null; retryable: boolean } {
  const wire = (cause ?? {}) as { detail?: unknown; retryable?: unknown };
  return {
    detail: typeof wire.detail === "string" ? wire.detail : null,
    retryable: wire.retryable !== false,
  };
}

/** The text Copy details puts on the clipboard. */
export function detailsText(target: string, message: string, detail: string | null, at: Date = new Date()): string {
  return [`AI Juggler · a switch to ${target} didn't complete · ${at.toISOString()}`, `What it said: ${message}`, `Details: ${detail ?? "none recorded"}`].join("\n");
}

export function FailedStep({ target, message, cause, results }: { target: string; message: string; cause: unknown; results: AppliedInfo[] }): JSX.Element {
  const { detail } = failureOf(cause);
  const [copied, setCopied] = useState<"yes" | "no" | null>(null);
  const copy = () =>
    void navigator.clipboard
      ?.writeText(detailsText(target, message, detail))
      .then(
        () => setCopied("yes"),
        () => setCopied("no"),
      );
  return (
    <div className="flex flex-col gap-4" data-testid="switch-failed">
      <h3 className="text-title text-warning">The switch didn't complete</h3>
      <p className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning" data-testid="switch-failed-words">
        {message}
      </p>
      <div className="flex items-center gap-3">
        <button type="button" onClick={copy} className="rounded-lg border border-hairline px-2.5 py-1 text-meta text-ink outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="switch-copy-details">
          Copy details
        </button>
        {/* AJ-E2911: the same facts and the switch's steps, shown first. */}
        <button
          type="button"
          onClick={() => openReportBug({ from: "switch", words: message, detail, atMs: Date.now() })}
          className="rounded-lg border border-hairline px-2.5 py-1 text-meta text-ink outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
          data-testid="switch-report-bug"
        >
          Report a bug…
        </button>
        {copied && (
          <span className="text-meta text-ink-2" role="status">
            {copied === "yes" ? "Copied. Paste it into a new issue at github.com/figurehead-ai/AI-juggler/issues." : "Couldn't copy: select the words above instead."}
          </span>
        )}
      </div>
      {detail && (
        <details className="text-meta text-ink-2" data-testid="switch-details">
          <summary className="cursor-pointer select-none outline-none focus-visible:ring-2 focus-visible:ring-accent/60">What Copy details copies</summary>
          <p className="mt-1 select-text break-words rounded-lg bg-surface-2 px-2.5 py-1.5 font-mono text-label text-ink">{detail}</p>
        </details>
      )}
      {/* What was set up before the switch failed stays undoable. */}
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}

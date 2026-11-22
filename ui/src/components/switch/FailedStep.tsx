/**
 * A switch that didn't complete (0.3.9): a short title, then the
 * failure's own words (what happened and what to do next, said once:
 * `SwitchFailure::words`), and Copy details: what the program said,
 * where it stopped and when, for a bug report, shown before it's copied
 * (the home folder written `~`), and Report a bug… (AJ-E2911) with the
 * switch's steps. Retry shows only when
 * trying again can succeed (`retryable`).
 *
 * 0.4.9: a refusal that left everything as it was is `calm`: its own
 * title in ink and its words as an info Alert, never amber (the switch
 * screen's Move to account). 0.5.0: anything else is a broken Alert.
 */
import { useState, type JSX } from "react";

import { Alert } from "../Alert";
import { SwitchSetupResults } from "../SwitchSetupList";
import { Disclosure } from "../controls";
import { IconCopy } from "../icons-actions";
import { IconHelp } from "../icons-menu";
import { Button } from "../primitives";
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

export function FailedStep({
  target,
  message,
  cause,
  results,
  calm,
}: {
  target: string;
  message: string;
  cause: unknown;
  results: AppliedInfo[];
  /** Nothing changed: said calmly, under this title. */
  calm?: { title: string };
}): JSX.Element {
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
    <div className="flex flex-col gap-5" data-testid="switch-failed" data-calm={calm ? "true" : undefined}>
      {/* 0.4.9 / ALR-1: a refusal that changed nothing is a fact (info);
          a switch that broke part way is broken, said at once. */}
      <h3 className="type-section">{calm ? calm.title : "The switch didn't complete"}</h3>
      <Alert level={calm ? "info" : "broken"} className="text-body">
        <span data-testid="switch-failed-words">{message}</span>
      </Alert>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary" type="button" onClick={copy} data-testid="switch-copy-details">
          <IconCopy aria-hidden="true" width={14} height={14} />
          Copy details
        </Button>
        {/* AJ-E2911: the same facts and the switch's steps, shown first. */}
        <Button size="sm" variant="secondary" type="button" onClick={() => openReportBug({ from: "switch", words: message, detail, atMs: Date.now() })} data-testid="switch-report-bug">
          <IconHelp aria-hidden="true" width={14} height={14} />
          Report a bug…
        </Button>
        {copied && (
          <span className="text-meta text-ink-2" role="status">
            {copied === "yes" ? "Copied. Paste it into a new issue at github.com/figurehead-ai/AI-juggler/issues." : "Couldn't copy: select the words above instead."}
          </span>
        )}
      </div>
      {detail && (
        <Disclosure className="text-meta text-ink-2" summary="What Copy details copies" testId="switch-details">
          <p className="mt-1 select-text break-words rounded-lg bg-surface-2 px-2.5 py-1.5 font-mono text-label text-ink">{detail}</p>
        </Disclosure>
      )}
      {/* What was set up before the switch failed stays undoable. */}
      <SwitchSetupResults target={target} results={results} />
    </div>
  );
}

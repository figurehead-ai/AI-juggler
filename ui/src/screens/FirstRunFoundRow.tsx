/**
 * One setup on first run's found step (and Accounts › Add account's):
 * its tick, name, how sure AI Juggler is (AJ-E2906: Found, Likely or
 * Guessed, and where it came from), when it was used, and where it is in
 * bringing in. ▸ Details lists each field with its own word and source,
 * and the scan's notes, verbatim.
 */
import { useId, useState, type JSX } from "react";
import type { DiscoveredInfo } from "../lib/bridge";
import { Checkbox } from "../components/controls";
import { Check } from "../components/StepCheck";
import { confidenceWord, fieldRows, mayHaveExpired, rowConfidence, sourceWords } from "./first-run-confidence";

export type RowProgress = { state: "pending" | "working" | "done" | "already" | "failed"; error?: string } | undefined;

export function FoundRow({
  found,
  name,
  used,
  live,
  toolName,
  colour,
  first,
  checked,
  disabled,
  progress,
  onCheck,
}: {
  found: DiscoveredInfo;
  name: string;
  used: string;
  /** Open or in use now (the used words show live). */
  live: boolean;
  toolName: string;
  colour: string;
  first: boolean;
  checked: boolean;
  disabled: boolean;
  progress: RowProgress;
  onCheck: (checked: boolean) => void;
}): JSX.Element {
  const id = `first-run-${found.path}`;
  const detailsId = useId();
  const [open, setOpen] = useState(false);
  const confidence = rowConfidence(found);
  const source = sourceWords(found, toolName);
  return (
    <div className={(first ? "" : "border-t border-hairline/60 ") + (found.managed ? "opacity-60" : "hover:bg-surface-2/60")} data-testid="first-run-row" data-confidence={confidence}>
      <div className="flex items-center gap-2 pr-3">
        <label htmlFor={id} className={"flex min-w-0 flex-1 items-center gap-4 py-3 pl-5 " + (found.managed ? "" : "cursor-pointer")}>
          <Checkbox id={id} checked={checked} disabled={found.managed || disabled} onChange={(event) => onCheck(event.target.checked)} className="shrink-0" />
          <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: colour }} />
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="flex min-w-0 items-baseline gap-2">
              <span className="truncate text-section font-medium text-ink">{name}</span>
              <span
                className={"shrink-0 rounded-md border px-1.5 text-label " + (confidence === "guessed" ? "border-warning/40 text-warning" : "border-hairline text-ink-2")}
                data-testid="first-run-confidence"
              >
                {confidenceWord(confidence)}
              </span>
              {/* That it can be renamed is said once, in the footer (the 0.4 design review, 53). */}
            </span>
            <span className="truncate text-label text-muted" title={`${found.path} · ${source}`}>
              <span className="font-mono">{found.path.replace(/^\/Users\/[^/]+/, "~")}</span>
              <span data-testid="first-run-source"> · {source}</span>
            </span>
          </span>
          <span className={"shrink-0 text-xs " + (live ? "text-live" : "text-muted")}>{used}</span>
          <span className="flex w-32 shrink-0 items-center justify-end gap-1.5 text-xs">
            {progress?.state === "working" ? (
              <span className="text-muted">Bringing in…</span>
            ) : progress?.state === "done" ? (
              <span className="flex items-center gap-1.5 text-success"><Check />Brought in</span>
            ) : progress?.state === "already" ? (
              <span className="text-muted" title={progress.error}>Already in AI Juggler</span>
            ) : progress?.state === "failed" ? (
              <span className="text-danger">Couldn't bring in</span>
            ) : found.managed ? (
              <span className="text-muted">Already in AI Juggler</span>
            ) : found.login === "logged_in" ? (
              <span className="flex items-center gap-1.5 text-success"><Check />Signed in</span>
            ) : mayHaveExpired(found) ? (
              <span className="text-warning">May have expired</span>
            ) : (
              <span className="text-muted">Needs sign-in</span>
            )}
          </span>
        </label>
        <button
          type="button"
          aria-expanded={open}
          aria-controls={detailsId}
          aria-label={`Details for ${name}`}
          onClick={() => setOpen((value) => !value)}
          className="flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 text-label text-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent"
          data-testid="first-run-details-toggle"
        >
          <span aria-hidden="true" className={"inline-block transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}>▸</span>
          Details
        </button>
      </div>
      {open && (
        <div id={detailsId} className="flex flex-col gap-2 pb-3 pl-[4.25rem] pr-5 text-label" data-testid="first-run-details">
          <dl className="grid grid-cols-[auto_auto_auto_1fr] items-baseline gap-x-3 gap-y-1">
            {fieldRows(found, { name, used }).map((row) => (
              <div key={row.field} className="contents" data-testid="first-run-field">
                <dt className="text-muted">{row.field}</dt>
                <dd className="text-ink">{row.value}</dd>
                <dd className={row.confidence === "guessed" ? "text-warning" : "text-ink-2"}>{confidenceWord(row.confidence)}</dd>
                <dd className="text-muted">from {row.source}</dd>
              </div>
            ))}
          </dl>
          {found.notes.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-muted" data-testid="first-run-notes">
              {found.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * AJ-E3013 (D-116): what Add writes, before it does — each file in a
 * line ("Creates ~/…/plugin.json"), its exact text in a fold under it —
 * and what Remove deletes, each file in a line, the ones someone changed
 * named as staying. The consent card's diff; nothing is written here.
 */
import type { JSX } from "react";

import { FoldChevron } from "../../../components/this-mac-memory";
import { fileLine, keptLine, removeLine } from "../../../lib/connector";
import type { ConnectorFileInfo } from "../../../lib/ipc/connector";

/** Each file Add would write, with its text. */
export function ConnectorFiles({ files, testId }: { files: readonly ConnectorFileInfo[]; testId: string }): JSX.Element {
  return (
    <ul className="flex flex-col gap-1.5" data-testid={`${testId}-files`}>
      {files.map((file) => (
        <li key={file.path} className="flex flex-col gap-0.5" data-action={file.action} data-testid={`${testId}-file`}>
          <span className={"text-meta [overflow-wrap:anywhere] " + (file.action === "refuse" ? "text-warning" : "text-ink")}>{fileLine(file.action, file.path)}</span>
          {file.action !== "same" && file.action !== "refuse" && (
            <details className="group text-meta">
              <summary className="inline-flex min-h-6 cursor-pointer list-none items-center gap-1 rounded text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden" data-testid={`${testId}-file-show`}>
                <FoldChevron />
                {file.before === null ? "Its text" : "Its text, after"}
              </summary>
              {/* Its whole text, wrapped (no scrolling box of its own). */}
              <pre className="mt-1 whitespace-pre-wrap break-words rounded-lg bg-surface px-3 py-2 font-mono text-label text-ink" data-testid={`${testId}-file-text`}>
                {file.after}
              </pre>
            </details>
          )}
        </li>
      ))}
    </ul>
  );
}

/** What Remove deletes, and what it leaves because someone changed it. */
export function RemoveFiles({ files, changed, testId }: { files: readonly string[]; changed: readonly string[]; testId: string }): JSX.Element {
  return (
    <ul className="flex flex-col gap-1" data-testid={`${testId}-remove-files`}>
      {files.map((path) => (
        <li key={path} className={"text-meta [overflow-wrap:anywhere] " + (changed.includes(path) ? "text-warning" : "text-ink")} data-testid={`${testId}-remove-file`}>
          {changed.includes(path) ? keptLine(path) : removeLine(path)}
        </li>
      ))}
    </ul>
  );
}

/**
 * AJ-E3013 (D-116): what Add writes, before it does — each file in a
 * line ("Creates ~/…/plugin.json"), its exact text in a fold under it —
 * and what Remove deletes, each file in a line, the ones someone changed
 * named as staying. The consent card's diff; nothing is written here.
 */
import type { JSX } from "react";

import { Disclosure } from "../../../components/controls";
import { fileLine, keptLine, removeLine, shortPath } from "../../../lib/connector";
import type { ConnectorFileInfo } from "../../../lib/ipc/connector";

/** Each file Add would write, with its text. */
export function ConnectorFiles({ files, testId }: { files: readonly ConnectorFileInfo[]; testId: string }): JSX.Element {
  return (
    <ul className="flex flex-col gap-1.5" data-testid={`${testId}-files`}>
      {files.map((file) => (
        <li key={file.path} className="flex flex-col gap-0.5" data-action={file.action} data-testid={`${testId}-file`}>
          <span className={"text-meta [overflow-wrap:anywhere] " + (file.action === "refuse" ? "text-warning" : "text-ink")}>{fileLine(file.action, file.path)}</span>
          {file.action !== "same" && file.action !== "refuse" && (
            // Round 7: the app's Disclosure (a button), which an
            // accessibility press opens; a flex <summary> wasn't one.
            <Disclosure tone="link" summary={file.before === null ? "Its text" : "Its text, after"} testId={`${testId}-file-show`}>
              {/* Its whole text, wrapped (no scrolling box of its own). */}
              <pre className="mt-1 whitespace-pre-wrap break-words rounded-lg bg-surface px-3 py-2 font-mono text-label text-ink" data-testid={`${testId}-file-text`}>
                {file.after}
              </pre>
            </Disclosure>
          )}
        </li>
      ))}
    </ul>
  );
}

/** What Remove deletes ("Deletes ~/…"); or, `anyway`, what Remove anyway
 * would delete, each by its path, the changed one said so. */
export function RemoveFiles({ files, changed, anyway = false, testId }: { files: readonly string[]; changed: readonly string[]; anyway?: boolean; testId: string }): JSX.Element {
  const line = (path: string) => (changed.includes(path) ? keptLine(path) : anyway ? shortPath(path) : removeLine(path));
  return (
    <ul className={"flex flex-col gap-1" + (anyway ? " list-disc pl-5" : "")} data-testid={`${testId}-remove-files`}>
      {files.map((path) => (
        <li key={path} className={"text-meta [overflow-wrap:anywhere] " + (changed.includes(path) ? "text-warning" : "text-ink")} data-testid={`${testId}-remove-file`}>
          {line(path)}
        </li>
      ))}
    </ul>
  );
}

/**
 * A folder path inside a sentence, in the monospace face, that breaks
 * only after a "/" (a `<wbr>` there), never inside a folder's name: the
 * 0.4.6 RC walk saw "…/ClaudeProfi" + "les" on Claude app · Work Max's
 * "Find this app's other profiles". A path on a line of its own can keep
 * `break-all`; this is for prose.
 */
import { Fragment, type JSX } from "react";

export function PathText({ path, className = "" }: { path: string; className?: string }): JSX.Element {
  const parts = path.split("/");
  return (
    <span className={`font-mono [overflow-wrap:anywhere] ${className}`} data-path={path}>
      {parts.map((part, at) => (
        <Fragment key={at}>
          {part}
          {at < parts.length - 1 && (
            <>
              /<wbr />
            </>
          )}
        </Fragment>
      ))}
    </span>
  );
}

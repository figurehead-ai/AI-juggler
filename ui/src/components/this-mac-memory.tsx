/**
 * This Mac's memory as a whole (the 0.1.4 review: "Under memory pressure"
 * beside "AI tools: about 16 GB of 103 GB" read as plenty left). A gauge
 * whose parts are named (AI tools, other apps and macOS, kept compressed,
 * available), memory pressure explained in words whenever macOS reports
 * it, and on the This Mac page the biggest other apps, by name only,
 * folded and read-only (D-08). Home's panel and the page draw the gauge
 * the same way (D-25).
 *
 * Readability (the maintainer's bar): body 14 px, meta 13 px, nothing
 * under 12 px; secondary words in ink at 80 % (7:1 or better on every
 * surface, both themes); figures in tabular numbers.
 */
import type { JSX } from "react";
import { memoryBasisWords } from "./this-mac-parts";
import { bytesWords, type ThisMacInfo } from "../lib/machine";
import { memoryParts, otherAppBytes, PRESSURE_MORE, PRESSURE_SHORT, underPressure } from "../lib/machine-memory";

/** The rail's fold chevron: one look for every disclosure. */
export function FoldChevron({ group = "group" }: { group?: "group" | "group/more" }): JSX.Element {
  return (
    <svg
      aria-hidden="true"
      width="12"
      height="12"
      viewBox="0 0 10 10"
      className={`shrink-0 text-muted transition-transform motion-reduce:transition-none ${group === "group" ? "group-open:rotate-90" : "group-open/more:rotate-90"}`}
    >
      <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** Each part's look: a colour and, for compressed memory, stripes, so no
 * part is told by colour alone. */
const PART_LOOK = {
  tools: "bg-accent",
  other: "bg-faint",
  compressed: "bg-[repeating-linear-gradient(135deg,var(--color-faint)_0_2px,transparent_2px_5px)] ring-1 ring-inset ring-faint",
  available: "bg-surface-3 ring-1 ring-inset ring-hairline",
} as const;
type PartKey = keyof typeof PART_LOOK;

/** Memory pressure in plain words: one line, and on the page the rest
 * behind a disclosure. Said whenever macOS reports pressure, so a free
 * figure never stands against it unexplained. */
export function PressureNote({ info, more }: { info: ThisMacInfo; more: boolean }): JSX.Element | null {
  if (!underPressure(info.machine)) return null;
  return (
    <div className="flex flex-col gap-1 text-body text-muted" data-testid="this-mac-pressure-note">
      <p>{PRESSURE_SHORT}</p>
      {more && (
        <details className="group/more">
          <summary className="flex w-fit cursor-pointer list-none items-center gap-1.5 rounded-md text-meta text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            <FoldChevron group="group/more" />
            What it means
          </summary>
          <p className="mt-1 max-w-prose pl-[18px]">{PRESSURE_MORE}</p>
        </details>
      )}
    </div>
  );
}

/** Memory: a gauge of the whole Mac's, in named parts, with how much is in
 * use as a figure; pressure in words beside it. `more` (the page) adds the
 * explanation's disclosure. */
export function MemoryMeter({ info, more = false }: { info: ThisMacInfo; more?: boolean }): JSX.Element | null {
  const machine = info.machine;
  const { total, used, tools, compressed, other, available } = memoryParts(info);
  if (total <= 0) return null;
  const basis = memoryBasisWords(machine.memory_basis);
  const running = info.tools.count > 0;
  const about = info.tools.memory_estimate ? "about " : "";
  const share = (bytes: number) => `${Math.min(100, (bytes / total) * 100)}%`;
  const parts = [
    running ? { key: "tools", words: `AI tools ${about}${bytesWords(tools)}`, bytes: tools } : null,
    used !== null ? { key: "other", words: `${running ? "Other apps and macOS" : "Apps and macOS"} ${bytesWords(other)}`, bytes: other } : null,
    used !== null && compressed > 0 ? { key: "compressed", words: `Kept compressed ${bytesWords(compressed)}`, bytes: compressed } : null,
    used !== null ? { key: "available", words: `Available ${bytesWords(available)}`, bytes: available } : null,
  ].filter((part): part is { key: PartKey; words: string; bytes: number } => part !== null);
  return (
    <div className="flex flex-col gap-2.5">
      <p className="flex flex-wrap items-baseline gap-x-1.5 text-body text-muted" data-testid="this-mac-memory" title={used !== null ? `In use: ${basis}` : undefined}>
        {used !== null ? (
          <>
            <span className="text-title font-semibold tabular-nums text-ink">{bytesWords(used)}</span>{" "}
            <span className="tabular-nums">in use of {bytesWords(total)}</span>
          </>
        ) : (
          <span className="tabular-nums">
            Memory: {bytesWords(total)}
            {running ? ` · AI tools ${about}${bytesWords(tools)}` : ""}
          </span>
        )}
      </p>
      {used !== null && (
        <>
          <div
            className="flex h-3 gap-px overflow-hidden rounded-full bg-surface-3"
            role="meter"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(Math.min(100, (used / total) * 100))}
            aria-label={`Memory: ${bytesWords(used)} of ${bytesWords(total)} in use (${basis}); ${parts.map((part) => part.words).join(", ")}`}
            title={`In use: ${basis}`}
          >
            {/* The tools' share keeps its own colour: the Mac's state is
                the status line's to say, not a blame on this part. */}
            {parts
              .filter((part) => part.key !== "available" && part.bytes > 0)
              .map((part) => (
                <div key={part.key} className={`h-full ${PART_LOOK[part.key]}`} style={{ width: share(part.bytes) }} title={part.words} />
              ))}
          </div>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(12rem,1fr))] gap-x-4 gap-y-1 text-meta tabular-nums text-ink" aria-label="Memory by part" data-testid="this-mac-memory-parts">
            {parts.map((part) => (
              <li key={part.key} className="flex items-center gap-2" data-part={part.key}>
                <span aria-hidden="true" className={`size-3 shrink-0 rounded-sm ${PART_LOOK[part.key]}`} />
                {part.words}
              </li>
            ))}
          </ul>
          {more && compressed > 0 && (
            // The 0.2 walk (F13): an alert named one app at more than
            // "Other apps and macOS" here.
            <p className="text-meta text-muted" data-testid="this-mac-memory-compressed-note">
              Memory macOS keeps compressed is shown on its own, and an app's own figure counts memory its processes share more than once, so one app can show more than “Other apps and macOS” here.
            </p>
          )}
        </>
      )}
      <PressureNote info={info} more={more} />
    </div>
  );
}

/** The This Mac page's other apps (the 0.1.4 review): what isn't an AI
 * tool, folded; the biggest by name only (never a path or an argument),
 * and nothing to click (D-08). */
export function OtherAppsGroup({ info }: { info: ThisMacInfo }): JSX.Element | null {
  const apps = info.other_apps;
  const { used, other, compressed } = memoryParts(info);
  if (apps.length === 0 && used === null) return null;
  const figure = used !== null ? bytesWords(other) : `about ${bytesWords(apps.reduce((sum, app) => sum + app.rss_bytes, 0))}`;
  return (
    <details className="group border-t border-hairline/60 first:border-t-0" data-testid="this-mac-other-apps">
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-3 gap-y-1 rounded-lg py-3 outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
        <FoldChevron />
        <span className="flex min-w-0 flex-1 basis-40 items-baseline gap-2 text-body font-medium text-ink">
          <span className="truncate">{info.tools.count > 0 ? "Other apps and macOS" : "Apps and macOS"}</span>
          <span className="shrink-0 text-meta font-normal text-muted">not AI tools</span>
        </span>
        <span className="ml-auto shrink-0 text-body tabular-nums text-ink">{figure}</span>
      </summary>
      <div className="mb-3 ml-[22px] flex flex-col gap-2">
        {apps.length > 0 && (
          <ul className="flex flex-col" aria-label="Other apps, biggest first">
            {apps.map((app) => (
              <li key={app.name} className="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-hairline/60 py-2 first:border-t-0" data-testid="this-mac-other-app">
                <span className="min-w-0 flex-1 basis-28 truncate text-body text-ink">{app.name}</span>
                <span className="ml-auto flex shrink-0 flex-col items-end gap-0.5 tabular-nums">
                  <span className="text-body text-ink">{otherAppBytes(app)}</span>
                  {app.processes > 1 && <span className="text-meta text-muted">{app.processes} processes</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="text-meta text-muted">
          The biggest, by name. The rest is macOS itself and smaller apps
          {compressed > 0 ? `, and macOS keeps ${bytesWords(compressed)} more compressed for every app, AI tools too` : ""}. AI Juggler only reads
          these, and never stops them.
        </p>
      </div>
    </details>
  );
}

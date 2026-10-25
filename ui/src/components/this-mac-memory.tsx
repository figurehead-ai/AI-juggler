/**
 * This Mac's memory as a whole (the 0.1.4 review: "Under memory pressure"
 * beside "AI tools: about 16 GB of 103 GB" read as plenty left). The bar's
 * parts are named (AI tools, other apps and macOS, available), what macOS
 * keeps compressed is said, memory pressure is explained beside the
 * figures whenever macOS reports it, and the This Mac page lists the
 * biggest other apps, by name only, folded and read-only (D-08). Home's
 * panel and the page draw the meter the same way (D-25).
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
      width="10"
      height="10"
      viewBox="0 0 10 10"
      className={`shrink-0 text-faint transition-transform motion-reduce:transition-none ${group === "group" ? "group-open:rotate-90" : "group-open/more:rotate-90"}`}
    >
      <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** The meter's parts, in its order: each with its look and its words. */
const PART_LOOK = {
  tools: "bg-accent",
  other: "bg-faint",
  available: "border border-hairline bg-surface-3",
} as const;

/** Memory pressure in plain words: one line, and on the page the rest
 * behind a disclosure. Said whenever macOS reports pressure, so a free
 * figure never stands against it unexplained. */
export function PressureNote({ info, more }: { info: ThisMacInfo; more: boolean }): JSX.Element | null {
  if (!underPressure(info.machine)) return null;
  return (
    <div className="flex flex-col gap-1 text-[11px] text-muted" data-testid="this-mac-pressure-note">
      <p>{PRESSURE_SHORT}</p>
      {more && (
        <details className="group/more">
          <summary className="flex w-fit cursor-pointer list-none items-center gap-1.5 rounded-md outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
            <FoldChevron group="group/more" />
            What it means
          </summary>
          <p className="mt-1 max-w-prose pl-4">{PRESSURE_MORE}</p>
        </details>
      )}
    </div>
  );
}

/** Memory: the whole Mac's, in named parts, with what macOS keeps
 * compressed; the AI tools' share of CPU. `more` (the page) adds the
 * pressure explanation's disclosure. */
export function MemoryMeter({ info, more = false }: { info: ThisMacInfo; more?: boolean }): JSX.Element | null {
  const machine = info.machine;
  const { total, used, tools, other, available } = memoryParts(info);
  if (total <= 0) return null;
  const basis = memoryBasisWords(machine.memory_basis);
  const running = info.tools.count > 0;
  const about = info.tools.memory_estimate ? "about " : "";
  const share = (bytes: number) => `${Math.min(100, (bytes / total) * 100)}%`;
  const parts = [
    running ? { key: "tools", words: `AI tools ${about}${bytesWords(tools)}`, bytes: tools } : null,
    used !== null ? { key: "other", words: `${running ? "Other apps and macOS" : "Apps and macOS"} ${bytesWords(other)}`, bytes: other } : null,
    used !== null ? { key: "available", words: `Available ${bytesWords(available)}`, bytes: available } : null,
  ].filter((part): part is { key: keyof typeof PART_LOOK; words: string; bytes: number } => part !== null);
  const compressed = used !== null && machine.memory_compressed ? `${bytesWords(machine.memory_compressed)} kept compressed` : null;
  const line = [
    used !== null ? `In use: ${bytesWords(used)} of ${bytesWords(total)}` : `Memory: ${bytesWords(total)}`,
    compressed,
    used === null && running ? `AI tools ${about}${bytesWords(tools)}` : null,
    running ? `AI tools: ${info.tools.cpu_pct === null ? "CPU —" : `${Math.round(info.tools.cpu_pct)}% of all CPU cores`}` : null,
  ].filter(Boolean);
  return (
    <div className="flex flex-col gap-1.5">
      {used !== null && (
        <>
          <div
            className="flex h-1.5 overflow-hidden rounded-full bg-surface-3"
            role="meter"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(Math.min(100, (used / total) * 100))}
            aria-label={`Memory: ${bytesWords(used)} of ${bytesWords(total)} in use (${basis}); ${parts.map((part) => part.words).join(", ")}`}
            title={`In use: ${basis}`}
          >
            {/* The tools' share keeps its own colour: the Mac's state is
                the status line's to say, not a blame on this segment. */}
            {parts
              .filter((part) => part.key !== "available")
              .map((part) => (
                <div key={part.key} className={`h-full ${PART_LOOK[part.key]}`} style={{ width: share(part.bytes) }} title={part.words} />
              ))}
          </div>
          <ul className="flex flex-wrap gap-x-3 gap-y-0.5 text-[11px] text-muted" aria-label="Memory by part" data-testid="this-mac-memory-parts">
            {parts.map((part) => (
              <li key={part.key} className="flex items-center gap-1.5" data-part={part.key}>
                <span aria-hidden="true" className={`size-2 shrink-0 rounded-sm ${PART_LOOK[part.key]}`} />
                {part.words}
              </li>
            ))}
          </ul>
        </>
      )}
      <span className="text-[11px] text-muted" data-testid="this-mac-memory" title={used !== null ? `In use: ${basis}` : undefined}>
        {line.join(" · ")}
      </span>
      <PressureNote info={info} more={more} />
    </div>
  );
}

/** The This Mac page's other apps (the 0.1.4 review): what isn't an AI
 * tool, folded; the biggest by name only (never a path or an argument),
 * and nothing to click (D-08). */
export function OtherAppsGroup({ info }: { info: ThisMacInfo }): JSX.Element | null {
  const apps = info.other_apps;
  const { used, other } = memoryParts(info);
  if (apps.length === 0 && used === null) return null;
  const figure = used !== null ? bytesWords(other) : `about ${bytesWords(apps.reduce((sum, app) => sum + app.rss_bytes, 0))}`;
  return (
    <details className="group border-t border-hairline/60 first:border-t-0" data-testid="this-mac-other-apps">
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-3 gap-y-1 rounded-lg py-2.5 outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
        <FoldChevron />
        <span className="flex min-w-0 flex-1 basis-40 items-center gap-2 text-sm font-medium text-ink">
          <span className="truncate">{info.tools.count > 0 ? "Other apps and macOS" : "Apps and macOS"}</span>
          <span className="shrink-0 text-[11px] font-normal text-muted">not AI tools</span>
        </span>
        <span className="ml-auto shrink-0 font-mono text-xs text-ink">{figure}</span>
      </summary>
      <div className="mb-2 ml-5 flex flex-col gap-1.5">
        {apps.length > 0 && (
          <ul className="flex flex-col" aria-label="Other apps, biggest first">
            {apps.map((app) => (
              <li key={app.name} className="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-hairline/60 py-2 first:border-t-0" data-testid="this-mac-other-app">
                <span className="min-w-0 flex-1 basis-28 truncate text-sm text-ink">{app.name}</span>
                <span className="ml-auto flex shrink-0 flex-col items-end gap-0.5 font-mono text-xs">
                  <span className="text-ink">{otherAppBytes(app)}</span>
                  {app.processes > 1 && <span className="text-faint">{app.processes} processes</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="text-[11px] text-muted">
          The biggest, by name. The rest is macOS itself, memory it keeps compressed and smaller apps. AI Juggler only reads these, and never stops them.
        </p>
      </div>
    </details>
  );
}

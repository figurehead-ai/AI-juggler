/**
 * E14 / AJ-E1404 — Home's "This Mac": how the Mac is doing in words, what
 * your AI tools use right now, the heaviest of them, and anything left
 * running after its session closed.
 *
 * D-25 calm: colour only when the Mac is not "Plenty of room", and every
 * state is also a word and a shape. A tool AI Juggler didn't start (the
 * Claude app, `claude` in a terminal) is shown and counted, labelled as
 * such, and offers nothing to click (D-08: unowned is untouchable).
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { MachineState, MemoryMeter, TreeRow } from "./this-mac-parts";
import { machineStatus, type ThisMacInfo } from "../lib/machine";


export function ThisMacPanel({
  info,
  nameOf,
  onOpenSession,
  onOpenCleanup,
  onOpenPage,
}: {
  info: ThisMacInfo | null;
  /** A session's name on screen (its place: branch, else folder). */
  nameOf: (sessionId: string) => string;
  onOpenSession: (id: string) => void;
  onOpenCleanup?: () => void;
  /** AJ-E1528: the This Mac page, where all of this is in full. */
  onOpenPage?: () => void;
}): JSX.Element | null {
  const [showAll, setShowAll] = useState(false);
  // Nothing to say until the first sample lands.
  const level = machineStatus(info);
  if (!info || !level) return null;
  const machine = info.machine;
  return (
    <section
      id="this-mac"
      tabIndex={-1}
      aria-labelledby="home-this-mac"
      className="flex scroll-mt-4 flex-col gap-2.5 outline-none"
      data-testid="home-this-mac"
      data-level={machine.summary.level}
    >
      <div className="flex items-baseline justify-between gap-3">
        <h2 id="home-this-mac" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
          This Mac
        </h2>
        {onOpenPage && (
          <button type="button" className="text-xs text-muted hover:text-ink hover:underline" onClick={onOpenPage} data-testid="this-mac-open-page">
            See all <span className="sr-only">of This Mac</span>
          </button>
        )}
      </div>
      <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface px-4 py-3.5">
        <MachineState info={info} />

        <MemoryMeter info={info} />

        {info.top.length > 0 && (
          <ul className="flex flex-col" aria-label="Heaviest right now" data-testid="this-mac-top">
            {[...info.top, ...(showAll ? info.others : [])].map((tree) => (
              <TreeRow
                key={tree.root_pid !== null ? `pid-${tree.root_pid}` : (tree.session_id ?? `${tree.place}-${tree.account_label}-${tree.rss_bytes}`)}
                tree={tree}
                nameOf={nameOf}
                onOpenSession={onOpenSession}
              />
            ))}
          </ul>
        )}
        {/* Never a row dropped without a word (QA, P6): the rest are one
            click away. */}
        {info.others.length > 0 && (
          <button
            type="button"
            className="self-start text-xs text-muted underline-offset-2 hover:text-ink hover:underline"
            aria-expanded={showAll}
            onClick={() => setShowAll((open) => !open)}
            data-testid="this-mac-more"
          >
            {showAll ? "Show fewer" : `and ${info.others.length} more`}
          </button>
        )}
        {info.theirs.count > 0 && info.top.every((tree) => tree.session_id !== null) && (
          <p className="text-[11px] text-muted">
            Also counted: {info.theirs.count === 1 ? "1 Claude tool" : `${info.theirs.count} Claude tools`} not started by AI Juggler.
          </p>
        )}

        {info.recommendations.length > 0 && (
          <ul className="flex flex-col gap-1 border-t border-hairline/60 pt-3 text-xs text-muted" aria-label="From your history" data-testid="this-mac-recommendations">
            {info.recommendations.map((line) => (
              <li key={line}>{line}.</li>
            ))}
          </ul>
        )}

        {info.leftover_processes > 0 && (
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-t border-hairline/60 pt-3" data-testid="this-mac-leftovers">
            <span className="min-w-0 flex-1 basis-40 text-xs text-muted">
              {info.leftover_processes === 1 ? "1 process" : `${info.leftover_processes} processes`} left running after
              {info.leftover_processes === 1 ? " its" : " their"} session closed
              {info.leftover_ports > 0 ? ` · ${info.leftover_ports === 1 ? "holding a port" : `holding ${info.leftover_ports} ports`}` : ""}
            </span>
            {onOpenCleanup && (
              <Button size="sm" variant="secondary" onClick={onOpenCleanup}>
                Clean up
              </Button>
            )}
          </div>
        )}
      </div>
    </section>
  );
}

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
import { ToolMark } from "./ToolMark";
import { useItemMenu } from "./ItemMenu";
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { bytesWords, machineStatus, reasonSentence, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";


/** Why a tool AI Juggler didn't start shows "about": its helpers share
 * memory, and the sum counts that once per helper. */
const ESTIMATE_WORDS =
  "An estimate: the sum of each helper's memory, so memory they share is counted more than once";

/** What "in use" counts, in words (the backend's `memory_basis`). */
export function memoryBasisWords(basis: string | null): string {
  return basis === "app_wired_compressed"
    ? "app memory, wired and compressed — close to Activity Monitor's Memory Used"
    : basis === "total_minus_available"
      ? "everything the system doesn't list as available"
      : "not measured";
}

/** A tool AI Juggler didn't start, in words. */
export function placeWords(place: string | null): string {
  return place === "desktop" ? "Claude app" : place === "terminal" ? "claude in a terminal" : "Claude";
}

/** "CPU 12%" (of one core, as Activity Monitor counts it) or "CPU —". */
function cpuWords(cpu: number | null): string {
  return cpu === null ? "CPU —" : `CPU ${Math.round(cpu)}%`;
}

function TreeRow({
  tree,
  nameOf,
  onOpenSession,
}: {
  tree: ToolTreeInfo;
  nameOf: (sessionId: string) => string;
  onOpenSession: (id: string) => void;
}): JSX.Element {
  const ours = tree.session_id !== null;
  const title = ours ? nameOf(tree.session_id!) : placeWords(tree.place);
  const detail = [tree.account_label, ours ? null : "not started by AI Juggler"].filter(Boolean).join(" · ");
  const numbers = (
    <span className="ml-auto flex shrink-0 flex-col items-end gap-0.5 font-mono text-xs">
      <span
        className={tree.heavy_note ? "font-semibold text-warning" : "text-ink"}
        title={tree.memory_estimate ? ESTIMATE_WORDS : undefined}
      >
        {tree.memory_estimate ? `about ${bytesWords(tree.rss_bytes)}` : bytesWords(tree.rss_bytes)}
      </span>
      <span className="text-faint" title="As Activity Monitor counts it: 100% is one core busy">
        {cpuWords(tree.cpu_pct)}
      </span>
    </span>
  );
  const body = (
    <>
      <span className="flex min-w-0 flex-1 basis-28 flex-col gap-0.5">
        <span className="flex min-w-0 items-center gap-1.5 text-sm text-ink">
          {tree.tool && <ToolMark tool={tree.tool} size={12} className="text-muted" />}
          <span className="truncate">{title}</span>
          {tree.heavy_note && <span className="sr-only"> — heavy: {tree.heavy_note}</span>}
        </span>
        {detail && <span className="truncate text-[11px] text-muted">{detail}</span>}
      </span>
      {numbers}
    </>
  );
  // AJ-E1537: right-click, ⋯ or Shift+F10 — go to the session, or say
  // it isn't AI Juggler's.
  const menu = useItemMenu("this-mac", tree, title, { openSessionById: onOpenSession });
  return (
    <li className="group relative border-t border-hairline/60 first:border-t-0" data-testid="this-mac-tree" data-ours={ours ? "true" : "false"} {...menu.triggerProps}>
      {ours ? (
        <button type="button" className="-mx-2 flex w-[calc(100%+1rem)] flex-wrap items-center gap-x-3 gap-y-1 rounded-lg px-2 py-2 text-left hover:bg-surface-2" onClick={() => onOpenSession(tree.session_id!)}>
          {body}
        </button>
      ) : (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 py-2">{body}</div>
      )}
      {/* D-08: a tool AI Juggler didn't start has no controls at all. */}
      {ours && menu.button({ label: `Actions for ${title}`, className: "absolute -left-6 top-1/2 -translate-y-1/2" })}
      {menu.element}
    </li>
  );
}

export function ThisMacPanel({
  info,
  nameOf,
  onOpenSession,
  onOpenCleanup,
}: {
  info: ThisMacInfo | null;
  /** A session's name on screen (its place: branch, else folder). */
  nameOf: (sessionId: string) => string;
  onOpenSession: (id: string) => void;
  onOpenCleanup?: () => void;
}): JSX.Element | null {
  const [showAll, setShowAll] = useState(false);
  // Nothing to say until the first sample lands.
  const level = machineStatus(info);
  if (!info || !level) return null;
  const machine = info.machine;
  const calm = level.calm;
  const total = machine.memory_total;
  const toolsShare = total > 0 ? Math.min(100, (info.tools.rss_bytes / total) * 100) : 0;
  // Memory in use only when it was read, on its stated basis — never a
  // different measure under the same name.
  const used = machine.memory_used;
  const basis = memoryBasisWords(machine.memory_basis);
  const usedShare = total > 0 && used !== null ? Math.min(100, (used / total) * 100) : 0;
  const otherShare = Math.max(0, usedShare - toolsShare);
  const nothing = info.tools.count === 0;
  const reasons = machine.summary.reasons.slice(0, 2).map((reason) => reason.words);
  return (
    <section
      id="this-mac"
      tabIndex={-1}
      aria-labelledby="home-this-mac"
      className="flex scroll-mt-4 flex-col gap-2.5 outline-none"
      data-testid="home-this-mac"
      data-level={machine.summary.level}
    >
      <h2 id="home-this-mac" className="text-[11px] font-semibold uppercase tracking-wider text-muted">
        This Mac
      </h2>
      <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface px-4 py-3.5">
        <div className="flex flex-col gap-0.5" role="status">
          <span className="flex items-center gap-2 text-sm font-medium text-ink">
            <span aria-hidden="true" className={`text-[11px] ${level.tone}`}>{level.glyph}</span>
            {nothing && calm ? "Nothing running — your Mac has room" : level.words}
          </span>
          {!calm && reasons.length > 0 && (
            <span className="text-xs text-muted" data-testid="this-mac-reasons">
              {reasonSentence(reasons)}
            </span>
          )}
        </div>

        {total > 0 && (
          <div className="flex flex-col gap-1.5">
            {used !== null && (
            <div
              className="flex h-1.5 overflow-hidden rounded-full bg-surface-3"
              role="meter"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(usedShare)}
              aria-label={`Memory: ${bytesWords(used)} of ${bytesWords(total)} in use (${basis}), ${bytesWords(info.tools.rss_bytes)} by AI tools`}
              title={`In use: ${basis}`}
            >
              {/* The tools' share keeps its own colour: the Mac's state is
                  the status line's to say, not a blame on this segment. */}
              <div className="h-full bg-accent" style={{ width: `${toolsShare}%` }} />
              <div className="h-full bg-faint/40" style={{ width: `${otherShare}%` }} />
            </div>
            )}
            <span className="text-[11px] text-muted" data-testid="this-mac-memory" title={used !== null ? `In use: ${basis}` : undefined}>
              {nothing
                ? used !== null
                  ? `Memory: ${bytesWords(used)} of ${bytesWords(total)} in use`
                  : `Memory: ${bytesWords(total)}`
                : `AI tools: ${info.tools.memory_estimate ? "about " : ""}${bytesWords(info.tools.rss_bytes)} of ${bytesWords(total)} memory · ${
                    info.tools.cpu_pct === null ? "CPU —" : `${Math.round(info.tools.cpu_pct)}% of all CPU cores`
                  }`}
            </span>
          </div>
        )}

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

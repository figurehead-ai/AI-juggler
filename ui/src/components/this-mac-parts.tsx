/**
 * E14 — This Mac's shared parts: the Mac's state in words and a process
 * tree's row (the memory meter is in this-mac-memory.tsx). Home's panel (a summary) and the This
 * Mac page (AJ-E1528, the detail) draw them the same way (D-25).
 */
import type { JSX } from "react";
import { ToolMark } from "./ToolMark";
import { useItemMenu } from "./ItemMenu";
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
export function cpuWords(cpu: number | null): string {
  return cpu === null ? "CPU —" : `CPU ${Math.round(cpu)}%`;
}

export function TreeRow({
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


/** The Mac's state in words, with its glyph, and why (when not calm). */
export function MachineState({ info, reasonsShown = 2 }: { info: ThisMacInfo; reasonsShown?: number }): JSX.Element | null {
  const level = machineStatus(info);
  if (!level) return null;
  const calm = level.calm;
  const nothing = info.tools.count === 0;
  const reasons = info.machine.summary.reasons.slice(0, reasonsShown).map((reason) => reason.words);
  return (
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

  );
}

/** AJ-E1509 Req.5: whether AI Juggler keeps the Mac awake for the
 * sessions working now, and why not when it doesn't — with the setting a
 * click away. Said only while sessions work. */
export function AwakeLine({ info }: { info: ThisMacInfo }): JSX.Element | null {
  const { working, kept_awake: kept, on_battery: battery } = info.awake;
  if (working === 0) return null;
  // Only the sessions AI Juggler runs keep the Mac awake: said so, beside
  // a Home that also lists other tools' conversations as running.
  const sessions = working === 1 ? "1 AI Juggler session works" : `${working} AI Juggler sessions work`;
  const words = kept
    ? `Kept awake while ${sessions}`
    : battery
      ? `On battery, so not kept awake while ${sessions}`
      : `Not kept awake while ${sessions} (off in Settings)`;
  return (
    <p className="flex flex-wrap items-center gap-x-2 text-[11px] text-muted" data-testid="this-mac-awake" data-kept={kept ? "true" : "false"}>
      <span aria-hidden="true" className={`size-1.5 rounded-full ${kept ? "bg-live" : "bg-faint"}`} />
      <span>{words}</span>
      <button
        type="button"
        className="text-muted underline-offset-2 hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        onClick={() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "sessions" } }))}
        data-testid="this-mac-awake-settings"
      >
        Change{" "}
        <span className="sr-only">when the Mac is kept awake</span>
      </button>
    </p>
  );
}

/** AJ-E1509: AI Juggler itself — its own memory with its crash
 * watchdog's (a process of AI Juggler, never a stray one), and whether it
 * opens at login. */
export function OwnLine({ info }: { info: ThisMacInfo }): JSX.Element {
  const { rss_bytes: bytes, watchdog, opens_at_login: atLogin } = info.own;
  const parts = [
    `AI Juggler itself: ${bytesWords(bytes)}`,
    watchdog ? "with its crash watchdog" : null,
    atLogin ? "opens at login" : null,
  ].filter(Boolean);
  return (
    <p className="text-[11px] text-muted" data-testid="this-mac-own">
      {parts.join(" · ")}
    </p>
  );
}

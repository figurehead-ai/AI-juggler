/**
 * D-136, AJ-E3805 — the found rows: each one is its record's source link,
 * built from its kind and id (never from text). The session-row anatomy:
 * its App's mark, its name and place, its own state (amber only when it
 * needs you, D-117), and on hover or focus Watch… and Go to it. An
 * agent's words are quoted data (the guard ran on them). A past answer
 * shows how a row stands now beside how it stood then, and a record
 * since removed as "No longer here".
 */
import { useId, type JSX, type ReactNode } from "react";
import { RowMarkFor } from "./BotRowMark";
import { IconArrowRight, IconBookmark, IconEye } from "./bot-icons";
import { useBotNav } from "./bot-nav";
import { Marked } from "./Marked";
import { STATE_WORDS, rowLabel, rowPlace, stateLevel, stateSub, WORDS } from "../../lib/bot-words";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";

const DOT: Partial<Record<string, string>> = {
  working: "bg-live",
  finished: "bg-success",
  done: "bg-success",
  blocked: "bg-faint",
  in_review: "bg-faint",
  ended: "bg-faint",
};


function StateCell({ row, now }: { row: BotRowInfo; now: number }): JSX.Element | null {
  if (row.gone) return <span className="text-meta text-faint">{WORDS.gone}</span>;
  if (row.kind === "account") {
    return (
      <span className="flex flex-col items-end gap-px text-meta whitespace-nowrap">
        {row.used_percent !== null && <span className="font-medium text-ink tabular-nums">{row.used_percent} % used</span>}
        <span className="text-muted">{stateSub(row, now)}</span>
      </span>
    );
  }
  if (!row.state) return null;
  const level = stateLevel(row.state);
  const dot = DOT[row.state];
  return (
    <span className="flex flex-col items-end gap-px text-meta whitespace-nowrap">
      <span className={"flex items-center gap-1.5 font-medium " + (level === 2 ? "font-semibold text-warning" : level === 1 ? "text-danger" : "text-ink-2")}>
        {dot && <span aria-hidden="true" className={`size-1.5 rounded-full ${dot}`} />}
        {STATE_WORDS[row.state]}
      </span>
      <span className="text-muted tabular-nums">{stateSub(row, now)}</span>
    </span>
  );
}

function Facts({ row, alias }: { row: BotRowInfo; alias?: string }): JSX.Element | null {
  const pills: ReactNode[] = [];
  if (alias) pills.push(<span className="inline-flex items-center gap-1"><IconBookmark width={12} height={12} aria-hidden="true" />you call it {alias}</span>);
  if (row.files_changed) pills.push(`${row.files_changed} ${row.files_changed === 1 ? "file" : "files"} changed`);
  if (row.branch) pills.push(row.branch);
  if (row.kind === "account" && row.used_percent !== null) {
    return (
      <div className="col-span-2 col-start-2 mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-3" aria-hidden="true">
        <i className="block h-full rounded-full bg-ink-2 opacity-70" style={{ width: `${Math.min(100, row.used_percent)}%` }} />
      </div>
    );
  }
  if (pills.length === 0) return null;
  return (
    <div className="col-span-2 col-start-2 mt-1.5 flex flex-wrap gap-1.5">
      {pills.map((pill, i) => (
        <span key={i} className="inline-flex h-[22px] items-center rounded-full bg-surface-3 px-2 text-label font-medium text-ink-2">
          {pill}
        </span>
      ))}
    </div>
  );
}

/** One row, its source link. */
function BotRow({ row, n, now, find, onWatch, alias }: { row: BotRowInfo; n?: number; now: number; find?: string; onWatch?: (row: BotRowInfo) => void; alias?: string }): JSX.Element {
  const nav = useBotNav();
  const stateId = useId();
  const live = row.kind === "session" && (row.state === "working" || row.state === "asks_you" || row.state === "stuck" || row.state === "limit");
  return (
    <li
      className={"group relative grid grid-cols-[24px_minmax(0,1fr)_auto] items-start gap-x-2.5 rounded-[10px] px-2.5 py-[9px] " + (row.gone ? "" : "hover:bg-selected focus-within:bg-selected")}
      data-testid="bot-row"
      data-kind={row.kind}
      data-state={row.state ?? ""}
    >
      <span className="mt-px">
        <RowMarkFor row={row} />
      </span>
      <button
        type="button"
        className="col-span-1 flex min-w-0 flex-col items-start text-left outline-none after:absolute after:inset-0 after:rounded-[10px] focus-visible:after:ring-2 focus-visible:after:ring-accent/60"
        onClick={() => !row.gone && nav.openRow(row)}
        aria-describedby={stateId}
        title={rowLabel(row, now)}
        aria-disabled={row.gone || undefined}
        data-testid="bot-row-open"
      >
        <span className={"flex min-w-0 items-center gap-1.5 text-body " + (row.gone ? "text-muted" : "font-medium text-ink")}>
          {n !== undefined && (
            <span className="inline-grid size-[18px] shrink-0 place-items-center rounded-md bg-accent-soft text-label font-semibold text-accent-hi">{n}</span>
          )}
          <span className="truncate">{row.name_hidden ? "A record with a hidden name" : <Marked text={row.title} words={find} />}</span>
        </span>
        <span className="text-meta text-muted">
          <span className="sr-only"> </span>
          {rowPlace(row)}
        </span>
      </button>
      <span id={stateId} className={row.gone ? "" : "group-hover:invisible group-focus-within:invisible"}>
        <StateCell row={row} now={now} />
      </span>
      {row.quote && (
        <div className="col-span-2 col-start-2 mt-1.5 py-0.5 pl-2.5 text-meta leading-snug text-ink-2 shadow-[inset_2px_0_0_var(--color-hairline)]">“{row.quote}”</div>
      )}
      <Facts row={row} alias={alias} />
      {row.then_state && row.state && !row.gone && (
        <div className="col-span-2 col-start-2 mt-1.5 flex items-center gap-2 text-meta" data-testid="bot-row-then">
          <span className="text-faint">{WORDS.then}</span>
          <s className="text-muted">{STATE_WORDS[row.then_state]}</s>
          <IconArrowRight width={12} height={12} aria-hidden="true" className="text-faint" />
          <span className="text-faint">{WORDS.now}</span>
          <span className="font-medium text-ink-2">{STATE_WORDS[row.state]}</span>
        </div>
      )}
      <span className={"absolute top-[7px] right-2 z-[1] items-center gap-0.5 " + (row.gone ? "hidden" : "pointer-events-none flex opacity-0 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100")}>
        {onWatch && row.kind === "session" && (
          <button type="button" className="grid size-[30px] place-items-center rounded-lg text-muted outline-none hover:bg-surface-3 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" title={WORDS.watch} aria-label={`${WORDS.watch} ${rowLabel(row, now)}`} onClick={() => onWatch(row)} data-testid="bot-row-watch">
            <IconEye width={15} height={15} aria-hidden="true" />
          </button>
        )}
        {live && nav.goTo && (
          <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg px-2 text-meta font-medium text-ink-2 outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.goTo?.(row)} aria-label={`${WORDS.goTo}: ${rowLabel(row, now)}`} data-testid="bot-row-goto">
            {WORDS.goTo}
            <IconArrowRight width={13} height={13} aria-hidden="true" />
          </button>
        )}
      </span>
    </li>
  );
}

/** The rows, as one card; numbered when a written answer cites them. */
export function BotRows({ rows, numbered = false, now, find, onWatch, label, aliases }: { rows: readonly BotRowInfo[]; numbered?: boolean; now: number; find?: string; onWatch?: (row: BotRowInfo) => void; label: string; aliases?: ReadonlyMap<string, string> }): JSX.Element {
  return (
    <ul className="flex flex-col rounded-[14px] bg-surface p-1 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface [&>li+li]:before:absolute [&>li+li]:before:inset-x-2.5 [&>li+li]:before:-top-px [&>li+li]:before:h-px [&>li+li]:before:bg-divider" aria-label={label} data-testid="bot-rows">
      {rows.map((row, i) => (
        <BotRow key={`${row.kind}:${row.id}`} row={row} n={numbered ? i + 1 : undefined} now={now} find={find} onWatch={onWatch} alias={aliases?.get(`${row.kind}:${row.id}`)} />
      ))}
    </ul>
  );
}

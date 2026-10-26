/**
 * AJ-E2404 — a row's actions: Stop… (always the verify sheet) where a stop
 * is offered, and a menu with icons in the menus' order (go first, then
 * the row's own, then copy): Keep, Remind me in 1 h, Go to session, Reveal
 * folder, Copy PID; for another app's process, Go to its app and Show in
 * Activity Monitor. A session that left something behind offers "Clean up
 * what this session left (N)…".
 */
import { useState, type JSX, type MouseEvent } from "react";
import { Button } from "../../components/primitives";
import { MenuItem, MenuSeparator, RowMenu, menuPoint } from "../../components/RowMenu";
import { IconCopy, IconFolderOpen, IconGoTo, IconSnooze, IconStop, IconInfo } from "../../components/icons-actions";
import { IconComputer, IconBroom } from "../../components/icons-actions";
import { IconPin } from "../../components/icons";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";

export interface RowHandlers {
  stop: (row: ProcessGraphRowInfo) => void;
  mark: (row: ProcessGraphRowInfo, mark: "keep" | "snooze" | null) => void;
  goToSession: (row: ProcessGraphRowInfo) => void;
  goToApp: (row: ProcessGraphRowInfo) => void;
  reveal: (row: ProcessGraphRowInfo) => void;
  activityMonitor: () => void;
  copyPid: (pid: number) => void;
  cleanSession: (row: ProcessGraphRowInfo) => void;
  /** On the session's own page (its Processes tab): its Stop is there. */
  onSessionPage?: boolean;
}

/** A live session's own process AI Juggler started: stopping it is its
 * page's Stop, with its own rules (0.1.5 took it out of Cleanup), so the
 * row offers Go to session instead. */
export function isSessionItself(row: ProcessGraphRowInfo): boolean {
  const proc = row.process;
  return row.kind === "process" && row.origin === "ours" && !!row.session_key && proc?.role === "tool" && proc.stop === "never" && ALIVE.has(row.state);
}

const ALIVE = new Set(["working", "idle", "left_behind", "left_behind_closed", "picked_up", "didnt_stop"]);

/** Whether the row offers Stop…. */
export function canStop(row: ProcessGraphRowInfo): boolean {
  const proc = row.process;
  return row.kind === "process" && !!proc && proc.stop !== "never" && ALIVE.has(row.state);
}

function Line({ Icon, children }: { Icon: (props: { width: number; height: number; "aria-hidden": "true" }) => JSX.Element; children: string }): JSX.Element {
  return (
    <span className="flex items-center gap-2">
      <Icon width={14} height={14} aria-hidden="true" />
      {children}
    </span>
  );
}

export function RowActions({ row, on }: { row: ProcessGraphRowInfo; on: RowHandlers }): JSX.Element | null {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  if (row.kind === "app") return null;
  const proc = row.process;
  const theirs = row.origin === "theirs";
  const leftovers = row.kind === "session" ? (row.rollup?.to_clean ?? 0) : 0;
  const program = row.kind === "program";
  const sessionItself = isSessionItself(row) && !on.onSessionPage;
  const name = row.kind === "session" ? (row.session?.title ?? "session") : program ? (row.app?.name ?? "program") : (proc?.name ?? "process");
  const open = (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    setMenu(menuPoint(event));
  };
  const run = (action: () => void) => () => {
    setMenu(null);
    action();
  };
  return (
    <>
      {canStop(row) && (
        <Button size="sm" variant="secondary" tabIndex={-1} onClick={(event) => { event.stopPropagation(); on.stop(row); }} data-testid={`process-stop-${proc!.pid}`} aria-label={`Stop ${name}…`}>
          Stop…
        </Button>
      )}
      {sessionItself && (
        <Button size="sm" variant="secondary" className="whitespace-nowrap" tabIndex={-1} onClick={(event) => { event.stopPropagation(); on.goToSession(row); }} data-testid={`process-go-session-${proc!.pid}`} aria-label={`Go to session (${name} is the session itself)`}>
          Go to session
        </Button>
      )}
      {row.kind === "session" && leftovers > 0 && (
        <Button size="sm" variant="secondary" className="whitespace-nowrap" tabIndex={-1} onClick={(event) => { event.stopPropagation(); on.cleanSession(row); }} data-testid={`session-clean-${row.key}`} aria-label={`Clean up what ${name} left (${leftovers})…`}>
          Clean up ({leftovers})…
        </Button>
      )}
      <button
        type="button"
        tabIndex={-1}
        aria-label={`More for ${name}`}
        aria-haspopup="menu"
        onClick={open}
        className="inline-flex size-7 items-center justify-center rounded-lg text-muted hover:bg-surface-3 hover:text-ink"
        data-testid={`process-menu-${row.key}`}
      >
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
          <circle cx="3" cy="7" r="1.3" fill="currentColor" />
          <circle cx="7" cy="7" r="1.3" fill="currentColor" />
          <circle cx="11" cy="7" r="1.3" fill="currentColor" />
        </svg>
      </button>
      {menu && (
        <RowMenu heading={name} ariaLabel={`Actions for ${name}`} at={menu} onClose={() => setMenu(null)}>
          {row.kind === "session" && leftovers > 0 && (
            <MenuItem onClick={run(() => on.cleanSession(row))} testId="menu-clean-session">
              <Line Icon={IconBroom}>{`Clean up what this session left (${leftovers})…`}</Line>
            </MenuItem>
          )}
          {canStop(row) && (
            <MenuItem onClick={run(() => on.stop(row))} danger testId="menu-stop">
              <Line Icon={IconStop}>Stop…</Line>
            </MenuItem>
          )}
          {row.session_key && !theirs && (
            <MenuItem onClick={run(() => on.goToSession(row))} testId="menu-go-session">
              <Line Icon={IconGoTo}>Go to session</Line>
            </MenuItem>
          )}
          {theirs && (
            <MenuItem onClick={run(() => on.goToApp(row))} testId="menu-go-app">
              <Line Icon={IconGoTo}>Go to its app</Line>
            </MenuItem>
          )}
          {program ? (
            <MenuItem onClick={run(on.activityMonitor)} testId="menu-activity-monitor">
              <Line Icon={IconComputer}>Show in Activity Monitor</Line>
            </MenuItem>
          ) : (
            <MenuItem onClick={run(() => on.reveal(row))} testId="menu-reveal">
              <Line Icon={IconFolderOpen}>Reveal folder</Line>
            </MenuItem>
          )}
          {row.kind === "process" && proc && (
            <>
              <MenuSeparator />
              {proc.mark === "keep" ? (
                <MenuItem onClick={run(() => on.mark(row, null))} testId="menu-unkeep">
                  <Line Icon={IconPin}>Don't keep</Line>
                </MenuItem>
              ) : (
                <MenuItem onClick={run(() => on.mark(row, "keep"))} testId="menu-keep">
                  <Line Icon={IconPin}>Keep (not a leftover)</Line>
                </MenuItem>
              )}
              {proc.mark === "snooze" ? (
                <MenuItem onClick={run(() => on.mark(row, null))} testId="menu-unsnooze">
                  <Line Icon={IconSnooze}>Remind me now</Line>
                </MenuItem>
              ) : (
                <MenuItem onClick={run(() => on.mark(row, "snooze"))} testId="menu-snooze">
                  <Line Icon={IconSnooze}>Remind me in 1 h</Line>
                </MenuItem>
              )}
              <MenuSeparator />
              <MenuItem onClick={run(() => on.copyPid(proc.pid))} testId="menu-copy-pid">
                <Line Icon={IconCopy}>{`Copy PID (${proc.pid})`}</Line>
              </MenuItem>
              {theirs && (
                <MenuItem onClick={run(on.activityMonitor)} testId="menu-activity-monitor">
                  <Line Icon={IconComputer}>Show in Activity Monitor</Line>
                </MenuItem>
              )}
              {proc.stop === "never" && proc.stop_reason && (
                <div className="flex max-w-[18rem] items-start gap-2 px-2 py-1 text-label text-muted">
                  <IconInfo width={14} height={14} aria-hidden="true" className="mt-0.5 shrink-0" />
                  <span>{proc.stop_reason}</span>
                </div>
              )}
            </>
          )}
        </RowMenu>
      )}
    </>
  );
}

/**
 * AJ-E2404 (carried over from Cleanup, AJ-E508/E512): what the automatic
 * cleanup is doing, said on the page and never silent: paused, held after
 * a crash until you re-confirm, a crash's leftover about to be tidied with
 * its Undo window, the ledger unable to record (read-only), and the
 * passive alerts (a heavy helper, an idle one). An alert's only action
 * finds the process in the list; stopping it is the row's verified sheet.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import {
  cleanupAlerts,
  cleanupPauseSet,
  cleanupResumeAfterCrash,
  cleanupStatus,
  cleanupTier2Decide,
  errorText,
  type CleanupAlertData,
  type CleanupStatusData,
} from "../../lib/bridge";

const BANNER = "flex flex-wrap items-center gap-2 rounded-lg border border-warning/40 bg-warning/5 px-3 py-2 text-meta text-ink";

function Tier2Notices({ status, onDecide }: { status: CleanupStatusData; onDecide: (pid: number) => void }): JSX.Element {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1_000);
    return () => clearInterval(timer);
  }, []);
  return (
    <ul className="flex flex-col gap-2" data-testid="cleanup-tier2-notices">
      {status.tier2_pending.map((notice) => {
        const left = Math.max(0, Math.ceil((notice.deadline_ms - now) / 1000));
        return (
          <li key={notice.pid} className={BANNER} data-testid={`cleanup-tier2-notice-${notice.pid}`}>
            <span className="min-w-0 flex-1">
              {notice.cmdline_trunc} (PID {notice.pid}) outlived a crash: it will be tidied{left > 0 ? ` in ${left} s` : " now"} unless you keep it.
            </span>
            <span aria-live="polite" className="sr-only" data-testid={`cleanup-tier2-countdown-${notice.pid}`}>
              {left > 0 ? `${left} seconds until the automatic tidy` : "The undo window has closed"}
            </span>
            <Button size="sm" variant="secondary" onClick={() => onDecide(notice.pid)} data-testid={`cleanup-tier2-undo-${notice.pid}`}>
              Undo: keep it running
            </Button>
          </li>
        );
      })}
    </ul>
  );
}

export function CleanupStatus({ degraded, onFind }: { degraded: boolean; onFind: (pid: number) => void }): JSX.Element | null {
  const [status, setStatus] = useState<CleanupStatusData | null>(null);
  const [alerts, setAlerts] = useState<CleanupAlertData[]>([]);
  const [resumeError, setResumeError] = useState<string | null>(null);
  const resuming = useRef(false);

  useEffect(() => {
    let alive = true;
    void cleanupStatus().then((next) => alive && setStatus(next), () => undefined);
    void cleanupAlerts().then((next) => alive && setAlerts(next.alerts), () => undefined);
    return () => {
      alive = false;
    };
  }, []);
  // A Tier-2 window counts down: its status is read each second while open.
  useEffect(() => {
    if ((status?.tier2_pending.length ?? 0) === 0) return;
    const timer = setInterval(() => void cleanupStatus().then(setStatus, () => undefined), 1_000);
    return () => clearInterval(timer);
  }, [status?.tier2_pending.length]);

  const pending = status?.tier2_pending.length ?? 0;
  if (!status?.paused && !status?.suspended && pending === 0 && !degraded && alerts.length === 0) return null;
  return (
    <div className="flex flex-col gap-2" data-testid="cleanup-status">
      {status?.paused && (
        <div className={BANNER}>
          <p role="status" className="min-w-0 flex-1" data-testid="cleanup-paused-banner">
            Automatic cleanup is paused.
          </p>
          <Button size="sm" variant="secondary" onClick={() => void cleanupPauseSet(false).then(setStatus)} data-testid="cleanup-pause-toggle">
            Resume automatic cleanup
          </Button>
        </div>
      )}
      {status?.suspended && (
        <div className={BANNER} role="status" data-testid="cleanup-suspended-banner">
          <span className="min-w-0 flex-1">AI Juggler closed unexpectedly, so automatic cleanup waits for you to re-confirm it (up to 24 h).</span>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              if (resuming.current) return;
              resuming.current = true;
              setResumeError(null);
              void cleanupResumeAfterCrash()
                .then(setStatus)
                .catch((cause: unknown) => setResumeError(errorText(cause)))
                .finally(() => {
                  resuming.current = false;
                });
            }}
            data-testid="cleanup-resume-button"
          >
            Re-confirm
          </Button>
          {resumeError !== null && (
            <span role="alert" className="text-danger" data-testid="cleanup-resume-error">
              Couldn't resume: {resumeError}
            </span>
          )}
        </div>
      )}
      {status && pending > 0 && <Tier2Notices status={status} onDecide={(pid) => void cleanupTier2Decide(pid, true).then(setStatus)} />}
      {degraded && (
        <p className={BANNER} role="alert" data-testid="cleanup-degraded-banner">
          AI Juggler can't record what it starts right now, so nothing can be stopped from here until it restarts.
        </p>
      )}
      {alerts.length > 0 && (
        <ul role="status" aria-label="Alerts about processes" className="flex flex-col gap-1.5" data-testid="cleanup-alerts">
          {alerts.map((alert) => (
            <li key={`${alert.kind}-${alert.pid}`} className="flex flex-wrap items-center gap-2 rounded-lg border border-hairline bg-surface px-3 py-2 text-meta text-ink" data-testid={`${alert.kind === "hog" ? "hog-alert" : "idle-nudge"}-${alert.pid}`}>
              <span className="min-w-0 flex-1">{alert.text}</span>
              <Button size="sm" variant="ghost" onClick={() => onFind(alert.pid)} data-testid={`alert-find-${alert.pid}`}>
                Show it
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

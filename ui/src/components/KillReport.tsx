/**
 * The stop sheet's last step: how each process ended, in the words the
 * rest of the app uses ("Stopped when asked", "Needed a force stop"), what
 * still holds a port (a fact, never claimed freed), what a wider stop left
 * alone and why (ADR-0010: every member is checked again before any
 * signal), and where the stop is saved.
 */
import { Button } from "./primitives";
import { outcomeWords } from "../lib/process-states";
import type { KillExecuteData, KillMemberOutcomeInfo } from "./KillSheet";

export function KillReport({
  report,
  pid,
  goalPort,
  onShowHistory,
  onReoffer,
}: {
  report: KillExecuteData;
  pid: number;
  goalPort: number | null;
  /** Opens Cleanup history (a plain sentence without it). */
  onShowHistory?: () => void;
  /** Checks the process that still holds the port, afresh. */
  onReoffer: (pid: number) => void;
}) {
  const reoffer = report.reoffer_pid;
  return (
    <div data-testid={`kill-report-${pid}`}>
      {report.refusal !== null && (
        <p className="rounded-lg border border-danger/40 bg-danger/5 p-2.5 text-ink">{report.refusal}</p>
      )}
      {report.cancelled && <p className="text-ink">Cancelled: it was asked to stop and may still close on its own.</p>}
      <ul className="mt-2 flex flex-col gap-1 font-mono text-label">
        {report.members.map((member) => (
          <li key={member.pid} data-testid={`kill-outcome-${member.pid}`}>
            PID {member.pid} — <span className="font-sans">{outcomeWords(member.outcome)}</span>
          </li>
        ))}
      </ul>
      {report.still_holding.length > 0 && (
        <div className="mt-2" data-testid={`kill-still-holding-${pid}`}>
          <p className="text-warning">Still in use:</p>
          <ul className="font-mono text-label">
            {report.still_holding.map((held) => (
              <li key={`${held.pid}-${held.port}`}>
                :{held.port} by {held.exe} (PID {held.pid})
              </li>
            ))}
          </ul>
        </div>
      )}
      {!report.all_succeeded && report.refusal === null && !report.cancelled && (
        <p className="mt-1 text-danger">A process didn't stop, even when forced: it is still running.</p>
      )}
      {report.all_succeeded && report.still_holding.length === 0 && (
        <p className="mt-1 text-ink" data-testid={`kill-success-${pid}`}>
          {goalPort !== null ? `:${goalPort} is free.` : "Done: every process it checked has stopped."}
        </p>
      )}
      {report.refusal === null && <LeftAlone members={report.members} target={pid} />}
      {report.refusal === null && (
        <p className="mt-2 text-label" data-testid={`kill-history-${pid}`}>
          {onShowHistory ? (
            <button
              type="button"
              className="rounded text-accent underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent"
              onClick={onShowHistory}
              data-testid={`kill-history-link-${pid}`}
            >
              Saved in Cleanup history
            </button>
          ) : (
            "Saved in Cleanup history."
          )}
        </p>
      )}
      {reoffer !== null && reoffer !== pid && (
        <p className="mt-2">
          <Button size="sm" variant="secondary" onClick={() => onReoffer(reoffer)} data-testid={`kill-reoffer-${pid}`}>
            Check PID {reoffer}: it still holds the port
          </Button>
        </p>
      )}
    </div>
  );
}

/** What a wider stop left alone, a sentence each ("1 process had already
 * ended."). The target's own outcome is already its line above. */
function LeftAlone({ members, target }: { members: KillMemberOutcomeInfo[]; target: number }) {
  const others = members.filter((member) => member.pid !== target);
  const ended = others.filter((member) => member.outcome === "already gone").length;
  const left = others.filter((member) => member.outcome.startsWith("left alone")).length;
  if (ended === 0 && left === 0) return null;
  const count = (n: number) => (n === 1 ? "1 process" : `${n} processes`);
  const said = [
    ended > 0 ? `${count(ended)} had already ended.` : null,
    left > 0 ? `${count(left)} left running: not confirmed as ${left === 1 ? "one" : "ones"} AI Juggler started.` : null,
  ];
  return (
    <p className="mt-1" data-testid="kill-left-alone">
      {said.filter((part) => part !== null).join(" ")}
    </p>
  );
}

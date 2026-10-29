/**
 * AJ-E2404 — "Clean up what this session left (N)…": every process the
 * session left behind, each checked now (the same check as the kill
 * sheet's: AI Juggler's own through the ledger, another app's through its
 * sighting), then stopped one at a time, this process only, with each
 * outcome shown. A process that fails its check is listed with why, and
 * left alone.
 */
import { useEffect, useState, type JSX } from "react";
import { Modal } from "../../components/form";
import { Button } from "../../components/primitives";
import { killExecute, killVerify } from "../../lib/bridge";
import {
  processesObservedStop,
  processesObservedVerify,
  processesPage,
  processQuery,
  type ProcessGraphRowInfo,
} from "../../lib/ipc/processes";
import { checkedWords, outcomeWords, portWords } from "../../lib/process-states";

type Line = {
  row: ProcessGraphRowInfo;
  check: "checking" | "ok" | "refused";
  why: string | null;
  /** May hold unsaved state (an MCP server, a shell, a database). */
  warning: string | null;
  outcome: string | null;
};

const CLEAN = new Set(["left_behind", "left_behind_closed", "didnt_stop"]);

function checkWords(observed: boolean): string {
  return observed ? `${checkedWords(true)}, and the session has ended` : checkedWords(false);
}

export function GroupCleanupSheet({ session, onClose }: { session: ProcessGraphRowInfo; onClose: (changed: boolean) => void }): JSX.Element {
  const info = session.session!;
  const observed = session.origin === "theirs";
  const [lines, setLines] = useState<Line[] | null>(null);
  const [running, setRunning] = useState(false);
  const [done, setDone] = useState(false);

  useEffect(() => {
    let alive = true;
    void (async () => {
      const page = await processesPage(processQuery({ view: "running", session_id: info.session_id }), { cursor: null, limit: 200 });
      const rows = page.items.filter((row) => row.kind === "process" && CLEAN.has(row.state) && row.process?.stop !== "never" && row.process?.mark !== "keep");
      if (!alive) return;
      setLines(rows.map((row) => ({ row, check: "checking", why: null, warning: null, outcome: null })));
      for (const [at, row] of rows.entries()) {
        const proc = row.process!;
        try {
          const verdict = observed ? await processesObservedVerify(proc.pid, proc.start_time) : await killVerify(info.session_id, proc.pid);
          if (!alive) return;
          const warning = verdict.members.find((member) => member.pid === proc.pid)?.unsaved_state_warning ?? null;
          setLines((now) => now && now.map((line, i) => (i === at ? { ...line, check: verdict.verified ? "ok" : "refused", why: verdict.refusal, warning } : line)));
        } catch (failure) {
          setLines((now) => now && now.map((line, i) => (i === at ? { ...line, check: "refused", why: String(failure) } : line)));
        }
      }
    })();
    return () => {
      alive = false;
    };
  }, [info.session_id, observed]);

  const stopAll = async () => {
    if (!lines) return;
    setRunning(true);
    for (const [at, line] of lines.entries()) {
      if (line.check !== "ok") continue;
      const proc = line.row.process!;
      const attempt = `group-${proc.pid}-${Date.now()}`;
      try {
        const result = observed
          ? await processesObservedStop(proc.pid, proc.start_time, attempt)
          : await killExecute(info.session_id, proc.pid, "pid", attempt, null);
        const said = result.refusal ?? result.members.map((member) => member.outcome).join("; ");
        setLines((now) => now && now.map((other, i) => (i === at ? { ...other, outcome: result.refusal ? said : outcomeWords(said) } : other)));
      } catch (failure) {
        setLines((now) => now && now.map((other, i) => (i === at ? { ...other, outcome: String(failure) } : other)));
      }
    }
    setRunning(false);
    setDone(true);
  };

  const ready = lines?.filter((line) => line.check === "ok").length ?? 0;
  const checking = lines?.some((line) => line.check === "checking") ?? true;
  return (
    <Modal title={`Clean up what ${info.title || "this session"} left`} subtitle="Each one is checked now, then stopped on its own: asked politely first, forced after 5 s only if it ignores that." onClose={() => onClose(done)} wide testId="group-cleanup">
      <ul className="flex flex-col divide-y divide-hairline" data-testid="group-cleanup-lines">
        {(lines ?? []).map((line) => {
          const proc = line.row.process!;
          return (
            <li key={line.row.key} className="flex flex-col gap-0.5 py-2.5" data-testid={`group-line-${proc.pid}`}>
              <span className="text-body font-medium text-ink">
                {proc.name}
                {line.row.ports.length > 0 && <span className="text-muted"> {portWords(line.row.ports)}</span>}
                <span className="text-label font-normal text-muted"> PID {proc.pid}</span>
              </span>
              <span className={`text-meta ${line.check === "refused" ? "text-warning" : "text-muted"}`}>
                {line.check === "checking" ? "Checking…" : line.check === "ok" ? checkWords(observed) : `Won't stop it: ${line.why ?? "the check failed"}`}
              </span>
              {line.check === "ok" && line.warning && (
                <span className="text-meta text-warning" data-testid={`group-unsaved-${proc.pid}`}>
                  {line.warning}
                </span>
              )}
              {line.outcome && (
                <span className="text-meta font-medium text-ink" role="status">
                  {line.outcome}
                </span>
              )}
            </li>
          );
        })}
        {lines?.length === 0 && <li className="py-4 text-body text-muted">Nothing left to clean up here.</li>}
      </ul>
      <div className="mt-4 flex justify-end gap-2">
        <Button variant="ghost" onClick={() => onClose(done)}>
          {done ? "Close" : "Cancel"}
        </Button>
        {!done && (
          <Button variant="danger" disabled={running || checking || ready === 0} onClick={() => void stopAll()} data-testid="group-cleanup-stop" data-autofocus>
            {ready === 1 ? "Stop 1 checked process" : `Stop ${ready} checked processes`}
          </Button>
        )}
      </div>
    </Modal>
  );
}

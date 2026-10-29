import { useEffect, useRef, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import {
  errorText,
  fleetRemote,
  fleetRemoteClean,
  type RemoteCommandOutcomeInfo,
  type RemoteMachineInfo,
  type RemoteProcessInfo,
} from "../lib/bridge";

/**
 * AJ-E707 — paired machines' processes on the Processes page, grouped
 * Machine → Instance. Each row shows its verification state BEFORE any
 * action; only a process its own machine verified as owned can be
 * cleaned, and only with View + control and an explicit confirmation.
 * The owning machine re-verifies and runs its own TERM → grace → KILL
 * ladder; the outcome comes back in its words. Everything else reads
 * "Not owned — never touched" and has no Clean action.
 */
export function RemoteCleanup(): JSX.Element | null {
  const [machines, setMachines] = useState<RemoteMachineInfo[]>([]);
  const [confirm, setConfirm] = useState<{ machine: RemoteMachineInfo; row: RemoteProcessInfo } | null>(null);
  const [outcomes, setOutcomes] = useState<Record<string, RemoteCommandOutcomeInfo | string>>({});
  const [busy, setBusy] = useState(false);
  const headerRefs = useRef<Record<string, HTMLHeadingElement | null>>({});

  useEffect(() => {
    let alive = true;
    const load = () => {
      fleetRemote()
        .then((next) => {
          if (alive) setMachines(next);
        })
        .catch(() => null);
    };
    load();
    const timer = window.setInterval(load, 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  const withProcesses = machines.filter((machine) => (machine.processes ?? []).length > 0);
  if (withProcesses.length === 0) return null;

  const clean = async (machine: RemoteMachineInfo, row: RemoteProcessInfo) => {
    setBusy(true);
    const key = `${machine.machine_id}:${row.pid}`;
    try {
      const outcome = await fleetRemoteClean(machine.machine_id, row.instance_id, row.pid);
      setOutcomes((now) => ({ ...now, [key]: outcome }));
    } catch (cause) {
      setOutcomes((now) => ({ ...now, [key]: errorText(cause) }));
    } finally {
      setBusy(false);
      setConfirm(null);
      // Focus returns to the machine's group header after the action.
      headerRefs.current[machine.machine_id]?.focus();
    }
  };

  return (
    <section className="mt-4 flex flex-col gap-3" aria-labelledby="remote-cleanup-title" data-testid="remote-cleanup">
      <h2 id="remote-cleanup-title" className="text-section font-semibold text-ink">
        Your other Macs
      </h2>
      {withProcesses.map((machine) => {
        const canClean = machine.permission === "control" && machine.connection === "connected";
        const byInstance = groupBy(machine.processes, (row) => row.instance_id);
        const verifiedCount = machine.processes.filter((row) => row.verified).length;
        return (
          <Card key={machine.machine_id} className="flex flex-col gap-2 p-3" data-testid="remote-cleanup-machine">
            <h3
              ref={(element) => {
                headerRefs.current[machine.machine_id] = element;
              }}
              tabIndex={-1}
              className="flex flex-wrap items-center gap-2 text-sm font-semibold outline-none"
            >
              {machine.label}
              <Chip tone="neutral">
                {machine.freshness === "live" ? "● live" : machine.freshness === "stale" ? "◐ stale" : "○ offline"}
              </Chip>
              <span className="text-label font-normal text-muted">
                {verifiedCount} verified · {machine.processes.length - verifiedCount} not owned or unverified
              </span>
            </h3>
            {!canClean && (
              <p className="text-label text-muted" data-testid="remote-cleanup-readonly">
                {machine.permission === "control"
                  ? `${machine.label} isn't connected — cleaning waits until it is.`
                  : `View-only pairing — cleaning needs View + control on ${machine.label}.`}
              </p>
            )}
            {Object.entries(byInstance).map(([instanceId, rows]) => (
              <div key={instanceId} className="flex flex-col gap-1">
                <p className="text-label font-medium text-muted">
                  {rows[0]!.account_label || "Session"} · {rows[0]!.provider}
                </p>
                <ul className="flex flex-col divide-y divide-hairline rounded-lg border border-hairline">
                  {rows.map((row) => {
                    const key = `${machine.machine_id}:${row.pid}`;
                    const outcome = outcomes[key];
                    return (
                      <li key={row.pid} className="flex flex-wrap items-center gap-2 px-2.5 py-1.5 text-xs" data-testid="remote-cleanup-row" data-verified={row.verified ? "true" : "false"}>
                        <span className="font-mono text-label">{row.name || "process"}</span>
                        <span className="text-muted">pid {row.pid}</span>
                        <span className="text-muted">· {row.state_label}</span>
                        {row.ports && <span className="text-muted">· ports {row.ports}</span>}
                        <span className="flex-1" />
                        {row.verified ? (
                          <Chip tone="success">✓ Verified owned</Chip>
                        ) : (
                          <Chip tone="neutral">Not owned — never touched</Chip>
                        )}
                        {row.verified && (
                          <Button
                            size="sm"
                            variant="danger"
                            disabled={!canClean || busy}
                            onClick={() => setConfirm({ machine, row })}
                            aria-label={`Clean ${row.name || "process"} (pid ${row.pid}) on ${machine.label}`}
                            data-testid="remote-cleanup-clean"
                          >
                            Clean…
                          </Button>
                        )}
                        {outcome !== undefined && (
                          <span className="w-full text-label" aria-live="polite" data-testid="remote-cleanup-outcome">
                            {typeof outcome === "string"
                              ? outcome
                              : outcome.accepted
                                ? `${machine.label}: ${outcome.message}`
                                : `${machine.label} refused: ${outcome.message}`}
                          </span>
                        )}
                      </li>
                    );
                  })}
                </ul>
              </div>
            ))}
          </Card>
        );
      })}
      {confirm && (
        <CleanSheet
          machine={confirm.machine}
          row={confirm.row}
          busy={busy}
          onCancel={() => setConfirm(null)}
          onConfirm={() => void clean(confirm.machine, confirm.row)}
        />
      )}
    </section>
  );
}

function groupBy<T>(rows: T[], key: (row: T) => string): Record<string, T[]> {
  const out: Record<string, T[]> = {};
  for (const row of rows) (out[key(row)] ??= []).push(row);
  return out;
}

function CleanSheet({
  machine,
  row,
  busy,
  onCancel,
  onConfirm,
}: {
  machine: RemoteMachineInfo;
  row: RemoteProcessInfo;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}): JSX.Element {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    cancelRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6">
      <Card
        className="w-full max-w-sm p-5"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="remote-clean-title"
        aria-describedby="remote-clean-body"
        data-testid="remote-cleanup-confirm"
      >
        <p id="remote-clean-title" className="text-sm font-semibold">
          Clean {row.name || "this process"} on {machine.label}?
        </p>
        <p id="remote-clean-body" className="mt-1.5 text-xs leading-relaxed text-muted">
          {machine.label} checks again that pid {row.pid} is still the process it recorded for{" "}
          {row.account_label || "that session"}, then asks it to quit and stops it if it doesn't. If
          anything changed, nothing is touched.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <Button ref={cancelRef} size="sm" variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid="remote-cleanup-yes">
            Clean on {machine.label}
          </Button>
        </div>
      </Card>
    </div>
  );
}

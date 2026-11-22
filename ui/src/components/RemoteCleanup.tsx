import { useEffect, useRef, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { errorText } from "../lib/bridge";
import { fleetRemote, fleetRemoteClean } from "../lib/ipc/fleet-remote";
import type { RemoteCommandOutcomeInfo } from "../types/generated/RemoteCommandOutcomeInfo";
import type { RemoteMachineInfo } from "../types/generated/RemoteMachineInfo";
import type { RemoteProcessInfo } from "../types/generated/RemoteProcessInfo";
import { cliAppName } from "../lib/app-name";
import { providerDisplayName } from "../lib/providers";
import { ConfirmSheet } from "./ConfirmSheet";

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
    <section id="remote-cleanup" tabIndex={-1} className="mt-4 flex flex-col gap-3 outline-none" aria-labelledby="remote-cleanup-title" data-testid="remote-cleanup">
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
                  {rows[0]!.account_label ? cliAppName(rows[0]!.provider, rows[0]!.account_label) : `${providerDisplayName(rows[0]!.provider)} session`}
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
        <ConfirmSheet
          idPrefix="remote-clean"
          testId="remote-cleanup-confirm"
          title={`Clean ${confirm.row.name || "this process"} on ${confirm.machine.label}?`}
          action={`Clean on ${confirm.machine.label}`}
          actionTestId="remote-cleanup-yes"
          busy={busy}
          onCancel={() => setConfirm(null)}
          onConfirm={() => void clean(confirm.machine, confirm.row)}
        >
          {confirm.machine.label} checks again that pid {confirm.row.pid} is still the process it recorded for{" "}
          {confirm.row.account_label ? cliAppName(confirm.row.provider, confirm.row.account_label) : "that session"}, then asks it to quit and stops it if
          it doesn't. If anything changed, nothing is touched.
        </ConfirmSheet>
      )}
    </section>
  );
}

function groupBy<T>(rows: T[], key: (row: T) => string): Record<string, T[]> {
  const out: Record<string, T[]> = {};
  for (const row of rows) (out[key(row)] ??= []).push(row);
  return out;
}

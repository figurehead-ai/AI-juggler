/**
 * AJ-E1009 — Settings › Backup & restore. One file you can carry to a new
 * Mac (ADR-0016): AI Juggler's records, Launch settings, history,
 * lineage and parked packs, never sign-ins or this Mac's Fleet
 * identity. Export shows progress as it goes and ends by saying what it
 * wrote and what it withheld. Restore checks every file first and says
 * what it will do, including when the backup comes from another Mac
 * (its data, never its identity). Failures name the file and the reason.
 */
import { useEffect, useRef, useState } from "react";
import { Card } from "./primitives";
import {
  backupExport,
  backupRestore,
  backupVerify,
  errorText,
  onBackupProgress,
  pickBackupDestination,
  pickBackupFile,
  type BackupExportInfo,
  type RestorePlanInfo,
  type RestoreResultInfo,
} from "../lib/bridge";

type Phase =
  | { kind: "idle" }
  | { kind: "exporting"; done: number; total: number }
  | { kind: "exported"; info: BackupExportInfo }
  | { kind: "verifying"; path: string }
  | { kind: "plan"; plan: RestorePlanInfo }
  | { kind: "restoring"; plan: RestorePlanInfo }
  | { kind: "restored"; result: RestoreResultInfo }
  | { kind: "failed"; title: string; message: string };

function size(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const COUNT_WORDS: Record<string, string> = {
  accounts: "Accounts",
  launch_settings: "Launch settings",
  projects: "projects",
  instances: "Instances",
  history_events: "history events",
  switch_edges: "switches",
  epochs: "lineage steps",
};

function countLine(counts: Record<string, number>): string {
  return Object.entries(counts)
    .map(([key, value]) => `${value.toLocaleString()} ${COUNT_WORDS[key] ?? key}`)
    .join(" · ");
}

export function BackupRestoreCard(): JSX.Element {
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const restoreRef = useRef<HTMLButtonElement | null>(null);
  useEffect(
    () =>
      onBackupProgress((done, total) =>
        setPhase((now) => (now.kind === "exporting" ? { kind: "exporting", done, total } : now)),
      ),
    [],
  );
  // AJ-E1205: a Cloud restore lands here, in the same confirm step.
  useEffect(() => {
    const onPlan = (event: Event) => {
      const plan = (event as CustomEvent<RestorePlanInfo>).detail;
      if (plan) setPhase({ kind: "plan", plan });
    };
    window.addEventListener("aij:restore-plan", onPlan);
    return () => window.removeEventListener("aij:restore-plan", onPlan);
  }, []);
  useEffect(() => {
    if (phase.kind !== "plan") return;
    restoreRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPhase({ kind: "idle" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [phase.kind]);

  const exportNow = async () => {
    const dest = await pickBackupDestination();
    if (!dest) return;
    setPhase({ kind: "exporting", done: 0, total: 0 });
    try {
      setPhase({ kind: "exported", info: await backupExport(dest) });
    } catch (cause) {
      setPhase({ kind: "failed", title: "The backup was not written", message: errorText(cause) });
    }
  };
  const verify = async () => {
    const path = await pickBackupFile();
    if (!path) return;
    setPhase({ kind: "verifying", path });
    try {
      setPhase({ kind: "plan", plan: await backupVerify(path) });
    } catch (cause) {
      setPhase({ kind: "failed", title: "Nothing was restored", message: errorText(cause) });
    }
  };
  const restore = async (plan: RestorePlanInfo) => {
    setPhase({ kind: "restoring", plan });
    try {
      setPhase({ kind: "restored", result: await backupRestore(plan.path, plan.token) });
    } catch (cause) {
      setPhase({ kind: "failed", title: "The restore did not finish", message: errorText(cause) });
    }
  };
  const busy = phase.kind === "exporting" || phase.kind === "verifying" || phase.kind === "restoring";
  const percent = phase.kind === "exporting" && phase.total > 0 ? Math.floor((phase.done / phase.total) * 100) : 0;

  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-backup">
      <h3 className="text-sm font-semibold">Backup &amp; restore</h3>
      <p className="text-[11px] text-muted">
        One file with AI Juggler's records, Launch settings, history and lineage, to keep or to move to a new Mac. It never
        holds your sign-ins or this Mac's identity, and it stays on your disk; nothing is uploaded.
      </p>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => void exportNow()}
          className="rounded-lg border border-hairline px-3 py-1.5 text-xs font-medium text-ink hover:border-accent/50 disabled:text-muted"
          data-testid="backup-export"
        >
          Back up now…
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void verify()}
          className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink hover:border-accent/50 disabled:text-muted"
          data-testid="backup-restore"
        >
          Restore from a backup…
        </button>
      </div>

      <div aria-live="polite" className="flex flex-col gap-2 text-[11px]" data-testid="backup-status">
        {phase.kind === "exporting" && (
          <div className="flex flex-col gap-1">
            <span className="text-ink">Writing the backup… {percent}%</span>
            <div
              role="progressbar"
              aria-label="Backup progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
              className="h-1.5 w-full overflow-hidden rounded-full bg-surface-2"
            >
              <div className="h-full rounded-full bg-accent" style={{ width: `${percent}%` }} />
            </div>
          </div>
        )}
        {phase.kind === "exported" && (
          <div className="flex flex-col gap-1 text-ink" data-testid="backup-done">
            <p>
              <span aria-hidden="true">✓ </span>Backed up — {size(phase.info.bytes)}, every file checksummed.
            </p>
            <p className="break-all font-mono text-[10.5px] text-muted">{phase.info.path}</p>
            <p className="text-muted">{countLine(phase.info.manifest.counts)}</p>
            {phase.info.manifest.withheld.length > 0 && (
              <p className="text-muted">
                Withheld (re-enter after a restore): {phase.info.manifest.withheld.join("; ")}
              </p>
            )}
          </div>
        )}
        {phase.kind === "verifying" && <p className="text-ink">Checking every file in the backup…</p>}
        {phase.kind === "restoring" && <p className="text-ink">Restoring…</p>}
        {phase.kind === "restored" && (
          <div className="flex flex-col gap-1 text-ink" data-testid="restore-done">
            <p>
              <span aria-hidden="true">✓ </span>Restored — {countLine(phase.result.counts)}, matching the backup.
            </p>
            <ul className="ml-4 list-disc">
              {phase.result.next_steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ul>
            <p className="break-all text-muted">Kept from before: {phase.result.kept_before.join(" · ")}</p>
          </div>
        )}
        {phase.kind === "failed" && (
          <div role="alert" className="flex flex-col gap-0.5 text-ink" data-testid="backup-failed">
            <p className="font-medium">
              <span aria-hidden="true">⚠ </span>
              {phase.title}
            </p>
            <p className="break-all">{phase.message}</p>
          </div>
        )}
      </div>

      {(phase.kind === "plan" || phase.kind === "restoring") && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
          <div
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="restore-title"
            aria-describedby="restore-notes"
            data-testid="restore-plan"
            className="flex max-h-full w-full max-w-lg flex-col gap-3 overflow-y-auto rounded-xl border border-hairline bg-surface p-5 shadow-card"
          >
            <h3 id="restore-title" className="text-sm font-semibold">
              Restore this backup?
            </h3>
            <p className="text-[11px] text-ink">
              <span aria-hidden="true">✓ </span>Every file checked — {countLine(phase.plan.manifest.counts)}. Made{" "}
              {new Date(phase.plan.manifest.created_at_ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" })} on{" "}
              {phase.plan.manifest.origin.label}.
            </p>
            {phase.plan.other_machine && (
              <p
                className="rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-[11px] text-ink"
                data-testid="restore-other-machine"
              >
                <span aria-hidden="true">⚠ </span>
                This backup comes from {phase.plan.manifest.origin.label}: it carries that Mac's data, never its identity.
                This Mac keeps its own and must pair again with your other Macs.
              </p>
            )}
            <ul id="restore-notes" className="ml-4 list-disc text-[11px] text-ink" data-testid="restore-notes">
              {phase.plan.notes
                .filter((note) => !(phase.plan.other_machine && note.includes("never its identity")))
                .map((note) => (
                  <li key={note}>{note}</li>
                ))}
            </ul>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                disabled={phase.kind === "restoring"}
                onClick={() => setPhase({ kind: "idle" })}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
              >
                Cancel
              </button>
              <button
                ref={restoreRef}
                type="button"
                disabled={phase.kind === "restoring"}
                onClick={() => void restore(phase.plan)}
                className="rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink hover:border-accent"
                data-testid="restore-go"
              >
                {phase.kind === "restoring" ? "Restoring…" : "Restore"}
              </button>
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}

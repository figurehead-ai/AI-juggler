/**
 * AJ-E2404 — the Processes page: every app, session and process on this
 * Mac that AI Juggler started or sees in a session another app runs, what
 * needs you first. It replaces Cleanup (its Needs attention view is what
 * was left behind); the top bar's count and This Mac lead here.
 *
 * On a wide window the automatic cleanup sits beside the table; below it
 * on a narrow one.
 */
import { useEffect, useState, type JSX } from "react";
import { Page } from "../../components/Page";
import { Button } from "../../components/primitives";
import { processesCounts, processesSnapshot, onProcessesChanged, type ProcessCountsInfo } from "../../lib/ipc/processes";
import { RemoteCleanup } from "../../components/RemoteCleanup";
import { RulesPanel } from "./RulesPanel";
import { ProcessesView } from "./ProcessesView";
import type { Choice } from "./Toolbar";

function Tally({ value, label, tone, testId }: { value: number; label: string; tone: "warning" | "danger" | "neutral"; testId: string }): JSX.Element {
  const color = value === 0 ? "text-muted" : tone === "danger" ? "text-danger" : tone === "warning" ? "text-warning" : "text-ink";
  return (
    <div className="flex min-w-[9rem] flex-col gap-0.5 rounded-card border border-hairline bg-surface px-4 py-3" data-testid={testId}>
      <span className={`text-stat tabular-nums ${color}`}>{value}</span>
      <span className="text-meta text-muted">{label}</span>
    </div>
  );
}

export function useProcessCounts(): ProcessCountsInfo | null {
  const [counts, setCounts] = useState<ProcessCountsInfo | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () => void processesCounts().then((next) => alive && setCounts(next), () => undefined);
    read();
    const stop = onProcessesChanged((change) => alive && setCounts(change.counts));
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return counts;
}

export function ProcessesScreen({
  onOpenSession,
  onOpenConversation,
  accounts,
  projects,
}: {
  onOpenSession?: (id: string) => void;
  onOpenConversation?: (accountId: string, id: string) => void;
  accounts?: Choice[];
  projects?: Choice[];
}): JSX.Element {
  const counts = useProcessCounts();
  const [saved, setSaved] = useState<string | null>(null);
  const exportSnapshot = async () => {
    const snapshot = await processesSnapshot();
    const blob = new Blob([snapshot.json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = snapshot.file_name;
    anchor.click();
    URL.revokeObjectURL(url);
    setSaved(snapshot.file_name);
  };
  return (
    <Page
      title="Processes"
      subtitle="What your sessions and apps run, what they started, and what they left behind."
      actions={
        <Button variant="secondary" onClick={() => void exportSnapshot()} data-testid="processes-export" title="Names and hashed command lines only">
          Export snapshot
        </Button>
      }
      testId="processes-page"
    >
      {saved && (
        <p className="text-meta text-muted" role="status">
          Saved {saved}: names and hashed command lines only.
        </p>
      )}
      <div className="flex flex-wrap gap-3" data-testid="processes-tallies">
        <Tally value={counts?.to_clean ?? 0} label="to clean up" tone="warning" testId="tally-clean" />
        <Tally value={counts?.didnt_stop ?? 0} label="didn't stop" tone="danger" testId="tally-didnt-stop" />
        <Tally value={counts?.zombies ?? 0} label="waiting for their parent" tone="neutral" testId="tally-zombies" />
        <Tally value={counts?.processes ?? 0} label={`running in ${counts?.sessions ?? 0} sessions`} tone="neutral" testId="tally-running" />
      </div>
      <ProcessesView onOpenSession={onOpenSession} onOpenConversation={onOpenConversation} accounts={accounts} projects={projects} />
      <RulesPanel />
      {/* AJ-E707: what a paired Mac left behind, cleaned there (shows only
          with a paired Mac). */}
      <RemoteCleanup />
    </Page>
  );
}

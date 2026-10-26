/**
 * AJ-E2405 — Cleanup history (7 days): every stop, by whom (you, you for
 * another app's session, or an automatic rule) and how it ended, newest
 * first, a page at a time; one session's or one project's on request.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { agoWords } from "../../lib/time-words";
import { actorWords, outcomeWords } from "../../lib/process-states";
import { processesHistory, type CleanupEventInfo } from "../../lib/ipc/processes";
import { cleanupAuditExport } from "../../lib/bridge";

/** Saves the audit (7 days; command lines hashed) as a file. */
async function exportAudit(): Promise<string> {
  const text = await cleanupAuditExport();
  const name = `juggler-cleanup-audit-${new Date().toISOString().slice(0, 10)}.jsonl`;
  const url = URL.createObjectURL(new Blob([text], { type: "application/x-ndjson" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  URL.revokeObjectURL(url);
  return name;
}

export function HistoryView({
  sessionId = null,
  projectId = null,
}: {
  sessionId?: string | null;
  projectId?: string | null;
}): JSX.Element {
  const [items, setItems] = useState<CleanupEventInfo[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setItems(null);
    void processesHistory(sessionId, projectId, { cursor: null, limit: null }).then(
      (page) => {
        if (!alive) return;
        setItems(page.items);
        setNext(page.next_cursor);
      },
      (failure: unknown) => alive && setError(String(failure)),
    );
    return () => {
      alive = false;
    };
  }, [sessionId, projectId]);

  const more = async () => {
    if (!next) return;
    const page = await processesHistory(sessionId, projectId, { cursor: next, limit: null });
    setItems((now) => [...(now ?? []), ...page.items]);
    setNext(page.next_cursor);
  };

  if (error) return <p className="px-4 py-6 text-body text-danger">Couldn't read the history: {error}</p>;
  if (items === null) return <div className="h-40 animate-pulse rounded-card bg-surface-2" aria-label="Loading the history" />;
  if (items.length === 0) {
    return <p className="rounded-card border border-hairline bg-surface px-4 py-10 text-center text-body text-muted">Nothing was stopped in the last 7 days.</p>;
  }
  return (
    <div className="flex flex-col gap-3">
      <div className="overflow-x-auto rounded-card border border-hairline bg-surface">
        <table className="w-full text-left" data-testid="cleanup-history">
          <thead className="bg-surface-2 text-meta text-muted">
            <tr>
              <th className="px-3 py-2 font-medium">When</th>
              <th className="px-3 py-2 font-medium">Process</th>
              <th className="px-3 py-2 font-medium">Session</th>
              <th className="px-3 py-2 font-medium">By</th>
              <th className="px-3 py-2 font-medium">How it ended</th>
              <th className="px-3 py-2 font-medium">Ports</th>
            </tr>
          </thead>
          <tbody className="text-body">
            {items.map((event) => (
              <tr key={event.id} className="border-t border-hairline" data-testid={`history-${event.id}`}>
                <td className="whitespace-nowrap px-3 py-2 text-muted">{agoWords(event.at_ms, Date.now())}</td>
                <td className="px-3 py-2 text-ink">
                  {event.name} <span className="text-label text-muted">PID {event.pid}</span>
                </td>
                <td className="px-3 py-2 text-ink">{event.session_title ?? "A session"}</td>
                <td className="px-3 py-2 text-muted">{actorWords(event.actor)}</td>
                <td className="px-3 py-2 text-ink">{outcomeWords(event.outcome)}</td>
                <td className="px-3 py-2 tabular-nums text-muted">
                  {event.ports_before ? event.ports_before.split(",").map((port) => `:${port}`).join(", ") : "—"}
                  {event.ports_after ? <span className="text-warning"> (still held: {event.ports_after})</span> : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {next && (
          <Button variant="secondary" size="sm" onClick={() => void more()}>
            Show older
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={() => void exportAudit().then(setSaved)} data-testid="cleanup-history-export">
          Export the history
        </Button>
        {saved && (
          <span role="status" className="text-meta text-muted" data-testid="cleanup-history-exported">
            Saved {saved}: command lines hashed.
          </span>
        )}
      </div>
    </div>
  );
}

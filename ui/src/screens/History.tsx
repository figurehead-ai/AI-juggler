/**
 * AJ-E1007 — History: find any past Instance in seconds and act on it.
 * One search covers project, Account, Provider, model, branch, date
 * (YYYY-MM-DD) and transcript text (a local index; the matching words
 * are shown in context). Ended and Archived are separate lists. Each row
 * offers Replay (read-only), Archive or Unarchive, and Delete…, which is
 * its own confirmed dialog naming exactly what goes and what stays.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { SessionRow } from "../components/WorkflowRail";
import { ReplayDialog } from "../components/ReplayDialog";
import { DeleteInstanceDialog } from "../components/DeleteInstanceDialog";
import {
  errorText,
  instanceArchive,
  instanceUnarchive,
  transcriptSearch,
  type InstanceInfo,
  type ProjectInfo,
} from "../lib/bridge";

type ListId = "ended" | "archived";

const ENDED = new Set(["finished", "stopped", "failed", "handed_over"]);

function localDate(ms: number): string {
  const at = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
}

/** Does every word of `query` match one of the Instance's fields? */
export function matchesFields(session: InstanceInfo, project: string, query: string): boolean {
  const fields = [
    project,
    session.account_label,
    session.provider,
    session.model ?? "",
    session.git_branch ?? "",
    localDate(session.started_at_ms),
    session.last_activity_ms ? localDate(session.last_activity_ms) : "",
  ]
    .join(" ")
    .toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => fields.includes(word));
}

export function History({
  sessions,
  projects,
  onChanged,
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onChanged: () => void;
}): JSX.Element {
  const [list, setList] = useState<ListId>("ended");
  const [query, setQuery] = useState("");
  const [snippets, setSnippets] = useState<Map<string, string>>(new Map());
  const [searching, setSearching] = useState(false);
  const [replay, setReplay] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const searchRef = useRef<HTMLInputElement | null>(null);
  const names = useMemo(() => new Map(projects.map((project) => [project.id, project.name])), [projects]);
  const projectOf = (session: InstanceInfo) => (session.project_id ? (names.get(session.project_id) ?? "") : "");

  useEffect(() => searchRef.current?.focus(), []);
  // Transcript text: debounced, so typing never waits on the index.
  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed.length < 2) {
      setSnippets(new Map());
      return;
    }
    let alive = true;
    setSearching(true);
    const timer = window.setTimeout(() => {
      transcriptSearch(trimmed)
        .then((hits) => alive && setSnippets(new Map(hits.map((hit) => [hit.session_id, hit.snippet]))))
        .catch(() => alive && setSnippets(new Map()))
        .finally(() => alive && setSearching(false));
    }, 250);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [query]);

  const archived = (session: InstanceInfo) => session.lifecycle === "archived";
  const rows = sessions
    .filter((session) => (list === "archived" ? archived(session) : ENDED.has(session.status) && !archived(session)))
    .filter((session) => !query.trim() || snippets.has(session.id) || matchesFields(session, projectOf(session), query))
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms));
  const archivedCount = sessions.filter(archived).length;

  const act = async (session: InstanceInfo, run: () => Promise<unknown>, done: string) => {
    try {
      await run();
      setStatus(`${session.account_label}: ${done}.`);
      onChanged();
    } catch (cause) {
      setStatus(`${session.account_label}: ${errorText(cause)}`);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-4" data-testid="history">
      <header className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold tracking-tight">History</h2>
        <p className="text-xs text-muted">
          Every Instance that has ended. Search by project, Account, Provider, model, branch, date (2026-10-01) or
          anything said in the transcript.
        </p>
      </header>
      <div className="flex flex-wrap items-center gap-2">
        <input
          ref={searchRef}
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search history…"
          aria-label="Search history"
          className="min-w-0 flex-1 rounded-lg border border-hairline bg-surface px-3 py-1.5 text-sm text-ink"
          data-testid="history-search"
        />
        <div role="tablist" aria-label="Which Instances" className="flex rounded-lg border border-hairline p-0.5">
          {(
            [
              { id: "ended", label: "Ended" },
              { id: "archived", label: `Archived (${archivedCount})` },
            ] as const
          ).map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={list === tab.id}
              onClick={() => setList(tab.id)}
              className={
                "rounded-md px-2.5 py-1 text-[11px] font-medium " +
                (list === tab.id ? "bg-surface-2 text-ink" : "text-muted hover:text-ink")
              }
              data-testid={`history-tab-${tab.id}`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>
      <p className="sr-only" aria-live="polite" data-testid="history-status">
        {status || (searching ? "Searching transcripts…" : `${rows.length} found`)}
      </p>
      {rows.length === 0 ? (
        <p className="rounded-xl border border-dashed border-hairline p-6 text-center text-sm text-muted" data-testid="history-empty">
          {query.trim()
            ? `Nothing matches “${query.trim()}”${searching ? " yet — still searching transcripts" : ""}.`
            : list === "archived"
              ? "Nothing archived. Archive an ended Instance to tidy the rail — nothing is deleted."
              : "No ended Instances yet."}
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {rows.map((session) => {
            const snippet = snippets.get(session.id);
            const isArchived = archived(session);
            return (
              <li
                key={session.id}
                className="flex flex-col gap-1 rounded-xl border border-hairline bg-surface/60 pb-2 pr-2"
                data-testid="history-row"
                data-session={session.id}
              >
                <div className="flex items-center gap-2">
                  <div className="min-w-0 flex-1">
                    <SessionRow session={session} selected={false} onSelect={() => setReplay(session.id)} density="compact" />
                  </div>
                  {isArchived && (
                    <span className="shrink-0 rounded-full border border-hairline px-2 py-0.5 text-[10px] text-muted">
                      Archived
                    </span>
                  )}
                  {projectOf(session) && <span className="shrink-0 text-[11px] text-muted">{projectOf(session)}</span>}
                  <span className="shrink-0 text-[11px] text-muted">
                    {localDate(session.last_activity_ms ?? session.started_at_ms)}
                  </span>
                </div>
                {snippet && (
                  <p className="mx-3 rounded bg-surface-2/60 px-2 py-1 font-mono text-[11px] text-ink" data-testid="history-snippet">
                    {snippet}
                  </p>
                )}
                <div className="flex flex-wrap gap-1.5 px-3">
                  <button
                    type="button"
                    onClick={() => setReplay(session.id)}
                    className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] font-medium text-ink hover:border-accent/50"
                    data-testid="history-replay"
                  >
                    Replay
                  </button>
                  {isArchived ? (
                    <button
                      type="button"
                      onClick={() => void act(session, () => instanceUnarchive(session.id), "unarchived")}
                      className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink hover:border-accent/50"
                      data-testid="history-unarchive"
                    >
                      Unarchive
                    </button>
                  ) : (
                    <button
                      type="button"
                      onClick={() => void act(session, () => instanceArchive(session.id), "archived — nothing deleted")}
                      className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink hover:border-accent/50"
                      data-testid="history-archive"
                    >
                      Archive
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => setDeleting(session.id)}
                    className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink hover:border-danger/50"
                    data-testid="history-delete"
                  >
                    Delete…
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      )}
      {replay && <ReplayDialog sessionId={replay} onClose={() => setReplay(null)} />}
      {deleting && (
        <DeleteInstanceDialog
          sessionId={deleting}
          onClose={(deleted) => {
            setDeleting(null);
            if (deleted) {
              setStatus("Deleted.");
              onChanged();
            }
          }}
        />
      )}
    </div>
  );
}

/**
 * AJ-E1007, AJ-E1536 — History you can scan: every session that has
 * ended, AI Juggler's and the ones started outside it (AJ-E1529), each
 * named by what it was about (its title, else the first thing asked),
 * with its tool, project and account marks, opening its page. One search
 * covers the title, project, account, tool, model, branch, date
 * (YYYY-MM-DD) and, for AI Juggler's sessions, the transcript text (a
 * local index; the matching words show in context). Ended and Archived
 * are separate lists. Replay, Archive and Delete… are in the row's ⋯
 * menu (and on right-click), with every other item for what it is;
 * Delete is its own confirmed dialog naming what goes and what stays.
 */
import { showToast } from "../components/Toast";
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { ActionHostProvider, useActionHost } from "../components/ItemMenu";
import type { ActionHost } from "../lib/actions";
import { ReplayDialog } from "../components/ReplayDialog";
import { DeleteInstanceDialog } from "../components/DeleteInstanceDialog";
import { PagedSessions, useSessionRows, type Row } from "../pages/SessionsList";
import { sessionsQuery, type SessionsCountsInfo } from "../lib/ipc/sessions";
import { refreshPaged } from "../lib/paged";
import { historyKey } from "../lib/prefetch-data";
import { providerDisplayName } from "../lib/providers";
import { outsideSearchStatus } from "../lib/ipc/outsideSearch";
import {
  errorText,
  hasLiveBackend,
  instanceArchive,
  instanceUnarchive,
  transcriptSearch,
  type TranscriptHitInfo,
  type InstanceInfo,
  type ProjectInfo,
} from "../lib/bridge";

type ListId = "ended" | "archived";

function localDate(ms: number): string {
  const at = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
}

/** Does every word of `query` match one of these fields? */
function matchesWords(fields: (string | null | undefined)[], query: string): boolean {
  const text = fields.filter(Boolean).join(" ").toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => text.includes(word));
}

/** Does every word of `query` match one of the Instance's fields? */
export function matchesFields(session: InstanceInfo, project: string, query: string): boolean {
  return matchesWords(
    [
      session.label,
      session.title,
      project,
      session.account_label,
      session.provider,
      providerDisplayName(session.provider),
      session.model,
      session.git_branch,
      localDate(session.started_at_ms),
      session.last_activity_ms ? localDate(session.last_activity_ms) : null,
    ],
    query,
  );
}

/** A row's fields for the search: the session's, or the conversation's. */
function rowMatches(row: Row, project: string, query: string): boolean {
  if (row.item.kind !== "conversation") return matchesFields(row.item.session, project, query);
  const c = row.item.conversation;
  return matchesWords(
    [
      row.title,
      project,
      c.account_label,
      c.tool,
      providerDisplayName(c.tool),
      c.git_branch,
      c.started_ms ? localDate(c.started_ms) : null,
      c.last_activity_ms ? localDate(c.last_activity_ms) : null,
    ],
    query,
  );
}

/** A hit's key: its session, or (AJ-E1538) an outside conversation's
 * account and id, as the rows name them. */
export function hitKey(hit: TranscriptHitInfo): string {
  return hit.session_id ?? `${hit.account_id ?? ""}:${hit.conversation_id ?? ""}`;
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
  const live = hasLiveBackend();
  const [list, setList] = useState<ListId>("ended");
  const [query, setQuery] = useState("");
  // The backend's search waits for typing to pause; the preview's is instant.
  const [searched, setSearched] = useState("");
  const [snippets, setSnippets] = useState<Map<string, string>>(new Map());
  const [searching, setSearching] = useState(false);
  const [found, setFound] = useState<number | null>(null);
  const [counts, setCounts] = useState<SessionsCountsInfo | null>(null);
  // AJ-E1538: whether outside sessions' words are searched (opt-in).
  const [everyWord, setEveryWord] = useState(false);
  useEffect(() => void outsideSearchStatus().then((info) => setEveryWord(info.enabled), () => null), []);
  const [replay, setReplay] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const names = new Map(projects.map((project) => [project.id, project.name]));
  const projectName = (id: string | null) => (id ? (names.get(id) ?? "") : "");
  const host = useActionHost();
  const openSession = (id: string) => host.openSessionById?.(id);
  const all = useSessionRows(live ? [] : sessions, {}, openSession, Date.now());

  useEffect(() => searchRef.current?.focus(), []);
  useEffect(() => {
    const trimmed = query.trim();
    if (!live) {
      setSearched(trimmed);
      return;
    }
    setSearching(trimmed.length >= 2);
    const timer = window.setTimeout(() => setSearched(trimmed), 250);
    return () => window.clearTimeout(timer);
  }, [query, live]);
  // The preview's transcript words (the backend searches them itself).
  useEffect(() => {
    if (live) return;
    const trimmed = query.trim();
    if (trimmed.length < 2) {
      setSnippets(new Map());
      return;
    }
    let alive = true;
    setSearching(true);
    const timer = window.setTimeout(() => {
      transcriptSearch(trimmed)
        .then((hits) => alive && setSnippets(new Map(hits.map((hit) => [hitKey(hit), hit.snippet]))))
        .catch(() => alive && setSnippets(new Map()))
        .finally(() => alive && setSearching(false));
    }, 250);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [query, live]);

  const archived = (session: InstanceInfo) => session.lifecycle === "archived";
  const sessionOf = (row: Row) => (row.item.kind === "conversation" ? null : row.item.session);
  const snippetOf = (row: Row) =>
    row.snippet ??
    (row.item.kind === "conversation"
      ? snippets.get(`${row.item.conversation.account_id}:${row.item.conversation.id}`)
      : snippets.get(row.item.session.id));
  // The design preview's rows (no backend): narrowed here, as before.
  const localRows = () =>
    all
      .filter((row) => !row.live)
      .filter((row) => {
        const session = sessionOf(row);
        // Archiving is AI Juggler's: a conversation started outside it is
        // only ever under Ended.
        return list === "archived" ? session !== null && archived(session) : session === null || !archived(session);
      })
      .filter((row) => !searched || snippetOf(row) !== undefined || rowMatches(row, projectName(row.projectId), searched))
      .map((row) => ({ ...row, snippet: snippetOf(row) ?? null }));
  const archivedCount = live ? (counts?.archived ?? 0) : sessions.filter(archived).length;

  // What the preview pages changes with its inputs; the backend's list is
  // read again when a session ends, is archived or goes.
  const version = useMemo(
    () => (live ? "" : `${sessions.length}:${all.length}:${snippets.size}:${sessions.map((s) => `${s.id}${s.status}${s.lifecycle ?? ""}`).join(",")}`),
    [live, sessions, all.length, snippets],
  );
  const changes = useMemo(() => sessions.map((s) => `${s.id}:${s.status}:${s.lifecycle ?? ""}`).join(","), [sessions]);
  useEffect(() => {
    if (live) refreshPaged("history:");
  }, [live, changes]);
  const changed = () => {
    refreshPaged("history:");
    onChanged();
  };

  // AJ-E1503: archiving answers in a toast, with Undo (the other way).
  const act = async (session: InstanceInfo, run: () => Promise<unknown>, done: string, undo: () => Promise<unknown>) => {
    const label = session.project_id ? `${projectName(session.project_id)} · ${session.account_label}` : session.account_label;
    try {
      await run();
      showToast({
        id: `archive:${session.id}`,
        message: `${label}: ${done}.`,
        undo: {
          onUndo: async () => {
            await undo();
            changed();
          },
        },
      });
      changed();
    } catch (cause) {
      showToast({ tone: "warning", message: `${label}: ${errorText(cause)}`, seconds: 8 });
    }
  };

  // AJ-E1537: what History gives each row's menu.
  const historyHost: ActionHost = {
    replay: (session) => setReplay(session.id),
    archive: (session) => act(session, () => instanceArchive(session.id), "archived — nothing deleted", () => instanceUnarchive(session.id)),
    unarchive: (session) => act(session, () => instanceUnarchive(session.id), "unarchived", () => instanceArchive(session.id)),
    isArchived: archived,
    deleteHistory: (session) => setDeleting(session.id),
  };

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-4" data-testid="history">
      <header className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold tracking-tight">History</h2>
        <p className="text-xs text-muted" data-testid="history-scope">
          Every session that has ended, AI Juggler's and the ones started outside it. Search by what it was about,
          project, account, tool, model, branch or date (2026-10-01), or by what was said in it.
          {everyWord
            ? " Words are searched in every session."
            : " Words are searched in AI Juggler's sessions; for the ones started outside it, turn on Search the words of every session in Settings › Privacy."}
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
        <div role="tablist" aria-label="Which sessions" className="flex rounded-lg border border-hairline p-0.5">
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
                "rounded-md px-2.5 py-1 text-label font-medium " +
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
        {searching ? "Searching transcripts…" : found === null ? "" : `${found} found`}
      </p>
      <ActionHostProvider host={historyHost}>
        <PagedSessions
          listKey={historyKey(list, searched, version)}
          query={sessionsQuery({ which: list, text: searched || null })}
          local={localRows}
          onOpenSession={openSession}
          projects={projects}
          onCounts={setCounts}
          onTotal={(total) => {
            setFound(total);
            if (live) setSearching(false);
          }}
          empty={
            query.trim()
              ? `Nothing matches “${query.trim()}”${searching ? " yet — still searching transcripts" : ""}.${everyWord ? "" : " Words said in sessions started outside AI Juggler aren't searched (Settings › Privacy)."}`
              : list === "archived"
                ? "Nothing archived. Archive an ended session to tidy the rail — nothing is deleted."
                : "No ended sessions yet."
          }
          badge={(row) => {
            const session = sessionOf(row);
            return session && archived(session) ? (
              <span className="shrink-0 rounded-full border border-hairline px-2 py-0.5 text-label text-muted">Archived</span>
            ) : null;
          }}
          below={(row) => {
            const snippet = snippetOf(row);
            return snippet ? (
              <p className="mx-4 mb-2 rounded bg-surface-2/60 px-2 py-1 font-mono text-label text-ink" data-testid="history-snippet">
                {snippet}
              </p>
            ) : null;
          }}
        />
      </ActionHostProvider>
      {replay && <ReplayDialog sessionId={replay} onClose={() => setReplay(null)} />}
      {deleting && (
        <DeleteInstanceDialog
          sessionId={deleting}
          onClose={(deleted) => {
            setDeleting(null);
            if (deleted) {
              showToast({ message: "Deleted." });
              changed();
            }
          }}
        />
      )}
    </div>
  );
}

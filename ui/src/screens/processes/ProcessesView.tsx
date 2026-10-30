/**
 * AJ-E2404 — the Processes view: the one component the Processes page and
 * a session's Processes tab both show (the tab with `sessionId`, for any
 * app's session). Its views, search and filters, the tree table, and the
 * sheets its actions open: the verify sheet for a stop (AI Juggler's own
 * through the ledger, another app's leftover through its sighting), and
 * "Clean up what this session left".
 */
import { useCallback, useEffect, useMemo, useState, type JSX } from "react";
import { KillSheet, type KillBridge } from "../../components/KillSheet";
import { killCancel, killLiveness } from "../../lib/bridge";
import { conversationsGoTo } from "../../lib/ipc/conversations";
import {
  processesActivityMonitor,
  processesMark,
  processesObservedStop,
  processesObservedVerify,
  processesPorts,
  processesReveal,
  processQuery,
  type ProcessGraphRowInfo,
} from "../../lib/ipc/processes";
import { ZOMBIE_WORDS } from "../../lib/process-states";
import { PortInventoryList, type PortsBridge } from "../../components/PortMap";
import { CleanupStatus } from "./CleanupStatus";
import { GroupCleanupSheet } from "./GroupCleanupSheet";
import { HistoryView } from "./HistoryView";
import { ProcessTree } from "./ProcessTree";
import { clearProcess, onProcessAsk, openSubagents, peekProcess } from "../../lib/subagent-links";
import { RowActions, canStop, isSessionItself, type RowHandlers } from "./RowActions";
import { FiltersButton, NO_FILTERS, SearchBox, ViewTabs, filterCount, type Choice, type Filters, type ViewId } from "./Toolbar";
import { useProcessRows } from "./useProcessRows";

/** The Ports view reads the port inventory (the made-up one in the preview). */
const PORTS: PortsBridge = { load: processesPorts };
const VIEW_KEY = "aij.processes.view";

function savedView(): ViewId {
  try {
    const saved = localStorage.getItem(VIEW_KEY);
    return saved === "running" || saved === "ended" || saved === "ports" || saved === "history" ? saved : "attention";
  } catch {
    return "attention";
  }
}

/** The stop sheet's calls for another app's leftover (D-41b). */
function observedBridge(startTime: number | null): KillBridge {
  return {
    verify: (_session, pid) => processesObservedVerify(pid, startTime),
    execute: (_session, pid, _scope, attempt) => processesObservedStop(pid, startTime, attempt),
    cancel: (attempt) => killCancel(attempt),
    liveness: (pids) => killLiveness(pids),
  };
}

function useDebounced(value: string, ms: number): string {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), ms);
    return () => window.clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

export function ProcessesView({
  sessionId = null,
  onOpenSession,
  onOpenConversation,
  accounts = [],
  projects = [],
  height,
  testId = "processes-view",
}: {
  /** Only this session (a session's Processes tab). */
  sessionId?: string | null;
  onOpenSession?: (id: string) => void;
  onOpenConversation?: (accountId: string, id: string) => void;
  accounts?: Choice[];
  projects?: Choice[];
  height?: string;
  testId?: string;
}): JSX.Element {
  const embedded = sessionId !== null;
  const [view, setView] = useState<ViewId>(() => (embedded ? "running" : savedView()));
  const [search, setSearch] = useState("");
  /** An alert's "Show it": the process to focus once its row shows. */
  const [finding, setFinding] = useState<number | null>(null);
  const [filters, setFilters] = useState<Filters>(NO_FILTERS);
  const [expanded, setExpanded] = useState<string[]>([]);
  const [collapsed, setCollapsed] = useState<string[]>([]);
  const [stopping, setStopping] = useState<{ row: ProcessGraphRowInfo; trigger: HTMLElement | null } | null>(null);
  const [cleaning, setCleaning] = useState<ProcessGraphRowInfo | null>(null);
  // AJ-E1520 (0.3): a subagent's process, asked for from its page.
  const [selected, setSelected] = useState<string | null>(() => (embedded ? peekProcess() : null));
  useEffect(() => {
    if (!embedded) return;
    return onProcessAsk(() => {
      setView("running");
      setSelected(peekProcess());
    });
  }, [embedded]);
  const clearSelected = useCallback(() => {
    clearProcess();
    setSelected(null);
  }, []);
  const words = useDebounced(search, 150);

  const query = useMemo(
    () =>
      processQuery({
        view: view === "history" || view === "ports" ? "attention" : view,
        session_id: sessionId,
        states: filters.states,
        app: filters.app,
        account_id: filters.account,
        project_id: filters.project,
        has_port: filters.hasPort,
        heavy: filters.heavy,
        show_internal: filters.showInternal,
        others: filters.others,
        search: words,
        expanded,
        collapsed,
      }),
    [view, sessionId, filters, words, expanded, collapsed],
  );
  const { rows, total, counts, degraded, loading, current, error, hasMore, loadMore, refresh } = useProcessRows(query);

  const chooseView = (next: ViewId) => {
    setView(next);
    if (!embedded) {
      try {
        localStorage.setItem(VIEW_KEY, next);
      } catch {
        // A remembered tab is a nicety.
      }
    }
  };

  const apps: Choice[] = useMemo(() => rows.filter((row) => row.app).map((row) => ({ id: row.key, label: row.app!.name })), [rows]);
  const isOpen = useCallback((row: ProcessGraphRowInfo) => !row.session?.folded, []);
  const onToggle = useCallback((row: ProcessGraphRowInfo, open: boolean) => {
    if (row.kind !== "session") return;
    setExpanded((now) => (open ? [...now.filter((key) => key !== row.key), row.key] : now.filter((key) => key !== row.key)));
    setCollapsed((now) => (open ? now.filter((key) => key !== row.key) : [...now.filter((key) => key !== row.key), row.key]));
  }, []);

  const sessionOf = (row: ProcessGraphRowInfo) => rows.find((other) => other.key === row.session_key)?.session ?? row.session;
  // "Show it" takes the keyboard to the found row (its Enter is Stop…).
  useEffect(() => {
    // Once the search for it has been read (the view changes under it).
    if (finding === null || !current || words !== `pid:${finding}`) return;
    const row = document.querySelector<HTMLElement>(`[data-testid^="process-row-p:${finding}:"]`);
    if (!row) return;
    row.focus();
    setFinding(null);
  }, [finding, rows, current, words]);

  const handlers: RowHandlers = {
    onSessionPage: embedded,
    stop: (row) => setStopping({ row, trigger: document.activeElement as HTMLElement | null }),
    mark: (row, mark) => void processesMark(row.process!.pid, row.process!.start_time, mark, mark === "snooze" ? 60 : undefined).then(refresh),
    goToSession: (row) => {
      const session = sessionOf(row);
      if (session && row.origin === "ours") onOpenSession?.(session.session_id);
      else if (session?.account_id) onOpenConversation?.(session.account_id, session.session_id);
    },
    goToApp: (row) => {
      const session = sessionOf(row);
      if (session?.account_id) void conversationsGoTo(session.account_id, session.session_id, session.root_pid);
      else void processesActivityMonitor();
    },
    reveal: (row) => void processesReveal(row.session_key ?? row.key),
    activityMonitor: () => void processesActivityMonitor(),
    copyPid: (pid) => void navigator.clipboard?.writeText(String(pid)),
    cleanSession: (row) => setCleaning(row),
  };

  const stopRow = stopping?.row;
  const stopSession = stopRow ? sessionOf(stopRow) : null;
  // Explained when one shows (its row is listed), not when one is elsewhere.
  const zombies = rows.filter((row) => row.state === "zombie").length;
  const empty =
    view === "attention"
      ? "Nothing needs you: no process is left behind, stuck stopping or waiting on its parent."
      : filterCount(filters) > 0 || words
        ? "No process matches. Clear the search or the filters."
        : "No sessions are running processes now.";
  return (
    <div className="flex min-w-0 flex-col gap-3" data-testid={testId}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <ViewTabs view={view} counts={embedded ? null : counts} onView={chooseView} embedded={embedded} />
        {view !== "history" && view !== "ports" && (
          <div className="flex flex-wrap items-center gap-2">
            <SearchBox value={search} onChange={setSearch} />
            <FiltersButton filters={filters} onChange={setFilters} apps={apps} accounts={accounts} projects={projects} />
          </div>
        )}
      </div>
      {!embedded && (
        <CleanupStatus
          degraded={degraded}
          onFind={(pid) => {
            chooseView("running");
            setSearch(`pid:${pid}`);
            setFinding(pid);
          }}
        />
      )}
      {zombies > 0 && (view === "attention" || view === "running") && (
        <p className="text-meta text-muted" data-testid="zombie-words">
          <span className="font-medium text-ink">Waiting for its parent</span> means: {ZOMBIE_WORDS}
        </p>
      )}
      {error && rows.length === 0 && <p className="text-body text-danger">Couldn't read the processes: {error}</p>}
      {view === "ports" ? (
        <section className="flex flex-col gap-2" aria-label="Ports on this computer" data-testid="processes-ports">
          <p className="text-meta text-muted">Every port open on this computer: what each is, and whether a session left it running.</p>
          <div className="rounded-card border border-hairline bg-surface px-4 py-3">
            <PortInventoryList bridge={PORTS} />
          </div>
        </section>
      ) : view === "history" ? (
        <HistoryView sessionId={sessionId} />
      ) : loading ? (
        <div className="h-64 animate-pulse rounded-card bg-surface-2" aria-label="Loading the processes" data-testid="processes-loading" />
      ) : (
        <ProcessTree
          rows={rows}
          total={total}
          label={view === "attention" ? "Processes that need you" : view === "ended" ? "Processes that ended" : "Processes running"}
          ended={view === "ended"}
          isOpen={isOpen}
          onToggle={onToggle}
          actions={(row) => <RowActions row={row} on={handlers} />}
          onActivate={(row) =>
            canStop(row) ? handlers.stop(row) : isSessionItself(row) && !embedded ? handlers.goToSession(row) : row.kind === "session" ? onToggle(row, !isOpen(row)) : undefined
          }
          hasMore={hasMore}
          onNearEnd={loadMore}
          height={height}
          empty={empty}
          onSubagent={embedded ? (id) => openSubagents({ subagent: id }) : undefined}
          select={embedded ? selected : null}
          onSelected={clearSelected}
        />
      )}
      {stopRow && stopRow.process && (
        <KillSheet
          session={stopSession?.session_id ?? ""}
          where={[stopSession?.title, stopSession?.folder_name, stopRow.owner_words].filter(Boolean).join(" · ")}
          pid={stopRow.process.pid}
          goalPort={stopRow.ports[0] ?? null}
          trigger={stopping.trigger ?? undefined}
          bridge={stopRow.process.stop === "observed" ? observedBridge(stopRow.process.start_time) : undefined}
          onlyThisProcess={stopRow.process.stop === "observed" ? "Only this process: the app's session and its other processes are never touched." : undefined}
          observed={stopRow.process.stop === "observed"}
          sessionEnded={stopRow.state === "left_behind" || stopRow.state === "left_behind_closed" || stopSession?.state === "ended"}
          onShowHistory={() => {
            setStopping(null);
            refresh();
            chooseView("history");
          }}
          onClose={() => {
            setStopping(null);
            refresh();
          }}
        />
      )}
      {cleaning && (
        <GroupCleanupSheet
          session={cleaning}
          onClose={() => {
            setCleaning(null);
            refresh();
          }}
        />
      )}
    </div>
  );
}

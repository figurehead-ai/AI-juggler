import { useEffect, useMemo, useRef, useState } from "react";
import { KillSheet, type KillBridge } from "../components/KillSheet";
import { Button, CleanupEmpty, ProcessRow, type CleanupRowInfo } from "../components/ProcessRow";
import { errorText } from "../lib/bridge";

/**
 * AJ-E506/E507: the global Cleanup Centre — the read-only inventory
 * (Tier 0, D-08 default: what Instances left behind, what it costs,
 * and WHY Juggler thinks it is theirs) plus the ONE manual kill path:
 * verified rows offer `Terminate…`, which opens the staged
 * AJ-E507 sheet (verify → scope → consequence-named confirm). No
 * automatic signalling exists here (that is AJ-E508's tiers, opt-in
 * per Account); Unverifiable rows offer nothing at all.
 *
 * Reached from the top-right global menus (ADR-0007); ⌘⇧C opens it,
 * ⌘F focuses the filter. Healthy trees collapse by default so
 * attention lands on orphans (Apple HIG clarity/deference).
 */

export interface CleanupGroupInfo {
  session: string;
  provider: string;
  account: string | null;
  project: string | null;
  machine: string;
  healthy: boolean;
  rows: CleanupRowInfo[];
}

export interface AuditHistoryRowInfo {
  id: number;
  at_ms: number;
  actor: string;
  automatic: boolean;
  pid: number;
  cmdline_trunc: string;
  session_id: string;
  provider: string;
  account_id: string | null;
  machine_id: string;
  outcome: string;
  ports_before: string | null;
  ports_after: string | null;
}

export interface CleanupHistoryData {
  rows: AuditHistoryRowInfo[];
  empty_note: string | null;
}

export interface CleanupInventoryData {
  groups: CleanupGroupInfo[];
  /** AJ-E511: the executing node's machine id — rows whose
   * machine_id equals it are This Mac; others are display-only. */
  local_machine: string;
  clean_instances: number;
  ledger_degraded: boolean;
  history: CleanupHistoryData | null;
}

export const CLEANUP_FILTERS = [
  { id: "needs-attention", label: "Needs attention" },
  { id: "all-managed", label: "All managed" },
  { id: "re-adopted", label: "Re-adopted" },
  { id: "history", label: "History (7 days)" },
] as const;

export type CleanupFilterId = (typeof CLEANUP_FILTERS)[number]["id"];

export type CleanupBridge = {
  load: (
    filter: string,
    machineScope: "this" | "all",
  ) => Promise<CleanupInventoryData>;
};

/** AJ-E512: the passive alerts source (hog alerts + idle nudges) —
 * injectable for tests, defaulting to the bridge. */
export type CleanupAlertsBridge = {
  load: () => Promise<import("../lib/bridge").CleanupAlertsData>;
};

// Dynamic import (the vi.mock TDZ rule).
export const defaultCleanupAlertsBridge: CleanupAlertsBridge = {
  load: () => import("../lib/bridge").then((bridge) => bridge.cleanupAlerts()),
};

/** AJ-E508: the automation status surface (pause, suspension, the
 * Tier-2 Undo windows) — injectable for tests. */
export type CleanupStatusBridge = {
  status: () => Promise<import("../lib/bridge").CleanupStatusData>;
  pause: (paused: boolean) => Promise<import("../lib/bridge").CleanupStatusData>;
  resume: () => Promise<import("../lib/bridge").CleanupStatusData>;
  tier2Decide: (pid: number, keep: boolean) => Promise<import("../lib/bridge").CleanupStatusData>;
};

// Dynamic import (the vi.mock TDZ rule).
export const defaultCleanupStatusBridge: CleanupStatusBridge = {
  status: () => import("../lib/bridge").then((bridge) => bridge.cleanupStatus()),
  pause: (paused) => import("../lib/bridge").then((bridge) => bridge.cleanupPauseSet(paused)),
  resume: () => import("../lib/bridge").then((bridge) => bridge.cleanupResumeAfterCrash()),
  tier2Decide: (pid, keep) =>
    import("../lib/bridge").then((bridge) => bridge.cleanupTier2Decide(pid, keep)),
};

// Dynamic import (the vi.mock TDZ rule).
export const defaultCleanupBridge: CleanupBridge = {
  load: (filter, machineScope) =>
    import("../lib/bridge").then((bridge) =>
      bridge.cleanupInventory(filter, machineScope),
    ),
};

function GroupBlock(props: {
  group: CleanupGroupInfo;
  /** AJ-E511: this node's machine id — foreign rows are display-only. */
  localMachine?: string;
  onKill?: (row: CleanupRowInfo, session: string, trigger: HTMLElement) => void;
}) {
  const group = props.group;
  const [collapsed, setCollapsed] = useState(group.healthy);
  return (
    <section
      className="flex flex-col rounded-xl border border-hairline bg-panel p-3"
      data-testid={`cleanup-group-${group.session}`}
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => setCollapsed((open) => !open)}
          aria-expanded={!collapsed}
          className="flex min-w-0 flex-1 items-center gap-2 rounded-md px-1 py-0.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <span aria-hidden="true" className="w-3 text-center text-[9px] text-muted">
            {collapsed ? "▸" : "▾"}
          </span>
          <span className="truncate text-[11.5px] font-medium text-ink">
            {group.provider}
            {group.account !== null ? ` · ${group.account}` : ""}
            {group.project !== null ? ` · ${group.project}` : ""}
          </span>
          <span className="font-mono text-[9.5px] text-faint">
            {group.machine} · {group.rows.length}{" "}
            {group.rows.length === 1 ? "process" : "processes"}
          </span>
          {group.healthy && (
            <span className="ml-auto rounded-full border border-hairline px-1.5 py-[1px] text-[9px] text-muted">
              healthy
            </span>
          )}
        </button>
      </div>
      {!collapsed && (
        <ul className="mt-1">
          {group.rows.map((row) => (
            <ProcessRow
              key={row.pid}
              row={row}
              localMachine={props.localMachine}
              onKill={
                // AJ-E511: foreign-machine rows are display-only —
                // never even an affordance. Rows WITHOUT a machine id
                // are legacy-local: the backend stamps every row now.
                props.onKill !== undefined &&
                !(
                  row.machine_id !== undefined &&
                  props.localMachine !== undefined &&
                  row.machine_id !== props.localMachine
                )
                  ? (row, trigger) => props.onKill?.(row, group.session, trigger)
                  : undefined
              }
            />
          ))}
        </ul>
      )}
    </section>
  );
}

export function CleanupCentre(props: {
  bridge?: CleanupBridge;
  killBridge?: KillBridge;
  statusBridge?: CleanupStatusBridge;
  alertsBridge?: CleanupAlertsBridge;
}) {
  const bridge = props.bridge ?? defaultCleanupBridge;
  const killBridge = props.killBridge;
  const alertsBridge = props.alertsBridge ?? defaultCleanupAlertsBridge;
  const statusBridge = props.statusBridge ?? defaultCleanupStatusBridge;
  const [filter, setFilter] = useState<CleanupFilterId>("needs-attention");
  // AJ-E511: the machine dimension — "This machine" by default so
  // single-machine users see no fleet complexity.
  const [machineScope, setMachineScope] = useState<"this" | "all">("this");
  const [data, setData] = useState<CleanupInventoryData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [killTarget, setKillTarget] = useState<{
    session: string;
    pid: number;
    goalPort: number | null;
    trigger: HTMLElement;
  } | null>(null);
  const filterRef = useRef<HTMLDivElement>(null);
  const [reloadKey, setReloadKey] = useState(0);
  // AJ-E508: the automation status (pause kill-switch, the crash
  // suspension, the Tier-2 Undo windows) — loaded on mount, after
  // every decision, and polled while a Tier-2 window is open.
  const [status, setStatus] = useState<import("../lib/bridge").CleanupStatusData | null>(null);
  useEffect(() => {
    let alive = true;
    statusBridge
      .status()
      .then((result) => {
        if (alive) setStatus(result);
      })
      .catch(() => {
        /* the banners stay hidden on failure — never a false pause claim */
      });
    return () => {
      alive = false;
    };
  }, [statusBridge, reloadKey]);
  useEffect(() => {
    if ((status?.tier2_pending.length ?? 0) === 0) return;
    const timer = setInterval(() => {
      void statusBridge
        .status()
        .then((next) => setStatus(next))
        .catch(() => undefined);
    }, 1_000);
    return () => clearInterval(timer);
  }, [status?.tier2_pending.length, statusBridge]);

  useEffect(() => {
    let alive = true;
    bridge
      .load(filter, machineScope)
      .then((result) => {
        if (alive) {
          setData(result);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (alive) setError(errorText(cause));
      });
    return () => {
      alive = false;
    };
  }, [filter, machineScope, bridge, reloadKey]);

  // AJ-E512 Req.3: the passive alerts (hog alerts + idle nudges) —
  // `role=status` cards, never dialogs; a hog alert's only action is
  // STAGING the verified sheet (app prepares, user executes).
  const [alerts, setAlerts] = useState<
    import("../lib/bridge").CleanupAlertData[] | null
  >(null);
  useEffect(() => {
    let alive = true;
    alertsBridge
      .load()
      .then((result) => {
        if (alive) setAlerts(result.alerts);
      })
      .catch(() => {
        /* alerts stay hidden on failure — never a false claim */
      });
    return () => {
      alive = false;
    };
  }, [alertsBridge, reloadKey]);

  const visibleGroups = useMemo(() => data?.groups ?? [], [data]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3" data-testid="cleanup-centre">
      <div className="flex items-center gap-2">
        <h2 className="text-sm font-semibold tracking-wide">Cleanup Centre</h2>
        <span className="rounded-full border border-hairline px-1.5 py-[1px] text-[9.5px] text-muted">
          Tier 0 observe · verified manual kills
        </span>
        {/* The global kill-switch lives IN the header (Req.4). */}
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            void statusBridge
              .pause(!(status?.paused ?? false))
              .then(setStatus)
              .catch(() => undefined);
          }}
          data-testid="cleanup-pause-toggle"
        >
          {status?.paused ? "Resume automatic cleanup" : "Pause automatic cleanup"}
        </Button>
        {/* ⌘F lands here (2.1.1/2.4.7 keyboard-complete + visible). */}
        <div
          ref={filterRef}
          role="radiogroup"
          aria-label="Cleanup filter"
          className="ml-auto flex items-center rounded-lg border border-hairline bg-surface/60 p-0.5"
        >
          {CLEANUP_FILTERS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              role="radio"
              aria-checked={filter === entry.id}
              onClick={() => setFilter(entry.id)}
              className={
                "rounded-md px-2 py-1 text-[10.5px] font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (filter === entry.id
                  ? "bg-accent-soft text-accent-hi"
                  : "text-muted hover:bg-surface hover:text-ink")
              }
            >
              {entry.label}
            </button>
          ))}
        </div>
        <label className="ml-2 flex items-center gap-1 text-[10.5px] text-muted">
          Machine
          <select
            aria-label="Machine filter"
            value={machineScope}
            onChange={(event) =>
              setMachineScope(event.target.value === "all" ? "all" : "this")
            }
            className="rounded-md border border-hairline bg-surface px-1.5 py-1 text-[10.5px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
            data-testid="cleanup-machine-filter"
          >
            <option value="this">This machine</option>
            <option value="all">All machines</option>
          </select>
        </label>
      </div>

      {/* Filter changes announced politely (4.1.3). */}
      <p aria-live="polite" className="sr-only" data-testid="cleanup-filter-announcement">
        {`Cleanup filter: ${
          CLEANUP_FILTERS.find((entry) => entry.id === filter)?.label ?? filter
        } — ${visibleGroups.length} Instance ${
          visibleGroups.length === 1 ? "group" : "groups"
        }`}
      </p>

      {/* Req.4/5: the persistent banners — shown, never silent. */}
      {status?.paused && (
        <p
          className="rounded-lg border border-warning/40 bg-warning/5 p-2 text-[11px] text-ink"
          role="status"
          data-testid="cleanup-paused-banner"
        >
          Automatic cleanup is paused.
        </p>
      )}
      {status?.suspended && (
        <p
          className="flex items-center gap-2 rounded-lg border border-warning/40 bg-warning/5 p-2 text-[11px] text-ink"
          role="status"
          data-testid="cleanup-suspended-banner"
        >
          <span>
            Automatic cleanup is suspended for 24 h after the app closed unexpectedly —
            re-confirm to resume.
          </span>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              void statusBridge
                .resume()
                .then(setStatus)
                .catch(() => undefined);
            }}
            data-testid="cleanup-resume-button"
          >
            Re-confirm
          </Button>
        </p>
      )}
      {status !== null && status.tier2_pending.length > 0 && (
        <Tier2Notices
          status={status}
          onDecide={(pid, keep) => {
            void statusBridge
              .tier2Decide(pid, keep)
              .then(setStatus)
              .catch(() => undefined);
          }}
        />
      )}
      {data?.ledger_degraded && (
        <p
          className="rounded-lg border border-warning/40 bg-warning/5 p-2 text-[11px] text-ink"
          role="alert"
          data-testid="cleanup-degraded-banner"
        >
          Process tracking degraded — cleanup is read-only until restart.
        </p>
      )}

      {/* AJ-E512 Req.3: the passive alerts — status messages, never
       * focus-stealing dialogs. A hog alert's only affordance STAGES
       * the verified sheet for its helper (app prepares, user
       * executes); idle nudges carry no action at all (once-only by
       * the backend's persisted flag). */}
      {(alerts?.length ?? 0) > 0 && (
        <section
          role="status"
          aria-live="polite"
          aria-label="Passive cleanup alerts"
          className="flex flex-col gap-1.5"
          data-testid="cleanup-alerts"
        >
          {alerts?.map((alert) =>
            alert.kind === "hog" ? (
              <p
                key={`hog-${alert.pid}`}
                className="flex items-center gap-2 rounded-lg border border-hairline bg-surface/60 p-2 text-[11px] text-ink"
                data-testid={`hog-alert-${alert.pid}`}
              >
                <span aria-hidden="true" className="text-warning">
                  ▲
                </span>
                <span className="min-w-0 flex-1">{alert.text}</span>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={(event) => {
                    // Stage the sheet for the named helper — the row
                    // must be in the loaded inventory (killable only
                    // through the same verified path as every row).
                    const target = (data?.groups ?? [])
                      .flatMap((group) =>
                        group.rows.map((row) => ({ row, session: group.session })),
                      )
                      .find(({ row }) => row.pid === alert.pid);
                    if (!target) return;
                    setKillTarget({
                      session: target.session,
                      pid: target.row.pid,
                      goalPort:
                        target.row.ports !== null &&
                        target.row.ports.split(",").length === 1
                          ? Number(target.row.ports)
                          : null,
                      trigger: event.currentTarget,
                    });
                  }}
                  data-testid={`hog-stage-${alert.pid}`}
                >
                  Stage Terminate…
                </Button>
              </p>
            ) : (
              <p
                key={`idle-${alert.pid}`}
                className="rounded-lg border border-hairline bg-surface/40 p-2 text-[11px] text-muted"
                data-testid={`idle-nudge-${alert.pid}`}
              >
                <span aria-hidden="true" className="mr-1">
                  ○
                </span>
                {alert.text}
              </p>
            ),
          )}
        </section>
      )}

      {error !== null && (
        <p className="text-[11px] text-muted" role="alert" data-testid="cleanup-error">
          Cleanup inventory unavailable: {error}
        </p>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {(() => {
          const history = data?.history ?? null;
          return history !== null ? <HistoryList history={history} /> : null;
        })()}
        {data !== null && visibleGroups.length === 0 && error === null && (data?.history ?? null) === null && (
          <CleanupEmpty cleanInstances={data.clean_instances} />
        )}
        <div className="flex flex-col gap-2">
          {visibleGroups.map((group) => (
            <GroupBlock
              key={`${group.session}-${group.machine}`}
              localMachine={data?.local_machine}
              group={group}
              onKill={(row, session, trigger) =>
                setKillTarget({
                  session,
                  pid: row.pid,
                  goalPort:
                    row.ports !== null && row.ports.split(",").length === 1
                      ? Number(row.ports)
                      : null,
                  trigger,
                })
              }
            />
          ))}
        </div>
      </div>

      {killTarget !== null && (
        <KillSheet
          session={killTarget.session}
          pid={killTarget.pid}
          goalPort={killTarget.goalPort}
          trigger={killTarget.trigger}
          bridge={killBridge}
          onClose={(refresh) => {
            setKillTarget(null);
            if (refresh) setReloadKey((key) => key + 1);
          }}
        />
      )}

      <p className="text-[10px] text-faint" data-testid="cleanup-glossary">
        An <dfn>orphan</dfn> is a process still running after its Instance closed; a{" "}
        <dfn>helper</dfn> is any process an Instance spawned (dev server, watcher…).
      </p>
    </div>
  );
}

/** The audit history list (AJ-E509): the outcome vocabulary of the
 * ladders, manual/auto actors distinguished by icon+text. */
function HistoryList(props: { history: CleanupHistoryData }) {
  const [exported, setExported] = useState<string | null>(null);
  if (props.history.rows.length === 0) {
    return (
      <p className="text-[12px] text-muted" data-testid="cleanup-history-empty">
        {props.history.empty_note ?? "No cleanups yet — automatic cleanup is off by default."}
      </p>
    );
  }
  const doExport = () => {
    // User-initiated only; the JSONL download lands via a Blob anchor
    // (the standard save path in the webview).
    import("../lib/bridge")
      .then((bridge) => bridge.cleanupAuditExport())
      .then((jsonl) => {
        setExported(jsonl);
        const blob = new Blob([jsonl], { type: "application/jsonl" });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a");
        const date = new Date().toISOString().slice(0, 10);
        anchor.href = url;
        anchor.download = `juggler-cleanup-audit-${date}.jsonl`;
        anchor.click();
        URL.revokeObjectURL(url);
      })
      .catch(() => setExported(null));
  };
  return (
    <div className="flex flex-col gap-2" data-testid="cleanup-history">
      <div className="flex items-center gap-2">
        <h3 className="text-[11.5px] font-medium text-ink">History (7 days)</h3>
        <Button size="sm" variant="secondary" onClick={doExport}>
          Export audit log…
        </Button>
      </div>
      {exported !== null && (
        <p className="text-[10px] text-faint" data-testid="cleanup-history-exported">
          Exported {exported.split("\n").length} JSONL lines — command lines are hashed.
        </p>
      )}
      <ul className="flex flex-col">
        {props.history.rows.map((row) => (
          <li
            key={row.id}
            className="flex items-center gap-2 border-t border-hairline py-1.5 text-[11px]"
            data-testid={`cleanup-history-row-${row.id}`}
          >
            <span
              className={
                row.automatic
                  ? "inline-flex items-center gap-1 rounded-full border border-accent/40 px-1.5 py-[1px] text-[9.5px] text-accent"
                  : "inline-flex items-center gap-1 rounded-full border border-hairline px-1.5 py-[1px] text-[9.5px] text-muted"
              }
              data-testid={`cleanup-history-actor-${row.id}`}
            >
              {/* Icon+text, never colour alone (1.4.1). */}
              <span aria-hidden="true">{row.automatic ? "⚙" : "👤"}</span>
              {row.automatic ? "automatic" : "you"}
            </span>
            <span className="truncate font-mono text-[10.5px] text-ink">{row.cmdline_trunc}</span>
            <span className="font-mono text-[10px] text-muted">PID {row.pid}</span>
            <span className="ml-auto whitespace-nowrap text-[10px] text-muted">
              {row.outcome}
              {row.ports_before !== null && row.ports_after === null ? "" : ""}
            </span>
            <span className="whitespace-nowrap font-mono text-[9.5px] text-faint">
              {new Date(row.at_ms).toISOString().slice(5, 16).replace("T", " ")}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Focuses the filter group (the ⌘F target). */
export function focusCleanupFilter() {
  const group = document.querySelector<HTMLElement>('[role="radiogroup"][aria-label="Cleanup filter"]');
  const first = group?.querySelector<HTMLElement>("button");
  first?.focus();
}


/**
 * AJ-E508 Req.1: the Tier-2 crash notifications — the Undo window IS
 * the consent surface. One card per crash-orphan with a live
 * countdown (aria-live at 1 s), `Undo` / `Keep running` both stand
 * the candidate down.
 */
function Tier2Notices(props: {
  status: import("../lib/bridge").CleanupStatusData;
  onDecide: (pid: number, keep: boolean) => void;
}) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1_000);
    return () => clearInterval(timer);
  }, []);
  return (
    <ul className="flex flex-col gap-2" data-testid="cleanup-tier2-notices">
      {props.status.tier2_pending.map((notice) => {
        const secondsLeft = Math.max(0, Math.ceil((notice.deadline_ms - now) / 1000));
        return (
          <li
            key={notice.pid}
            className="flex flex-col gap-1 rounded-lg border border-warning/40 bg-warning/5 p-2 text-[11px] text-ink"
            data-testid={`cleanup-tier2-notice-${notice.pid}`}
          >
            <p>
              {notice.cmdline_trunc} (PID {notice.pid}) survived a crash — Juggler will tidy
              it{secondsLeft > 0 ? ` in ${secondsLeft}s` : " now"} unless you keep it.
            </p>
            <p aria-live="polite" className="sr-only" data-testid={`cleanup-tier2-countdown-${notice.pid}`}>
              {secondsLeft > 0
                ? `${secondsLeft} seconds until the automatic tidy`
                : "The undo window has closed"}
            </p>
            <span className="flex items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                onClick={() => props.onDecide(notice.pid, true)}
                data-testid={`cleanup-tier2-undo-${notice.pid}`}
              >
                Undo — keep it running
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => props.onDecide(notice.pid, true)}
                data-testid={`cleanup-tier2-keep-${notice.pid}`}
              >
                Keep running
              </Button>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

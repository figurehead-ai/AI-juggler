import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RemoteCleanup } from "../components/RemoteCleanup";
import { KillSheet, type KillBridge } from "../components/KillSheet";
import { Button, CleanupEmpty, ProcessRow, type CleanupRowInfo } from "../components/ProcessRow";
import { errorText, statsEvent } from "../lib/bridge";
import { providerDisplayName } from "../lib/providers";

/**
 * AJ-E506/E507: the global Cleanup screen (D-25: "Cleanup" on screen;
 * the tiers' names stay in Settings › Cleanup) — the read-only inventory
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
  { id: "re-adopted", label: "Picked up after restart" },
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

/** AJ-E511 / D-25: how many other Macs this one knows (paired, or
 * reporting) — the machine filter shows only when there is one. */
export type CleanupMachinesBridge = {
  others: () => Promise<number>;
};

// Dynamic import (the vi.mock TDZ rule).
export const defaultCleanupMachinesBridge: CleanupMachinesBridge = {
  others: () =>
    import("../lib/bridge").then(async (bridge) => {
      const [peers, remote] = await Promise.all([
        bridge.fleetPeers().catch(() => []),
        bridge.fleetRemote().catch(() => []),
      ]);
      return Math.max(peers.length, remote.length);
    }),
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
          <span aria-hidden="true" className="w-3 text-center text-label text-muted">
            {collapsed ? "▸" : "▾"}
          </span>
          <span className="truncate text-meta font-medium text-ink">
            {providerDisplayName(group.provider)}
            {group.account !== null ? ` · ${group.account}` : ""}
            {group.project !== null ? ` · ${group.project}` : ""}
          </span>
          <span className="font-mono text-label text-muted">
            {group.machine} · {group.rows.length}{" "}
            {group.rows.length === 1 ? "process" : "processes"}
          </span>
          {group.healthy && (
            <span className="ml-auto rounded-full border border-hairline px-1.5 py-[1px] text-label text-muted">
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
  machinesBridge?: CleanupMachinesBridge;
}) {
  const bridge = props.bridge ?? defaultCleanupBridge;
  const killBridge = props.killBridge;
  const alertsBridge = props.alertsBridge ?? defaultCleanupAlertsBridge;
  const statusBridge = props.statusBridge ?? defaultCleanupStatusBridge;
  const machinesBridge = props.machinesBridge ?? defaultCleanupMachinesBridge;
  const [otherMacs, setOtherMacs] = useState(false);
  useEffect(() => {
    let alive = true;
    machinesBridge
      .others()
      .then((count) => alive && setOtherMacs(count > 0))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [machinesBridge]);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const moreRef = useRef<HTMLButtonElement>(null);
  // Closing the menu from the keyboard (an item, Escape) — or a control
  // that unmounts itself (Resume on the banner) — hands focus back to ⋯,
  // never to <body> (WCAG 2.4.3).
  const closeMenu = useCallback((refocus: boolean) => {
    setMenuOpen(false);
    if (refocus) window.setTimeout(() => moreRef.current?.focus(), 0);
  }, []);
  // Opening it moves focus to its first item.
  useEffect(() => {
    if (menuOpen) menuRef.current?.querySelector<HTMLElement>("#cleanup-more button")?.focus();
  }, [menuOpen]);
  // The ⋯ menu closes on a click elsewhere or Escape, like the top bar's More.
  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) closeMenu(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeMenu(true);
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [menuOpen, closeMenu]);
  const [filter, setFilter] = useState<CleanupFilterId>("needs-attention");
  // AJ-E511: the machine dimension — "This machine" by default so
  // single-machine users see no fleet complexity.
  const [machineScope, setMachineScope] = useState<"this" | "all">("this");
  const [data, setData] = useState<CleanupInventoryData | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Re-confirm after a crash: why it didn't resume, if it didn't.
  const [resumeError, setResumeError] = useState<string | null>(null);
  const resuming = useRef(false);
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

  const togglePause = () => {
    closeMenu(true);
    void statusBridge
      .pause(!(status?.paused ?? false))
      .then(setStatus)
      .catch(() => undefined);
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3" data-testid="cleanup-centre">
      {/* D-25: the title and one plain line; the switches sit behind ⋯
          (the per-account tiers live in Settings › Cleanup). */}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <div className="flex min-w-0 flex-col">
          <h2 className="text-lg font-semibold tracking-tight">Cleanup</h2>
          <p className="text-xs text-muted" data-testid="cleanup-summary">
            What your sessions left running. You choose what to stop; automatic cleanup is off unless you
            turned it on.
          </p>
        </div>
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
                "rounded-md px-2 py-1 text-label font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (filter === entry.id
                  ? "bg-accent-soft text-accent-hi"
                  : "text-muted hover:bg-surface hover:text-ink")
              }
            >
              {entry.label}
            </button>
          ))}
        </div>
        {/* AJ-E511: the machine dimension appears once another Mac is
            paired (D-25: Fleet shows up when it has something). */}
        {(otherMacs || machineScope === "all") && (
          <label className="flex items-center gap-1 text-label text-muted">
            Machine
            <select
              aria-label="Machine filter"
              value={machineScope}
              onChange={(event) =>
                setMachineScope(event.target.value === "all" ? "all" : "this")
              }
              className="rounded-md border border-hairline bg-surface px-1.5 py-1 text-label text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
              data-testid="cleanup-machine-filter"
            >
              <option value="this">This machine</option>
              <option value="all">All machines</option>
            </select>
          </label>
        )}
        {/* AJ-E508 Req.4: the global kill-switch stays in the header,
            one step away behind ⋯; while paused, Resume is on the banner. */}
        <div className="relative" ref={menuRef}>
          <button
            type="button"
            aria-label="More cleanup options"
            aria-expanded={menuOpen}
            aria-controls="cleanup-more"
            ref={moreRef}
            onClick={() => setMenuOpen((open) => !open)}
            className="rounded-md px-2 py-1 text-muted outline-none hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            data-testid="cleanup-more"
          >
            ⋯
          </button>
          {menuOpen && (
            <div
              id="cleanup-more"
              className="absolute right-0 top-full z-40 mt-1.5 flex w-64 flex-col rounded-xl border border-hairline bg-surface p-1.5 shadow-xl"
            >
              {!status?.paused && (
                <button
                  type="button"
                  onClick={togglePause}
                  className="flex flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left hover:bg-surface-2"
                  data-testid="cleanup-pause-toggle"
                >
                  <span className="text-xs font-medium text-ink">Pause automatic cleanup</span>
                  <span className="text-label text-muted">For every account, until you resume it</span>
                </button>
              )}
              <button
                type="button"
                onClick={() => {
                  // Settings opens and takes focus at its Cleanup heading.
                  setMenuOpen(false);
                  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "cleanup" } }));
                }}
                className="flex flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left hover:bg-surface-2"
                data-testid="cleanup-open-settings"
              >
                <span className="text-xs font-medium text-ink">Cleanup settings…</span>
                <span className="text-label text-muted">What each account may clean up by itself</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Filter changes announced politely (4.1.3). */}
      <p aria-live="polite" className="sr-only" data-testid="cleanup-filter-announcement">
        {`Cleanup filter: ${
          CLEANUP_FILTERS.find((entry) => entry.id === filter)?.label ?? filter
        } — ${visibleGroups.length} session ${
          visibleGroups.length === 1 ? "group" : "groups"
        }`}
      </p>

      {/* Req.4/5: the persistent banners — shown, never silent. */}
      {status?.paused && (
        <div className="flex items-center gap-2 rounded-lg border border-warning/40 bg-warning/5 p-2 text-label text-ink">
          <p role="status" className="min-w-0 flex-1" data-testid="cleanup-paused-banner">
            Automatic cleanup is paused.
          </p>
          <Button size="sm" variant="secondary" onClick={togglePause} data-testid="cleanup-pause-toggle">
            Resume automatic cleanup
          </Button>
        </div>
      )}
      {status?.suspended && (
        <p
          className="flex items-center gap-2 rounded-lg border border-warning/40 bg-warning/5 p-2 text-label text-ink"
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
              if (resuming.current) return;
              resuming.current = true;
              setResumeError(null);
              void statusBridge
                .resume()
                .then((next) => {
                  setStatus(next);
                  // The banner (and this button) goes once resumed: focus
                  // to ⋯. Until then focus stays here.
                  if (!next.suspended) closeMenu(true);
                })
                .catch((cause) => setResumeError(errorText(cause)))
                .finally(() => {
                  resuming.current = false;
                });
            }}
            data-testid="cleanup-resume-button"
          >
            Re-confirm
          </Button>
          {resumeError !== null && (
            <span role="alert" className="text-danger" data-testid="cleanup-resume-error">
              Couldn't resume: {resumeError}
            </span>
          )}
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
          className="rounded-lg border border-warning/40 bg-warning/5 p-2 text-label text-ink"
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
                className="flex items-center gap-2 rounded-lg border border-hairline bg-surface/60 p-2 text-label text-ink"
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
                    statsEvent("resource.alert", { kind: "memory_hog", action: "acted" });
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
                className="rounded-lg border border-hairline bg-surface/40 p-2 text-label text-muted"
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
        <p className="text-label text-muted" role="alert" data-testid="cleanup-error">
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
        {/* AJ-E707: paired machines, under "All machines" only. */}
        {machineScope === "all" && <RemoteCleanup />}
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

      <p className="text-label text-muted" data-testid="cleanup-glossary">
        An <dfn>orphan</dfn> is a process still running after its session closed; a{" "}
        <dfn>helper</dfn> is any process a session spawned (dev server, watcher…).
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
      <p className="text-label text-muted" data-testid="cleanup-history-empty">
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
        <h3 className="text-meta font-medium text-ink">History (7 days)</h3>
        <Button size="sm" variant="secondary" onClick={doExport}>
          Export audit log…
        </Button>
      </div>
      {exported !== null && (
        <p className="text-label text-muted" data-testid="cleanup-history-exported">
          Exported {exported.split("\n").length} JSONL lines — command lines are hashed.
        </p>
      )}
      <ul className="flex flex-col">
        {props.history.rows.map((row) => (
          <li
            key={row.id}
            className="flex items-center gap-2 border-t border-hairline py-1.5 text-label"
            data-testid={`cleanup-history-row-${row.id}`}
          >
            <span
              className={
                row.automatic
                  ? "inline-flex items-center gap-1 rounded-full border border-accent/40 px-1.5 py-[1px] text-label text-accent"
                  : "inline-flex items-center gap-1 rounded-full border border-hairline px-1.5 py-[1px] text-label text-muted"
              }
              data-testid={`cleanup-history-actor-${row.id}`}
            >
              {/* Icon+text, never colour alone (1.4.1). */}
              <span aria-hidden="true">{row.automatic ? "⚙" : "👤"}</span>
              {row.automatic ? "automatic" : "you"}
            </span>
            <span className="truncate font-mono text-label text-ink">{row.cmdline_trunc}</span>
            <span className="font-mono text-label text-muted">PID {row.pid}</span>
            <span className="ml-auto whitespace-nowrap text-label text-muted">
              {row.outcome}
              {row.ports_before !== null && row.ports_after === null ? "" : ""}
            </span>
            <span className="whitespace-nowrap font-mono text-label text-muted">
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
            className="flex flex-col gap-1 rounded-lg border border-warning/40 bg-warning/5 p-2 text-label text-ink"
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

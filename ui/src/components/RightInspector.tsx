import { ParkCard } from "./ParkCard";
import { WorktreeGit } from "./WorktreeGit";
import { identityWords } from "../lib/identity";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button, Card, Chip, type ChipTone } from "./primitives";
import { TerminalView } from "./TerminalView";
import { DesktopPreview } from "./DesktopPreview";
import { PortInventoryList } from "./PortMap";
import { ProcessRow } from "./ProcessRow";
import {
  IconFleet,
  IconJuggler,
  IconProfiles,
  IconProjects,
  IconSessions,
  IconTerminal,
} from "./icons";
import type {
  AccountInfo,
  FocusOutcomeInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";
import { renewalLabel } from "../lib/renewal";
import { QuotaBar, formatCountdown } from "./primitives";
import { TwoClocks, WindowRing } from "./QuotaRings";
import { accountSetFee, quotaDisplay, switchTargets, type QuotaDisplayData, type SwitchBackSuggestionData, type SwitchTargetData, errorText,
  signedIn,
} from "../lib/bridge";
import { STATE_WORDS, formatBytes, stateChip } from "./indicator";
import { providerDisplayName } from "../lib/providers";
import { SwitchGraph } from "./SwitchGraph";
import { SwitchTargetRows } from "./SwitchTargetRows";
import { RemoteSwitchTargets } from "./RemoteSwitchTargets";
import { CrossMachineSwitchFlow } from "./CrossMachineSwitchFlow";
import { CrossMachineSwitchBack } from "./CrossMachineSwitchBack";
import {
  fleetDivergences,
  fleetSwitchBackPlan,
  fleetSwitchTargets,
  type DivergenceRecordInfo,
  type FleetSwitchBackPlanInfo,
  type RemoteSwitchTargetInfo,
  statsEvent,
} from "../lib/bridge";
import { RemoteTabBody, freshnessText, type RemoteContext } from "./RemoteTabs";

/**
 * RightInspector (AJ-E406, D-06): the one stable right-hand home. Six
 * tabs — Overview · Usage · History · Quality / Performance · Processes
 * · Actions — exist for EVERY selection type (instance, session,
 * provider, account, project) from day one; a tab without measurable
 * data renders its honest empty state with the owning task, never a
 * hidden or disabled tab. Tab state persists per selection type; the
 * embedded terminal NEVER unmounts on tab switches (panels toggle
 * `hidden`, they do not remount).
 */

export type InspectorKind = "instance" | "session" | "provider" | "account" | "project" | "machine";

export type InspectorSelection =
  | { kind: "none" }
  | { kind: "login"; id: string }
  | { kind: InspectorKind; id: string };

export const INSPECTOR_TABS = [
  { id: "overview", label: "Overview" },
  { id: "usage", label: "Usage" },
  { id: "history", label: "History" },
  { id: "quality", label: "Quality / Performance" },
  { id: "processes", label: "Processes" },
  { id: "actions", label: "Actions" },
] as const;

export type InspectorTab = (typeof INSPECTOR_TABS)[number]["id"];

/** Real data source per tab (footer contract, Req.5). */
/** Where each tab's numbers come from, in plain words (AJ-E406 Req.5;
 * D-25 wording). */
export const TAB_SOURCES: Record<InspectorTab, string> = {
  overview: "this Mac's process monitor",
  usage: "usage tracking",
  history: "the session's transcript",
  quality: "this Mac's process monitor",
  processes: "this Mac's process monitor",
  actions: "this Mac's process monitor",
};

const TAB_KEY = (kind: InspectorKind) => `aij.inspector.tab.${kind}`;

function readTab(kind: InspectorKind): InspectorTab {
  const raw = localStorage.getItem(TAB_KEY(kind));
  return INSPECTOR_TABS.some((tab) => tab.id === raw) ? (raw as InspectorTab) : "overview";
}

/** The honest empty state (D-06): visible, explained, never hidden —
 * in plain words (D-25: no task codes on screen). */
function HonestEmpty({ tab }: { tab: InspectorTab }) {
  const copy: Record<InspectorTab, string> = {
    overview: "Nothing to show here yet.",
    usage: "No usage measured for this yet.",
    history: "No history for this yet.",
    quality: "No quality measurements yet. AI Juggler never makes up a score.",
    processes: "No processes to show for this.",
    actions: "Nothing to do here yet.",
  };
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-10 text-center">
      <p className="text-[12px] font-medium text-muted">Unavailable</p>
      <p className="max-w-xs text-[11px] leading-relaxed text-muted">{copy[tab]}</p>
    </div>
  );
}

function statusTone(status: string): { tone: ChipTone; label: string } {
  const chip = stateChip(status);
  const tone: ChipTone =
    chip.tone === "live"
      ? "live"
      : chip.tone === "success"
        ? "success"
        : chip.tone === "warning"
          ? "warning"
          : chip.tone === "danger"
            ? "danger"
            : chip.tone === "accent"
              ? "accent"
              : "neutral";
  return { tone, label: chip.label };
}

/** How long ago, short: "40s", "12m", "30h", then days past 48 h ("7d"). */
export function since(ms: number, now: number): string {
  const secs = Math.max(1, Math.floor((now - ms) / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 48 * 3600) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86_400)}d`;
}

function KindIcon({ kind }: { kind: InspectorKind }) {
  const props = { width: 16, height: 16 };
  switch (kind) {
    case "instance":
      return <IconTerminal {...props} />;
    case "session":
      return <IconSessions {...props} />;
    case "provider":
      return <IconJuggler {...props} />;
    case "account":
      return <IconProfiles {...props} />;
    case "project":
      return <IconProjects {...props} />;
    case "machine":
      return <IconFleet {...props} />;
  }
}

/** D-25: a session is named by its place — branch, else folder. */
export function sessionPlace(session: InstanceInfo): string {
  const cwd = session.cwd ?? "";
  const folder = cwd.replace(/\/+$/, "").split("/").pop() || cwd;
  return session.git_branch ?? (folder || session.account_label);
}

/** "storefront · Work Max · started 38 min ago (· 18% left when low)". */
export function sessionSubline(session: InstanceInfo, projects: ProjectInfo[], now: number): string {
  const project = projects.find((p) => p.id === session.project_id)?.name;
  const left = session.quota ? Math.max(0, Math.round(100 - session.quota.used_percentage)) : null;
  return [
    project,
    session.account_label,
    `started ${since(session.started_at_ms, now)} ago`,
    left !== null && left <= 25 ? `${left}% left` : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

export const KIND_LABELS: Record<InspectorKind, string> = {
  // D-25: on screen, an Instance is a "session"; a Session (the
  // conversation record) is a "conversation".
  instance: "Session",
  session: "Conversation",
  provider: "Tool",
  account: "Account",
  project: "Project",
  machine: "Machine",
};

/** One resolved selection (the caller owns lookup). */
export type InspectorData = {
  instance?: InstanceInfo | null;
  project?: ProjectInfo | null;
  account?: AccountInfo | null;
  provider?: { id: string; name: string } | null;
  machineId?: string | null;
  /** AJ-E705: a paired machine's selection (the same shell, remote
   * tab bodies; every value source-labelled). */
  remote?: RemoteContext | null;
};

export function RightInspector({
  selection,
  data,
  sessions,
  projects,
  accounts,
  profiles,
  terminalTitle,
  freshness,
  now,
  primaryAction,
  onOpenProject,
  onOpenSession,
  onLaunch,
  onStopped,
  onHandover,
  switchBackSuggestions = [],
  onSwitchBack = () => {},
  // Kept on the props for callers; the empty work area (which used it)
  // is Home now (D-25).
  onNew: _onNew,
  processesBridge = defaultProcessesBridge,
  focusBridge = defaultFocusBridge,
  onRemoteBack,
  onRemoteSelect,
  onRemoteActionDone,
  focusTabsRequest = 0,
}: {
  selection: InspectorSelection;
  data: InspectorData;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  terminalTitle: string | null;
  freshness: Record<string, { resources: number; quota: number }>;
  now: number;
  primaryAction?: ReactNode;
  onOpenProject: (id: string) => void;
  onOpenSession: (id: string) => void;
  onLaunch: (recipe: LaunchRecipeInfo, project: ProjectInfo) => void;
  onStopped: () => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
  /** AJ-E207: open switch-back suggestions (the reset-trigger
   *  cards). Optional so bare test mounts stay honest (none). */
  switchBackSuggestions?: SwitchBackSuggestionData[];
  /** AJ-E207: opens the switch-back review for the away instance. */
  onSwitchBack?: (sessionId: string) => void;
  onNew: () => void;
  processesBridge?: ProcessesBridge;
  focusBridge?: FocusBridge;
  /** AJ-E705: back to Fleet from a remote selection (⌘[). */
  onRemoteBack?: () => void;
  /** AJ-E705: select an Instance on the same machine (null = the machine). */
  onRemoteSelect?: (instanceId: string | null) => void;
  /** AJ-E705: a remote action was accepted. */
  onRemoteActionDone?: () => void;
  /** Bumped when the opener goes away (the header's tool chip): focus
   *  moves to the selected tab once this selection's view is shown. */
  focusTabsRequest?: number;
}) {
  const kind: InspectorKind | "none" | "login" = selection.kind;
  const remote = data.remote ?? null;
  const [tab, setTab] = useState<InspectorTab>(() =>
    kind === "none" || kind === "login" ? "overview" : readTab(kind),
  );
  const tabsRef = useRef<HTMLDivElement | null>(null);

  // Switching selection restores that type's last tab (Req.3).
  useEffect(() => {
    if (kind !== "none" && kind !== "login") setTab(readTab(kind));
  }, [kind]);
  // A focus request waits until this kind's tab is the selected one.
  const pendingFocus = useRef(0);
  useEffect(() => {
    if (focusTabsRequest > 0) pendingFocus.current = focusTabsRequest;
  }, [focusTabsRequest]);
  useEffect(() => {
    if (pendingFocus.current === 0 || kind === "none" || kind === "login" || tab !== readTab(kind)) return;
    const selected = tabsRef.current?.querySelector<HTMLElement>("[role='tab'][aria-selected='true']");
    if (selected) {
      selected.focus();
      pendingFocus.current = 0;
    }
  });

  const chooseTab = (next: InspectorTab) => {
    setTab(next);
    if (kind !== "none" && kind !== "login") localStorage.setItem(TAB_KEY(kind), next);
  };
  // AJ-E1103: a tab the user picked (not one the app moved to).
  const pickTab = (next: InspectorTab) => {
    if (kind !== "none" && kind !== "login" && kind !== "machine") {
      statsEvent("ui.inspector_tab", { selection: kind, tab: next });
    }
    chooseTab(next);
  };

  // ⌘⇧H (App dispatches): jump to the Overview tab; the section itself
  // scrolls into view from InstanceOverview's own listener.
  useEffect(() => {
    const focusHandover = () => {
      if (kind !== "none" && kind !== "login") chooseTab("overview");
    };
    window.addEventListener("aij:focus-handover", focusHandover);
    return () => window.removeEventListener("aij:focus-handover", focusHandover);
  });

  // AJ-E411: `Edit renewal date...` (Account Actions) jumps to Overview;
  // RenewalEditor's own listener opens the date picker.
  useEffect(() => {
    const editRenewal = () => {
      if (kind === "account") chooseTab("overview");
    };
    window.addEventListener("aij:edit-renewal", editRenewal);
    return () => window.removeEventListener("aij:edit-renewal", editRenewal);
  });

  // Cmd/Ctrl+1..6 switches tabs (Req.6). Focus rings stay visible; the
  // tab strip supports arrow-key roving.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.shiftKey || event.altKey) return;
      const index = Number(event.key);
      if (index >= 1 && index <= INSPECTOR_TABS.length) {
        event.preventDefault();
        statsEvent("ui.shortcut_used", { shortcut: "inspector_tab" });
        pickTab(INSPECTOR_TABS[index - 1]!.id);
        const buttons = tabsRef.current?.querySelectorAll<HTMLButtonElement>("[role='tab']");
        buttons?.[index - 1]?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const headerName = remote
    ? (remote.instance?.account_label ?? remote.machine.label)
    : kind === "instance"
      ? (data.instance ? sessionPlace(data.instance) : "Session")
      : kind === "session"
        ? "Conversation"
        : kind === "provider"
          ? (data.provider?.name ?? "Tool")
          : kind === "account"
            ? (data.account?.label ?? "Account")
            : kind === "project"
              ? (data.project?.name ?? "Project")
              : "Fleet";
  const stateChipNode = remote ? (
    remote.instance ? (
      <Chip tone={statusTone(remote.instance.status).tone}>{statusTone(remote.instance.status).label}</Chip>
    ) : null
  ) : kind === "instance" && data.instance ? (
      (() => {
        const view = statusTone(data.instance.status);
        return (
          <span className="flex items-center gap-1.5">
            <Chip tone={view.tone}>{view.label}</Chip>
            {/* AJ-E510: the re-adopted marker + the honest terminal
             * note — output from before the crash is unavailable. */}
            {data.instance.adopted && (
              <span data-testid="instance-adopted-chip">
                <Chip tone="accent">↻ re-adopted</Chip>
              </span>
            )}
          </span>
        );
      })()
    ) : null;

  useEffect(() => {
    if (!remote || !onRemoteBack) return;
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "[") {
        event.preventDefault();
        onRemoteBack();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [remote, onRemoteBack]);

  // D-25: with nothing selected the work area is Home (screens/Home).
  if (kind === "none") return null;

  return (
    <div className="flex h-full min-h-0 flex-col gap-3" data-testid="right-inspector" data-inspector-kind={kind}>
      {remote && (
        <nav aria-label="Breadcrumb" className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted" data-testid="remote-breadcrumb">
          <button type="button" className="hover:text-ink" onClick={onRemoteBack}>
            Fleet
          </button>
          <span aria-hidden="true">/</span>
          {remote.instance ? (
            <button type="button" className="hover:text-ink" onClick={() => onRemoteSelect?.(null)}>
              {remote.machine.label}
            </button>
          ) : (
            <span aria-current="page" className="text-ink">
              {remote.machine.label}
            </span>
          )}
          {remote.instance && (
            <>
              <span aria-hidden="true">/</span>
              <span aria-current="page" className="text-ink">
                Session
              </span>
            </>
          )}
          <span className="ml-1">(⌘[ back)</span>
        </nav>
      )}
      {/* Header: type icon, name, I-04 state chip, primary action (Req.1). */}
      <header className="flex flex-wrap items-center gap-3">
        <span className="text-accent-hi" title={KIND_LABELS[kind as InspectorKind]}>
          <KindIcon kind={kind as InspectorKind} />
        </span>
        <h2 className="text-lg font-semibold tracking-tight">{headerName}</h2>
        {stateChipNode}
        {/* Fleet-shaped (Req.7): remote selections carry a machine chip. */}
        {remote ? (
          <span data-testid="remote-machine-chip">
            <Chip tone="accent">
              <IconFleet width={10} height={10} /> {remote.machine.label}
            </Chip>
          </span>
        ) : (
          data.machineId &&
          data.machineId !== "this-mac" && (
            <Chip tone="accent">
              <IconFleet width={10} height={10} /> {data.machineId}
            </Chip>
          )
        )}
        <div className="ml-auto flex items-center gap-2">
          {/* D-25: the session's one-step-away move, beside its Stop. */}
          {!remote && kind === "instance" && data.instance && data.instance.kind !== "desktop" && (
            <Button
              size="sm"
              variant="secondary"
              data-testid="header-switch"
              onClick={() => window.dispatchEvent(new Event("aij:focus-handover"))}
            >
              Switch account…
            </Button>
          )}
          {primaryAction ?? null}
        </div>
      </header>
      {!remote && kind === "instance" && data.instance && (
        <p className="-mt-2 text-xs text-muted" data-testid="session-subline">
          {sessionSubline(data.instance, projects, now)}
        </p>
      )}

      {remote && (
        <p
          className={
            "text-[11px] " +
            (remote.machine.freshness === "live" ? "text-muted" : "text-warning")
          }
          aria-live="polite"
          data-testid="remote-freshness"
          data-freshness={remote.machine.freshness}
        >
          {freshnessText(remote.machine, now)}
        </p>
      )}
      {/* The fixed six tabs (D-06): never hidden, never reordered. */}
      <div ref={tabsRef} role="tablist" aria-label={`${KIND_LABELS[kind as InspectorKind]} inspector tabs`} className="flex flex-wrap gap-1 border-b border-hairline pb-2">
        {INSPECTOR_TABS.map((entry, index) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            id={`inspector-tab-${entry.id}`}
            aria-selected={tab === entry.id}
            aria-controls={`inspector-panel-${entry.id}`}
            tabIndex={tab === entry.id ? 0 : -1}
            onClick={() => pickTab(entry.id)}
            onKeyDown={(event) => {
              if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
                event.preventDefault();
                const delta = event.key === "ArrowRight" ? 1 : -1;
                const next = (INSPECTOR_TABS.indexOf(INSPECTOR_TABS.find((t) => t.id === tab)!) + delta + INSPECTOR_TABS.length) % INSPECTOR_TABS.length;
                chooseTab(INSPECTOR_TABS[next]!.id);
                tabsRef.current?.querySelectorAll<HTMLButtonElement>(`[role='tab']`)[next]?.focus();
              }
            }}
            className={
              "rounded-lg px-2.5 py-1 text-[11.5px] font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
              (tab === entry.id
                ? "bg-accent-soft text-accent-hi"
                : "text-muted hover:bg-surface hover:text-ink") +
              (index < 5 ? " hidden sm:inline-flex" : "") // Actions stays; guard tiny widths
            }
          >
            {entry.label}
          </button>
        ))}
      </div>

      {/* Panels toggle `hidden` — the terminal never unmounts (gate). */}
      <div className="flex min-h-0 flex-1 flex-col">
        {INSPECTOR_TABS.map((entry) => (
          <section
            key={entry.id}
            role="tabpanel"
            id={`inspector-panel-${entry.id}`}
            aria-labelledby={`inspector-tab-${entry.id}`}
            hidden={tab !== entry.id}
            className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto"
          >
            {remote ? (
              <RemoteTabBody
                tab={entry.id}
                remote={remote}
                now={now}
                onSelectInstance={(id) => onRemoteSelect?.(id)}
                onActionDone={() => onRemoteActionDone?.()}
              />
            ) : (
            <InspectorPanel
              tab={entry.id}
              kind={kind as InspectorKind}
              data={data}
              processesBridge={processesBridge}
              sessions={sessions}
              projects={projects}
              accounts={accounts}
              profiles={profiles}
              terminalTitle={terminalTitle}
              now={now}
              onOpenProject={onOpenProject}
              onOpenSession={onOpenSession}
              onLaunch={onLaunch}
              onStopped={onStopped}
              onHandover={onHandover}
              switchBackSuggestions={switchBackSuggestions}
              onSwitchBack={onSwitchBack}
              onFocusTerminal={() => {
                chooseTab("overview");
                window.dispatchEvent(new Event("aij:focus-handover"));
              }}
              focusBridge={focusBridge}
              visible={tab === entry.id}
            />
            )}
            {/* Footer: updated-ago + the field's REAL source (Req.5). */}
            {/* Sticky with a solid background: a long tab (many switch
                destinations) scrolls under it, never draws over it. */}
            <footer className="sticky bottom-0 z-10 mt-auto flex items-center gap-1.5 border-t border-hairline/60 bg-canvas pt-2 pb-1 text-[10px] text-muted">
              <span>
                Updated{" "}
                {since(
                  remote
                    ? (remote.machine.last_heard_ms ?? now)
                    : (freshness[data.instance?.id ?? ""]?.resources ?? now),
                  now,
                )}{" "}
                ago
              </span>
              {/* D-25: where a number comes from stays one hover away;
                  a paired Mac's is worth saying out loud. */}
              {remote ? (
                <>
                  <span aria-hidden="true">·</span>
                  <span>from {remote.machine.label} over the Fleet link</span>
                </>
              ) : (
                <>
                  <span aria-hidden="true">·</span>
                  <span>from {TAB_SOURCES[entry.id]}</span>
                </>
              )}
            </footer>
          </section>
        ))}
      </div>
    </div>
  );
}

function InspectorPanel(props: {
  tab: InspectorTab;
  kind: InspectorKind;
  data: InspectorData;
  processesBridge: ProcessesBridge;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  terminalTitle: string | null;
  now: number;
  onOpenProject: (id: string) => void;
  onOpenSession: (id: string) => void;
  onLaunch: (recipe: LaunchRecipeInfo, project: ProjectInfo) => void;
  onStopped: () => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
  onFocusTerminal: () => void;
  focusBridge: FocusBridge;
  visible: boolean;
}) {
  const { tab, kind } = props;

  if (kind === "provider" || kind === "account" || kind === "project") {
    switch (tab) {
      case "overview":
        return kind === "provider" ? (
          <ProviderOverview {...props} />
        ) : kind === "account" ? (
          <AccountOverview {...props} />
        ) : (
          <ProjectOverview {...props} />
        );
      case "history":
        return kind === "account" || kind === "project" ? <GroupedHistory {...props} /> : <HonestEmpty tab="history" />;
      case "processes":
        // Rollups above Instance level are read-only (Req.2); orphan
        // stop actions live on the Instance's own tab.
        return <AggregateProcesses {...props} />;
      case "actions":
        return kind === "provider" ? (
          <ProviderActions />
        ) : kind === "account" ? (
          <AccountActions {...props} />
        ) : (
          <ProjectActions {...props} />
        );
      default:
        break;
    }
  }

  if (kind === "instance" || kind === "session") {
    switch (tab) {
      case "overview":
        return kind === "instance" ? (
          <InstanceOverview {...props} />
        ) : (
          <SessionOverview {...props} />
        );
      case "usage":
        return kind === "instance" ? <InstanceUsage {...props} /> : <SessionUsage {...props} />;
      case "history":
        return kind === "instance" ? <InstanceHistory {...props} /> : <SessionHistory {...props} />;
      case "quality":
        return <QualityTab {...props} />;
      case "processes":
        // Session (conversation) selections render the frozen final tree
        // read-only (E407 contract); the Instance tab owns stop actions.
        return <ProcessesTab {...props} readOnly={kind === "session"} />;
      case "actions":
        return kind === "instance" ? (
          <InstanceActions
            {...props}
            onRelaunch={(recipe, cwd) =>
              props.onLaunch(recipe, {
                id: "relaunch",
                root: cwd ?? "",
                name: "relaunch",
                last_activity_ms: null,
                active_instance: null,
                accounts_used: [],
                instance_count: 0,
                history_sessions: 0, history_last_used_ms: null, repo: null,
                missing: false,
              })
            }
          />
        ) : (
          <SessionActions {...props} />
        );
      default:
        break;
    }
  }

  // Honest empties for the tabs with nothing yet (D-06): visible and
  // explained, never hidden or disabled.
  return <HonestEmpty tab={tab} />;
}

/// The Processes-tab data/action seam (AJ-E410): production resolves the
/// real bridge lazily; tests inject a deterministic source. (Extracted
/// because vitest 5's mocker did not intercept this tab's dynamic
/// import site — the seam keeps the component testable either way.)
export type ProcessesBridge = {
  load: (sessionId: string) => Promise<import("../lib/bridge").ProcessesInfo | null>;
  stop: (sessionId: string, pid: number) => Promise<import("../lib/bridge").StopOutcomeInfo>;
  stopAll: (sessionId: string) => Promise<import("../lib/bridge").StopOutcomeInfo[]>;
  verify: (
    sessionId: string,
    pid: number,
  ) => Promise<import("../lib/bridge").OrphanVerifyInfo>;
};

const defaultProcessesBridge: ProcessesBridge = {
  load: async (sessionId) => (await import("../lib/bridge")).instanceProcesses(sessionId),
  stop: async (sessionId, pid) => (await import("../lib/bridge")).orphanStop(sessionId, pid),
  stopAll: async (sessionId) => (await import("../lib/bridge")).orphanStopAll(sessionId),
  verify: async (sessionId, pid) => (await import("../lib/bridge")).orphanVerify(sessionId, pid),
};

type PanelProps = {
  kind: InspectorKind;
  /** True while this panel's tab is the shown one (previews pause
   *  otherwise — AJ-E415 Req.1 "not visible"). */
  visible: boolean;
  data: InspectorData;
  /** AJ-E410 seam: the Processes-tab data/actions source (tests inject). */
  processesBridge: ProcessesBridge;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  terminalTitle: string | null;
  now: number;
  onOpenProject: (id: string) => void;
  onOpenSession: (id: string) => void;
  onLaunch: (recipe: LaunchRecipeInfo, project: ProjectInfo) => void;
  onStopped: () => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
  onFocusTerminal: () => void;
  /** AJ-E413 seam (E410 pattern): the desktop window-focus source
   *  (tests inject a recorder; the app uses the bridge default). */
  focusBridge: FocusBridge;
};

/** The desktop focus action: resolves to the honest outcome (which
 *  tier ran, the targeted window title — never overstated). */
export type FocusBridge = (sessionId: string) => Promise<FocusOutcomeInfo>;

const defaultFocusBridge: FocusBridge = async (sessionId) =>
  (await import("../lib/bridge")).desktopFocus(sessionId);

function InstanceOverview(props: PanelProps) {
  const handoverRef = useRef<HTMLDetailsElement | null>(null);
  // AJ-E414: which focus tier the last Open/Focus attempt achieved —
  // shown verbatim, never overstated. A missing Accessibility grant
  // additionally raises the one-time explainer (Req.3).
  const [focusOutcome, setFocusOutcome] = useState<FocusOutcomeInfo | null>(null);
  const [explainerDismissed, setExplainerDismissed] = useState(
    () => localStorage.getItem("aij.a11y-explainer.dismissed") === "1",
  );
  // D-25: the ranked switch list is one step away — open when this
  // session hit its limit (or a way back is on offer), otherwise behind
  // "Switch account…" (header) or ⌘⇧H.
  const [switchOpen, setSwitchOpen] = useState(() => props.data.instance?.status === "quota_blocked");
  useEffect(() => {
    const scroll = () => {
      setSwitchOpen(true);
      // jsdom has no scrollIntoView; the guard keeps tests honest.
      requestAnimationFrame(() => {
        handoverRef.current?.scrollIntoView?.({ block: "center" });
        // ⌘⇧H "focuses the hand-over section": its summary takes focus.
        handoverRef.current?.querySelector<HTMLElement>("summary")?.focus();
      });
    };
    window.addEventListener("aij:focus-handover", scroll);
    return () => window.removeEventListener("aij:focus-handover", scroll);
  }, []);
  // AJ-E209: the ranked + badged switch destinations for the selected
  // instance (§Q6 extended; suggest-only). Refetch when the instance
  // or its live state changes — the ranker's inputs (quota, sessions)
  // move with them.
  const [switchTargets_, setSwitchTargets] = useState<SwitchTargetData[] | null>(null);
  const instanceId = props.data.instance?.id ?? null;
  useEffect(() => {
    if (instanceId === null) return;
    let alive = true;
    switchTargets(instanceId)
      .then((rows) => {
        if (alive) setSwitchTargets(rows);
      })
      .catch(() => {
        if (alive) setSwitchTargets([]);
      });
    return () => {
      alive = false;
    };
  }, [instanceId, props.data.instance?.status]);
  // AJ-E706: paired machines as destinations, and — for an Instance
  // that came from another machine — the way back (with any divergence
  // that waits for the user, AJ-E708).
  const [remoteTargets, setRemoteTargets] = useState<RemoteSwitchTargetInfo[]>([]);
  const [backPlan, setBackPlan] = useState<FleetSwitchBackPlanInfo | null>(null);
  const [divergence, setDivergence] = useState<DivergenceRecordInfo | null>(null);
  const [crossTarget, setCrossTarget] = useState<RemoteSwitchTargetInfo | null>(null);
  const [backOpen, setBackOpen] = useState(false);
  const [crossTick, setCrossTick] = useState(0);
  useEffect(() => {
    if (instanceId === null) return;
    let alive = true;
    fleetSwitchTargets(instanceId)
      .then((rows) => alive && setRemoteTargets(rows))
      .catch(() => alive && setRemoteTargets([]));
    fleetSwitchBackPlan(instanceId)
      .then((plan) => alive && setBackPlan(plan))
      .catch(() => alive && setBackPlan(null));
    fleetDivergences()
      .then((rows) => alive && setDivergence(rows.find((row) => row.session_id === instanceId) ?? null))
      .catch(() => alive && setDivergence(null));
    return () => {
      alive = false;
    };
  }, [instanceId, props.data.instance?.status, crossTick]);
  const { kind, data, sessions, accounts, profiles, terminalTitle } = props;

  if (kind === "instance" && data.instance) {
    const session = data.instance;
    // AJ-E413: a desktop Instance has no terminal — the desktop surface
    // replaces the terminal card, and `Open / Focus window` is primary.
    const isDesktop = session.kind === "desktop";
    const terminalId = session.id;
    const live = session.status === "running" || session.status === "starting";
    const project = props.projects.find((p) => p.id === session.project_id) ?? null;
    return (
      // Grows with its content (the section scrolls): a long handover
      // list never squashes the overview or the terminal.
      <div className="flex flex-1 flex-col gap-3">
        {/* §5.1 Overview fields (every field honest: absent → named
            empty, never fabricated). Lineage is omitted entirely until
            the Switch Graph exists (E2) — no fabricated A→B links. */}
        <Card className="flex flex-col gap-2 p-3" data-testid="instance-overview-fields">
          {/* The header already says the state, the account and its low
              runway (D-25); this row adds only when it ended and the
              identity check. */}
          <div className="flex flex-wrap items-center gap-1.5 text-[11px] empty:hidden">
            {["finished", "stopped", "failed", "handed_over"].includes(session.status) && session.last_activity_ms ? (
              <span className="text-[11.5px] text-muted" data-testid="session-ended-stamp">
                Ended{" "}
                {new Date(session.last_activity_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })}
              </span>
            ) : null}
            {identityWords(session) && (
              <span data-testid="instance-identity" data-state={identityWords(session)!.state}>
                <Chip tone={identityWords(session)!.state === "verified" ? "success" : "neutral"}>
                  {identityWords(session)!.text}
                </Chip>
              </span>
            )}
          </div>
          {(() => {
            // AJ-E412 Req.3/5: the observed running model line — drift
            // shown in full, unobservable providers named honestly.
            const drift =
              session.running_model !== null &&
              session.launch_model !== null &&
              session.running_model !== session.launch_model
                ? `Launched with ${session.launch_model} - now running ${session.running_model}`
                : null;
            if (drift) {
              return (
                <p data-testid="instance-model-line" className="text-[11px] text-ink">
                  {drift} <span className="text-faint">(observed in transcript)</span>
                </p>
              );
            }
            if (session.running_model === null && session.launch_model === null) {
              return (
                <p data-testid="instance-model-line" className="text-[11px] text-muted">
                  Model: not reported by {providerDisplayName(session.provider)}
                </p>
              );
            }
            return null;
          })()}
          <div className="flex flex-col gap-1 text-[11px] text-muted">
            <WorktreeGit sessionId={session.id} live={live} />
            <ParkCard session={session} now={props.now} />
          </div>
          <details className="group text-[11px]" data-testid="instance-details">
            <summary className="w-fit cursor-pointer select-none rounded text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
              Details
            </summary>
            <div className="mt-2 flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <Chip tone="neutral">{providerDisplayName(session.provider)}</Chip>
            <Chip tone="neutral">
              {props.profiles.find((r) => r.id === session.launch_recipe_id)?.name ?? "launch settings"}
            </Chip>
            <Chip tone="accent">{session.model ?? "default model"}</Chip>
            {session.model_source === "launch" && session.launch_model !== null && (
              <Chip tone="warning">not verified live</Chip>
            )}
          </div>
          <div className="flex flex-col gap-1 text-muted">
            {project ? (
              <CopyField testId="instance-project" label="project" value={project.name} onOpen={() => props.onOpenProject(project.id)} />
            ) : null}
            <CopyField testId="instance-cwd" label="folder" value={session.cwd} mono />
            <span data-testid="instance-native-id">
              Claude session:{" "}
              <span className="font-mono text-ink">
                {session.conversation_id ?? "not found yet"}
              </span>
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[11px]">
            <span className="inline-flex items-center gap-1.5" data-testid="instance-quota">
              <QuotaBar quota={session.quota} />
            </span>
            <span data-testid="instance-tokens" className="inline-flex items-center gap-1.5 text-muted">
              {session.tokens ? (
                <>
                  tokens: <span className="tabular-nums text-ink">
                    {(session.tokens.input + session.tokens.output).toLocaleString()} ({session.tokens.input.toLocaleString()} in / {session.tokens.output.toLocaleString()} out)
                  </span>
                </>
              ) : (
                "No usage recorded yet"
              )}
            </span>
            <span data-testid="instance-cost" className="text-muted">
              cost: <span className="text-faint">—</span>
            </span>
            <span data-testid="instance-resources" className="tabular-nums text-muted">
              cpu {session.cpu_pct !== null ? `${Math.round(session.cpu_pct)}%` : "—"} · ram{" "}
              {session.rss_bytes !== null ? formatBytes(session.rss_bytes) : "—"} · helpers{" "}
              {session.child_count ?? "—"}
            </span>
          </div>
            </div>
          </details>
        </Card>
        {project && (
          <button
            type="button"
            onClick={() => props.onOpenProject(project.id)}
            className="flex w-fit items-center gap-1.5 rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink"
          >
            <IconProjects width={12} height={12} />
            {project.name}
          </button>
        )}
        {/* AJ-E413 Req.3: the terminal IS the Overview for a terminal-kind
            Instance; a desktop-kind Instance renders the desktop surface
            instead — status, account, project, resources, the
            per-Provider isolation claim, and window focus as primary. */}
        {isDesktop ? (
          <Card
            className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4"
            data-testid="desktop-surface"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Chip tone={statusTone(session.status).tone}>
                {statusTone(session.status).label}
              </Chip>
              <span title="desktop app session" className="inline-flex">
                <Chip tone="neutral">app</Chip>
              </span>
              <Chip tone="neutral">{providerDisplayName(session.provider)}</Chip>
              <span className="text-[11.5px] text-muted">
                {session.account_label}
                {project ? ` · ${project.name}` : ""}
              </span>
              <Button
                size="sm"
                variant="primary"
                data-testid="desktop-focus"
                onClick={async () => {
                  try {
                    setFocusOutcome(await props.focusBridge(session.id));
                  } catch (cause) {
                    setFocusOutcome({
                      outcome: `Focus failed: ${errorText(cause)}`,
                      permission_missing: false,
                      window_title: null,
                    });
                  }
                }}
              >
                Open / Focus window
              </Button>
            </div>
            {isDesktop && live && (
              <DesktopPreview
                sessionId={session.id}
                enabled={
                  props.accounts.find((a) => a.id === session.account_id)
                    ?.preview_opt_in === true
                }
                onFocus={() => {
                  void props.focusBridge(session.id);
                }}
                now={props.now}
                visible={props.visible}
              />
            )}
            <div
              className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[11px] tabular-nums text-muted"
              data-testid="desktop-resources"
            >
              <span>
                cpu {session.cpu_pct !== null ? `${Math.round(session.cpu_pct)}%` : "—"} · ram{" "}
                {session.rss_bytes !== null ? formatBytes(session.rss_bytes) : "—"} · children{" "}
                {session.child_count ?? "—"}
              </span>
              <span>
                {session.tokens
                  ? `tokens ${(
                      session.tokens.input + session.tokens.output
                    ).toLocaleString()} (${session.tokens.input.toLocaleString()} in / ${session.tokens.output.toLocaleString()} out)`
                  : "No usage recorded yet"}
              </span>
            </div>
            {session.isolation_label !== null && (
              <div
                className="flex items-start gap-2.5 rounded-lg border border-warning/25 bg-warning-soft p-2.5"
                data-testid="desktop-isolation"
              >
                <Chip tone="warning">isolation</Chip>
                <p className="text-[11.5px] leading-relaxed text-ink">
                  {session.isolation_label}
                </p>
              </div>
            )}
            {focusOutcome !== null && (
              <p className="text-[11.5px] text-muted" data-testid="desktop-focus-outcome">
                {focusOutcome.outcome}
                {focusOutcome.window_title !== null ? (
                  <span className="text-faint"> · window “{focusOutcome.window_title}”</span>
                ) : null}
              </p>
            )}
            {focusOutcome?.permission_missing === true && !explainerDismissed && (
              <div
                role="note"
                aria-label="Accessibility permission explainer"
                className="flex flex-col gap-2 rounded-lg border border-warning/25 bg-warning-soft p-2.5"
                data-testid="desktop-a11y-explainer"
              >
                <p className="text-[11.5px] leading-relaxed text-ink">
                  Juggler can open the app; exact-window focus needs Accessibility permission
                </p>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant="primary"
                    data-testid="open-a11y-settings"
                    onClick={() => {
                      void import("../lib/bridge").then((bridge) => bridge.openAccessibilitySettings());
                    }}
                  >
                    Open System Settings…
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      localStorage.setItem("aij.a11y-explainer.dismissed", "1");
                      setExplainerDismissed(true);
                    }}
                  >
                    Don’t show again
                  </Button>
                </div>
              </div>
            )}
            {!live && (
              <p className="text-[11px] text-faint" data-testid="desktop-state-line">
                {session.status === "failed"
                  ? "app crashed · exit details on the Processes tab"
                  : "app exited · relaunch from Actions"}
              </p>
            )}
            <p className="text-[11px] text-faint">
              The window is the app's own; Juggler tracks the process, never embeds it.
              Focus matches this session's recorded window — a stale title or a collision is
              reported, never guessed to a wrong window.
            </p>
          </Card>
        ) : (
          <Card className="min-h-[280px] flex-1 overflow-hidden bg-[#05070b] p-0">
            <TerminalView id={terminalId} title={terminalTitle ?? session.account_label} />
          </Card>
        )}
        {!live && !isDesktop && (
          <p className="text-[11px] text-faint">
            {`ran ${since(session.started_at_ms, Date.now())} · scrollback replayed`}
          </p>
        )}
        {!isDesktop && (
        <details
          ref={handoverRef}
          open={switchOpen || Boolean(backPlan) || (props.switchBackSuggestions ?? []).some((suggestion) => suggestion.session_id === session.id)}
          onToggle={(event) => setSwitchOpen((event.currentTarget as HTMLDetailsElement).open)}
          className="group flex flex-col gap-2"
          data-testid="handover-section"
        >
          <summary className="cursor-pointer list-none text-[11px] uppercase tracking-wider text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
            <span aria-hidden="true" className="mr-1 inline-block transition-transform group-open:rotate-90">▸</span>
            Continue on another account
          </summary>
          {/* AJ-E207 Req.3: the reset-trigger suggestion — suggested,
              never executed; the user clicks. */}
          {(props.switchBackSuggestions ?? [])
            .filter((suggestion) => suggestion.session_id === session.id)
            .map((suggestion) => (
              <button
                key={suggestion.session_id}
                type="button"
                onClick={() => props.onSwitchBack?.(suggestion.session_id)}
                data-testid="switch-back-card"
                className="rounded-lg border border-accent/40 bg-accent-soft px-2.5 py-2 text-left text-[11px] text-accent-hi transition-colors hover:border-accent/60"
              >
                <span className="block" data-testid="switch-back-card-copy">
                  {suggestion.copy}
                </span>
                <span className="text-muted">Resume the original conversation ▸</span>
              </button>
            ))}
          {/* AJ-E209 Req.1/3: the ranked, badged destinations —
              §Q6 + switch cost + return bonus; every cross-Provider
              row carries its ordered-pair badge and Drop list
              (D-04/D-11). Suggest-only; the click opens the E205
              flow. */}
          {backPlan && (
            <button
              type="button"
              onClick={() => setBackOpen(true)}
              data-testid={divergence ? "cross-back-diverged-card" : "cross-back-card"}
              className={
                "rounded-lg border px-2.5 py-2 text-left text-[11px] transition-colors " +
                (divergence
                  ? "border-warning/50 text-ink hover:border-warning"
                  : "border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60")
              }
            >
              <span className="block">
                {divergence ? (
                  <>
                    <span aria-hidden="true">⚠ </span>Diverged — {divergence.turns_since} new{" "}
                    {divergence.turns_since === 1 ? "turn" : "turns"} on {divergence.machine_label} since the work
                    left. Choose what to keep.
                  </>
                ) : (
                  <>
                    Came from {backPlan.original_machine_label} ({backPlan.original_account_label}) —{" "}
                    {backPlan.new_turns} new {backPlan.new_turns === 1 ? "turn" : "turns"} here.
                  </>
                )}
              </span>
              <span className="text-muted">
                {divergence ? "Review the conflict ▸" : `Switch back to ${backPlan.original_machine_label} ▸`}
              </span>
            </button>
          )}
          {switchTargets_ === null ? (
            <p className="text-[11px] text-faint" data-testid="handover-targets-loading">
              Ranking destinations…
            </p>
          ) : switchTargets_.length === 0 ? (
            <p className="text-[11px] text-faint" data-testid="handover-targets-empty">
              Add another account to juggle between them.
            </p>
          ) : (
            <SwitchTargetRows
              targets={switchTargets_}
              onPick={(target) =>
                props.onHandover(session.id, target.recipe_id, target.account_label)
              }
            />
          )}
          {remoteTargets.length > 0 && (
            <>
              <p className="mt-1 text-[11px] uppercase tracking-wider text-muted">
                Continue on another machine
              </p>
              <RemoteSwitchTargets targets={remoteTargets} onPick={setCrossTarget} />
            </>
          )}
        </details>
        )}
        {crossTarget && (
          <CrossMachineSwitchFlow
            sessionId={session.id}
            target={crossTarget}
            onClose={() => {
              setCrossTarget(null);
              setCrossTick((tick) => tick + 1);
            }}
          />
        )}
        {backOpen && backPlan && (
          <CrossMachineSwitchBack
            sessionId={session.id}
            plan={backPlan}
            divergence={
              divergence
                ? {
                    turns_since: divergence.turns_since,
                    last_activity_ms: divergence.last_activity_ms,
                    summary: divergence.summary,
                  }
                : null
            }
            onClose={() => {
              setBackOpen(false);
              setCrossTick((tick) => tick + 1);
            }}
            onResolved={() => setCrossTick((tick) => tick + 1)}
          />
        )}
      </div>
    );
  }

  if (kind === "project" && data.project) {
    const project = data.project;
    const history = sessions.filter((s) => s.project_id === project.id);
    return (
      <div className="flex flex-col gap-3">
        <p className="font-mono text-[11px] text-faint">{project.root}</p>
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          <Card className="flex flex-col gap-3 p-4">
            <p className="text-[11px] uppercase tracking-wider text-faint">On it now</p>
            {project.active_instance ? (
              <button
                type="button"
                onClick={() => props.onOpenSession(project.active_instance!.id)}
                className="flex items-center gap-2 rounded-lg border border-hairline px-3 py-2.5 text-left transition-colors hover:border-accent/40 hover:bg-surface"
              >
                <span className="dot-live size-1.5 rounded-full bg-live" />
                <span className="text-sm">{project.active_instance.account_label}</span>
                <Chip tone={statusTone(project.active_instance.status).tone}>
                  {statusTone(project.active_instance.status).label}
                </Chip>
                <span className="ml-auto flex items-center gap-1 text-[11px] text-muted">
                  open <IconTerminal width={12} height={12} />
                </span>
              </button>
            ) : (
              <p className="text-xs text-muted">No live session — free to juggle.</p>
            )}
            <p className="mt-2 text-[11px] uppercase tracking-wider text-faint">
              Launch with another account
            </p>
            <div className="flex flex-col gap-1.5">
              {profiles.slice(0, 5).map((recipe) => {
                const ready = signedIn(accounts.find((a) => a.id === recipe.account_id)?.login_state);
                return (
                  <button
                    key={recipe.id}
                    type="button"
                    disabled={!ready}
                    title={ready ? undefined : "Needs sign-in first (Ready home)"}
                    onClick={() => props.onLaunch(recipe, project)}
                    className={
                      "flex items-center gap-2 rounded-lg border border-hairline px-3 py-2 text-left text-xs transition-colors " +
                      (ready ? "hover:border-accent/40 hover:bg-surface" : "border-hairline/50 opacity-50")
                    }
                  >
                    <IconTerminal width={13} height={13} />
                    <span className="font-medium">{recipe.account_label}</span>
                    <span className="text-faint">·</span>
                    <span className="text-muted">{recipe.name}</span>
                    {recipe.model && (
                      <span className="ml-auto font-mono text-[10px] text-faint">{recipe.model}</span>
                    )}
                    {!ready && <Chip tone="neutral">sign in first</Chip>}
                  </button>
                );
              })}
              {profiles.length === 0 && (
                <p className="text-xs text-muted">No launch settings yet — add an account first.</p>
              )}
            </div>
          </Card>
          <Card className="flex flex-col gap-2 p-4">
            <p className="text-[11px] uppercase tracking-wider text-faint">Sessions in this project</p>
            {history.length === 0 ? (
              <p className="text-xs text-muted">Nothing recorded yet.</p>
            ) : (
              <div className="flex flex-col divide-y divide-hairline/50">
                {history.map((s) => (
                  <button
                    key={s.id}
                    type="button"
                    onClick={() => props.onOpenSession(s.id)}
                    className="flex items-center gap-2.5 py-2 text-left text-xs"
                  >
                    <span className="min-w-0 flex-1 truncate font-medium">{s.account_label}</span>
                    <Chip tone={statusTone(s.status).tone}>{statusTone(s.status).label}</Chip>
                  </button>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
    );
  }

  if (kind === "account" && data.account) {
    const account = data.account;
    const rows = sessions.filter((s) => s.account_id === account.id);
    return (
      <div className="flex flex-col gap-3">
        <Card className="flex flex-col gap-2 p-4">
          <p className="text-[11px] uppercase tracking-wider text-faint">Account</p>
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-[11.5px]">
            <span>
              tool: <span className="text-muted">{providerDisplayName(account.provider)}</span>
            </span>
            <span>
              plan: <span className="text-muted">{account.plan ?? "unknown"}</span>
            </span>
            <Chip tone={signedIn(account.login_state) ? "success" : "neutral"}>
              {signedIn(account.login_state) ? "signed in" : "signed out"}
            </Chip>
          </div>
        </Card>
        <Card className="flex flex-col gap-2 p-4">
          <p className="text-[11px] uppercase tracking-wider text-faint">Sessions on this account</p>
          {rows.length === 0 ? (
            <p className="text-xs text-muted">No sessions recorded yet.</p>
          ) : (
            rows.slice(0, 8).map((row) => (
              <button
                key={row.id}
                type="button"
                onClick={() => props.onOpenSession(row.id)}
                className="flex items-center gap-2 py-1.5 text-left text-xs"
              >
                <span className="min-w-0 flex-1 truncate font-medium">{row.account_label}</span>
                <Chip tone={statusTone(row.status).tone}>{statusTone(row.status).label}</Chip>
              </button>
            ))
          )}
        </Card>
      </div>
    );
  }

  if (kind === "provider" && data.provider) {
    const providerAccounts = accounts.filter((a) => a.provider === data.provider!.id);
    return (
      <div className="flex flex-col gap-3">
        <Card className="flex flex-col gap-2 p-4">
          <p className="text-[11px] uppercase tracking-wider text-faint">Tool</p>
          <p className="font-mono text-[11.5px]">{data.provider.id}</p>
          <p className="text-[11.5px] text-muted">
            {providerAccounts.length} account{providerAccounts.length === 1 ? "" : "s"} ·{" "}
            {providerAccounts.filter((a) => signedIn(a.login_state)).length} signed in
          </p>
        </Card>
        <Card className="flex flex-col gap-2 p-4">
          <p className="text-[11px] uppercase tracking-wider text-faint">Accounts</p>
          {providerAccounts.map((account) => (
            <p key={account.id} className="flex items-center gap-2 py-1 text-xs">
              <span className="min-w-0 flex-1 truncate font-medium">{account.label}</span>
              <Chip tone={signedIn(account.login_state) ? "success" : "neutral"}>
                {signedIn(account.login_state) ? "signed in" : "signed out"}
              </Chip>
            </p>
          ))}
          {providerAccounts.length === 0 && (
            <p className="text-xs text-muted">No accounts for this tool yet.</p>
          )}
        </Card>
      </div>
    );
  }

  // Session (conversation) — content lands with AJ-E407; the shell is honest.
  return <HonestEmpty tab="overview" />;
}




/** Copyable field row (§5.1: project + cwd copyable). */
function CopyField({ testId, label, value, mono, onOpen }: { testId: string; label: string; value: string; mono?: boolean; onOpen?: () => void }) {
  const [copied, setCopied] = useState(false);
  return (
    <span className="flex min-w-0 items-center gap-1.5" data-testid={testId}>
      <span className="shrink-0">{label}:</span>
      {onOpen ? (
        <button type="button" onClick={onOpen} className="min-w-0 truncate py-1 text-ink underline decoration-hairline underline-offset-2 outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
          {value}
        </button>
      ) : (
        <span className={"min-w-0 truncate " + (mono ? "font-mono text-ink" : "text-ink")}>{value}</span>
      )}
      <button
        type="button"
        title="Copy"
        aria-label={`Copy ${label}`}
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(value);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1200);
          } catch {
            setCopied(false);
          }
        }}
        className="shrink-0 rounded px-1 py-1.5 text-[9px] text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        {copied ? "copied" : "copy"}
      </button>
    </span>
  );
}

/** Instance Usage (§5.1): the real breakdown + honest chart empties. */
function InstanceUsage(props: PanelProps) {
  const instance = props.data.instance;
  if (!instance) return <HonestEmpty tab="usage" />;
  const tokens = instance.tokens;
  return (
    <div className="flex flex-col gap-3" data-testid="instance-usage">
      <Card className="flex flex-col gap-2 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">Token breakdown</p>
        {tokens ? (
          <table className="w-full max-w-sm text-[11.5px]">
            <tbody className="[&_td]:py-0.5">
              <tr><td className="text-muted">input</td><td className="text-right tabular-nums text-ink">{tokens.input.toLocaleString()}</td></tr>
              <tr><td className="text-muted">output</td><td className="text-right tabular-nums text-ink">{tokens.output.toLocaleString()}</td></tr>
              <tr><td className="text-muted">cache read</td><td className="text-right tabular-nums text-ink">{tokens.cache_read.toLocaleString()}</td></tr>
              <tr><td className="text-muted">cache write</td><td className="text-right tabular-nums text-ink">{tokens.cache_write.toLocaleString()}</td></tr>
              <tr className="border-t border-hairline"><td className="text-muted">total (in+out)</td><td className="text-right tabular-nums font-medium text-ink">{(tokens.input + tokens.output).toLocaleString()}</td></tr>
            </tbody>
          </table>
        ) : (
          <p className="text-[11.5px] text-muted">No usage recorded yet</p>
        )}
        <span className="text-[10px] text-muted">
          <span className="rounded border border-hairline px-1">Measured</span> counted from what the session reports
        </span>
      </Card>
      <Card className="flex flex-col gap-2 p-4" data-testid="two-clock-card">
        <p className="text-[11px] uppercase tracking-wider text-muted">Account · the two clocks</p>
        {/* AJ-E610: the account's windows + the SEPARATE subscription
            clock (a calendar line — never merged with the rings) and
            the forecast pill. */}
        <AccountTwoClocks accountId={instance.account_id} />
      </Card>
      {/* D-25: tokens/min and cost over time join this tab once a time
          series exists; until then the card isn't shown. */}
    </div>
  );
}

/** Instance History (§5.1): resumes/handovers/crashes; honest until the
 * indexer exists. Imported rows will carry the `imported` badge. */
function InstanceHistory(props: PanelProps) {
  const instance = props.data.instance;
  if (!instance) return <HonestEmpty tab="history" />;
  return (
    <div className="flex flex-col gap-3" data-testid="instance-history">
      <Card className="flex flex-col gap-2 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">This session</p>
        <p className="text-[11.5px] text-muted">No history recorded yet</p>
        <p className="text-[10px] text-muted">Resumes, handovers and crashes will show here.</p>
        <p className="text-[10px] text-muted">
          Started {new Date(instance.started_at_ms).toLocaleString()} · {STATE_WORDS[instance.status] ?? instance.status}
        </p>
      </Card>
    </div>
  );
}

/** One action row: disabled actions keep their reason text adjacent
 * (deep 02 §10 — never hidden). */
function ActionRow({
  label,
  shortcut,
  reason,
  onClick,
  danger,
}: {
  label: string;
  shortcut?: string;
  reason?: string;
  onClick?: () => void;
  danger?: boolean;
}) {
  const disabled = !onClick;
  return (
    <div className="flex items-center gap-3 py-1.5" data-testid={`action-${label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "")}`}>
      <button
        type="button"
        disabled={disabled}
        title={shortcut ? `${label} (${shortcut})` : label}
        onClick={onClick}
        className={
          "rounded-lg border px-2.5 py-1.5 text-[11.5px] font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (disabled
            ? "cursor-default border-hairline/50 text-faint"
            : danger
              ? "border-danger/30 bg-danger-soft text-danger hover:border-danger/50"
              : "border-hairline text-ink hover:border-accent/40 hover:bg-surface") +
          (danger ? " opacity-90" : "")
        }
      >
        {label}
        {shortcut ? <span className="ml-1.5 text-[9px] text-faint">{shortcut}</span> : null}
      </button>
      {reason ? <span className="text-[10.5px] text-muted">{reason}</span> : null}
    </div>
  );
}

/** Instance Actions (§5.1): real where the bridge is real; disabled
 * WITH reason where the owner task has not landed. */
function InstanceActions(props: PanelProps & { onRelaunch?: (recipe: LaunchRecipeInfo, cwd: string | null) => void }) {
  const instance = props.data.instance;
  if (!instance) return <HonestEmpty tab="actions" />;
  const live = instance.status === "running" || instance.status === "starting";
  const ended = ["finished", "stopped", "failed", "handed_over"].includes(instance.status);
  const recipe = props.profiles.find((r) => r.id === instance.launch_recipe_id) ?? null;
  return (
    <div className="flex flex-col gap-2" data-testid="instance-actions">
      <ActionRow
        label={instance.kind === "desktop" ? "Open / Focus window" : "Focus terminal"}
        onClick={
          instance.kind === "desktop"
            ? () => {
                void props.focusBridge(instance.id);
              }
            : props.onFocusTerminal
        }
      />
      <ActionRow label="Stop" shortcut="⌘." onClick={live ? props.onStopped : undefined} reason={live ? undefined : "not running"} />
      <ActionRow
        label={ended ? "Relaunch" : "Resume"}
        onClick={ended && recipe ? () => props.onRelaunch?.(recipe, instance.cwd) : undefined}
        reason={ended ? (recipe ? undefined : "launch settings missing") : "still running"}
      />
      <ActionRow
        label="Hand over…"
        shortcut="⌘⇧H"
        onClick={() => props.onFocusTerminal()}
        reason="scroll to Continue on another account"
      />
    </div>
  );
}

/** Session (conversation) Overview (§5.2): ended = immutable with the
 * Ended stamp; unreadable transcripts never zero stats. */
function SessionOverview(props: PanelProps) {
  const instance = props.data.instance;
  const ended = instance
    ? ["finished", "stopped", "failed", "handed_over", "quota_blocked"].includes(instance.status)
    : false;
  return (
    <div className="flex flex-col gap-3" data-testid="session-overview">
      <Card className="flex flex-col gap-2 p-4">
        <div className="flex flex-wrap items-center gap-2 text-[11.5px]">
          {instance ? (
            <Chip tone={statusTone(instance.status).tone}>{statusTone(instance.status).label}</Chip>
          ) : (
            <Chip tone="neutral">unknown state</Chip>
          )}
          {ended && instance?.last_activity_ms ? (
            <span className="text-muted" data-testid="session-ended-stamp">
              Ended{" "}
              {new Date(instance.last_activity_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })}
            </span>
          ) : null}
          {instance ? (
            <button
              type="button"
              onClick={() => props.onOpenSession(instance.id)}
              className="-my-1 py-1 text-muted underline decoration-hairline underline-offset-2 outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              owning session: {instance.account_label}
            </button>
          ) : null}
        </div>
        <div className="flex flex-col gap-1 text-[11px] text-muted">
          <span data-testid="session-tokens">
            tokens:{" "}
            {instance?.tokens ? (
              <span className="tabular-nums text-ink">
                {(instance.tokens.input + instance.tokens.output).toLocaleString()}
              </span>
            ) : (
              "No usage recorded yet"
            )}
          </span>
          <span data-testid="session-transcript" className="text-muted">
            Transcript not readable yet
          </span>
        </div>
      </Card>
      <Card className="flex flex-col gap-1 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">Turns · models · files touched</p>
        <p className="text-[11.5px] text-muted">Transcript not readable yet</p>
      </Card>
    </div>
  );
}


/** AJ-E610: the Usage tab's account block (engine query; a null
 * account id — orphaned launch settings — renders the honest empty). */
function AccountTwoClocks({ accountId }: { accountId: string | null }) {
  const [display, setDisplay] = useState<QuotaDisplayData | null>(null);
  useEffect(() => {
    if (!accountId) return;
    let alive = true;
    quotaDisplay(accountId).then((payload) => {
      if (alive) setDisplay(payload);
    });
    return () => {
      alive = false;
    };
  }, [accountId]);
  if (!accountId) {
    return <p className="text-xs text-muted">Account unknown — usage cannot be attributed.</p>;
  }
  if (!display) {
    return <p className="text-xs text-muted">loading…</p>;
  }
  return <TwoClocks display={display} />;
}

/** Session Usage (§5.2): per-turn table + CSV — honest until transcript
 * indexing exists; aggregate tokens are real. */
function SessionUsage(props: PanelProps) {
  const instance = props.data.instance;
  return (
    <div className="flex flex-col gap-3" data-testid="session-usage">
      <Card className="flex flex-col gap-2 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">Per-turn tokens · cost</p>
        {instance?.tokens ? (
          <p className="text-[11.5px] text-muted">
            aggregate:{" "}
            <span className="tabular-nums text-ink">
              {(instance.tokens.input + instance.tokens.output).toLocaleString()} tokens
            </span>
          </p>
        ) : (
          <p className="text-[11.5px] text-muted">No usage recorded yet</p>
        )}
      </Card>
    </div>
  );
}

/** Session History (§5.2): the A→B→A lineage chain. */
function SessionHistory(_props: PanelProps) {
  return (
    <div className="flex flex-col gap-3" data-testid="session-history">
      <Card className="flex flex-col gap-2 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">Lineage</p>
        <p className="text-[11.5px] text-muted">No switches between accounts yet.</p>
      </Card>
    </div>
  );
}

/** Session Actions (§5.2): Resume primary; the rest honest. */
function SessionActions(props: PanelProps) {
  const instance = props.data.instance;
  const recipe = instance
    ? props.profiles.find((r) => r.id === instance.launch_recipe_id) ?? null
    : null;
  const ended = instance
    ? ["finished", "stopped", "failed", "handed_over", "quota_blocked"].includes(instance.status)
    : false;
  return (
    <div className="flex flex-col gap-2" data-testid="session-actions">
      <ActionRow
        label="Resume"
        shortcut="⌘R"
        onClick={
          ended && recipe
            ? () =>
                props.onLaunch(recipe, {
                  id: "resume",
                  root: instance?.cwd ?? "",
                  name: "resume",
                  last_activity_ms: null,
                  active_instance: null,
                  accounts_used: [],
                  instance_count: 0,
                  history_sessions: 0, history_last_used_ms: null, repo: null,
                  missing: false,
                })
            : undefined
        }
        reason={ended ? (recipe ? undefined : "launch settings missing") : "conversation still live"}
      />
      <ActionRow
        label="Hand over…"
        shortcut="⌘⇧H"
        onClick={() => window.dispatchEvent(new Event("aij:focus-handover"))}
        reason="opens the owning session's hand-over section"
      />
    </div>
  );
}

// --- AJ-E408: Provider / Account / Project content -----------------------

/** Provider Overview (§5.3): counts, worst-first aggregate, honest
 * capability/version badges. */
/** What this Mac has of a tool: installed version, verified range. */
/** What's installed for one tool: null while checking (nothing shown),
 * "failed" when the check failed or doesn't know the tool. The result is
 * kept per tool, so another tool's version never shows while checking. */
export function useToolInstall(
  providerId: string | null,
): import("../lib/bridge").ProviderInstallData | "failed" | null {
  const [result, setResult] = useState<{
    providerId: string;
    install: import("../lib/bridge").ProviderInstallData | "failed";
  } | null>(null);
  useEffect(() => {
    if (!providerId) return;
    let alive = true;
    void import("../lib/bridge")
      .then((bridge) => bridge.providersDiscover())
      .then(
        (rows) => {
          if (alive) setResult({ providerId, install: rows.find((row) => row.provider === providerId) ?? "failed" });
        },
        () => {
          if (alive) setResult({ providerId, install: "failed" });
        },
      );
    return () => {
      alive = false;
    };
  }, [providerId]);
  return result && result.providerId === providerId ? result.install : null;
}

/** The installed version and verified range, in words. */
export function installWords(install: import("../lib/bridge").ProviderInstallData): string {
  const verified = install.verified_against ?? [];
  switch (install.status) {
    case "installed":
      return `${install.version ? `Version ${install.version}` : "Installed"}${verified.length > 0 ? ` · tested with ${verified.join(", ")}` : ""}`;
    case "unverified":
      return `Version ${install.version ?? "unknown"} · newer than AI Juggler has been tested with`;
    case "not_installed":
      return "Not installed on this Mac";
    default:
      return "Couldn't check what's installed";
  }
}

function ProviderOverview(props: PanelProps) {
  const provider = props.data.provider;
  const install = useToolInstall(provider?.id ?? null);
  if (!provider) return <HonestEmpty tab="overview" />;
  const providerAccounts = props.accounts.filter((a) => a.provider === provider.id);
  const providerSessions = props.sessions.filter((s) => s.provider === provider.id);
  const running = providerSessions.filter((s) => s.status === "running" || s.status === "starting");
  // Aggregate quota, WORST ACCOUNT FIRST (recognition over recall); each
  // row traces to its rail row (Req.7 — no unexplainable dashboard).
  const perAccount = providerAccounts
    .map((account) => {
      const rows = providerSessions.filter((s) => s.account_id === account.id && s.quota !== null);
      const worst = rows.reduce<InstanceInfo | null>((min, row) => {
        if (!min) return row;
        return (100 - row.quota!.used_percentage) < (100 - min.quota!.used_percentage) ? row : min;
      }, null);
      return { account, worst };
    })
    .sort((a, b) => {
      const leftA = a.worst ? 100 - a.worst.quota!.used_percentage : Number.POSITIVE_INFINITY;
      const leftB = b.worst ? 100 - b.worst.quota!.used_percentage : Number.POSITIVE_INFINITY;
      return leftA - leftB;
    });
  const tiers = new Set(
    perAccount.filter((entry) => entry.worst).map((entry) => entry.worst!.quota!.confidence),
  );
  return (
    <div className="flex flex-col gap-3" data-testid="provider-overview">
      <Card className="flex flex-col gap-2 p-4">
        <div className="flex flex-wrap items-center gap-2 text-[11.5px]">
          <span className="text-ink">
            {providerAccounts.length} account{providerAccounts.length === 1 ? "" : "s"}
          </span>
          <span className="text-muted">· {running.length} running</span>
        </div>
        {/* Shown once known; nothing in its place until then (D-25). */}
        {install !== null && (
          <p className="text-[11px] text-muted" data-testid="provider-install">
            {install === "failed" ? "Couldn't check what's installed" : installWords(install)}
          </p>
        )}
      </Card>
      <Card className="flex flex-col gap-1.5 p-4" data-testid="provider-aggregate">
        <p className="text-[11px] uppercase tracking-wider text-muted">Quota, lowest first</p>
        {perAccount.length === 0 ? (
          <p className="text-[11.5px] text-muted">No accounts for this tool yet.</p>
        ) : (
          perAccount.map(({ account, worst }) => (
            <div key={account.id} className="flex flex-col gap-1 py-0.5">
              <span className="text-[11.5px]">
                <span className="text-ink">{account.label}</span>
                {!signedIn(account.login_state) ? <span className="text-faint"> · signed out</span> : null}
              </span>
              {/* D-25: a signed-out account with no reading says "signed
                  out" above; an empty bar would only repeat it. */}
              {worst?.quota || signedIn(account.login_state) ? <QuotaBar quota={worst?.quota ?? null} /> : null}
            </div>
          ))
        )}
        {tiers.size > 1 ? (
          <span className="text-[10px] text-muted" data-testid="provider-mixed">
            <span className="rounded border border-hairline px-1">mixed</span> some of these are measured, some estimated
          </span>
        ) : null}
      </Card>
    </div>
  );
}

function ProviderActions() {
  return (
    <div className="flex flex-col gap-2" data-testid="provider-actions">
      <ActionRow label="Add account…" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))} reason="opens Accounts" />
    </div>
  );
}

/** Account Overview (§5.4): rings with % text, forecast, affinity,
 * renewal field (E411), logged-out collapse to the sign-in card. */
function AccountOverview(props: PanelProps) {
  const account = props.data.account;
  if (!account) return <HonestEmpty tab="overview" />;
  const rows = props.sessions.filter((s) => s.account_id === account.id);
  const running = rows.filter((s) => s.status === "running" || s.status === "starting");
  const worst = rows
    .filter((s) => s.quota !== null)
    .reduce<InstanceInfo | null>((min, row) => {
      if (!min) return row;
      return (100 - row.quota!.used_percentage) < (100 - min.quota!.used_percentage) ? row : min;
    }, null);
  const usualProjects = props.projects.filter((p) => p.accounts_used.includes(account.label));

  if (!signedIn(account.login_state)) {
    // Logged-out collapse: the Overview IS the sign-in card (Req.3).
    return (
      <Card className="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center" data-testid="account-signin-card">
        <p className="text-[13px] font-medium text-ink">{account.label} is signed out</p>
        <p className="max-w-xs text-[11.5px] leading-relaxed text-muted">
          Sign in from the Accounts screen to see quota, usage and launch controls again. Nothing is
          deleted — the isolated account state is kept.
        </p>
        <Button size="sm" variant="primary" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))}>
          Open Accounts to sign in
        </Button>
      </Card>
    );
  }

  return (
    <div className="flex flex-col gap-3" data-testid="account-overview">
      <Card className="flex flex-wrap items-center gap-4 p-4">
        <div className="flex flex-wrap items-center gap-4" data-testid="account-rings">
          {/* AJ-E610: engine-evaluated windows — ring + % + badge + reset;
              an omitted window renders its reason, never an empty ring. */}
          <AccountWindowRings accountId={account.id} />
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-1 text-[11px]">
          <span className="flex items-center gap-2">
            <Chip tone="success">signed in</Chip>
            <span className="text-muted">plan: <span className="text-ink">{account.plan ?? "unknown"}</span></span>
          </span>
          <span data-testid="account-renewal" className="flex flex-wrap items-center gap-1.5">
            {(() => {
              const label = renewalLabel(account.renewal_date, props.now);
              if (account.renewal_date === null) {
                return <span className="text-muted">Expiry: unknown</span>;
              }
              if (label?.tier === "warning") {
                return <Chip tone="warning">⚠ {label.text}</Chip>;
              }
              if (label?.tier === "danger") {
                return <Chip tone="danger">⛔ {label.text}</Chip>;
              }
              return <span className="text-muted">{label?.text ?? "Expiry: unknown"}</span>;
            })()}
          </span>
          <RenewalEditor account={account} />
          <FeeEditor account={account} />
          {/*
           * AJ-E508 (D-08): the Account launch-settings area's TWO
           * separate tier toggles — never one bundled "auto-cleanup"
           * switch. Exact labels; both off by default.
           */}
          <TierToggles accountId={account.id} initial={account.cleanup_opt_in} />
          <label data-testid="account-preview-toggle" className="flex items-center gap-1.5">
            <input
              type="checkbox"
              checked={account.preview_opt_in}
              onChange={(event) => {
                const optIn = event.target.checked;
                void import("../lib/bridge").then((bridge) =>
                  bridge.accountSetPreviewOptIn(account.id, optIn).catch(() => {
                    /* re-reads from props on the next list refresh */
                  }),
                );
              }}
            />
            <span className="text-muted">Live preview of desktop sessions (opt-in)</span>
          </label>
          <span data-testid="account-reset" className="text-muted">
            reset:{" "}
            {worst?.quota?.resets_at_ms ? (
              <span className="text-ink">{formatCountdown(worst.quota.resets_at_ms, props.now)}</span>
            ) : (
              <span className="text-faint">not known yet</span>
            )}
          </span>
        </div>
      </Card>
      <Card className="flex flex-col gap-1.5 p-4">
        <p className="text-[11px] uppercase tracking-wider text-muted">Running sessions ({running.length})</p>
        {running.length === 0 ? (
          <p className="text-[11.5px] text-muted">None right now.</p>
        ) : (
          running.map((row) => (
            <button key={row.id} type="button" onClick={() => props.onOpenSession(row.id)} className="flex items-center gap-2 py-1 text-left text-xs">
              <span className="min-w-0 flex-1 truncate font-medium">{row.account_label}</span>
              <Chip tone={statusTone(row.status).tone}>{statusTone(row.status).label}</Chip>
            </button>
          ))
        )}
      </Card>
      <Card className="flex flex-col gap-1.5 p-4" data-testid="account-affinity">
        <p className="text-[11px] uppercase tracking-wider text-muted">Usual projects</p>
        {usualProjects.length === 0 ? (
          <p className="text-[11.5px] text-muted">No project affinity yet.</p>
        ) : (
          usualProjects.map((project) => (
            <button key={project.id} type="button" onClick={() => props.onOpenProject(project.id)} className="flex items-center gap-2 py-1 text-left text-xs">
              <span className="min-w-0 flex-1 truncate font-medium">{project.name}</span>
              <span className="text-faint">usual</span>
            </button>
          ))
        )}
      </Card>
    </div>
  );
}

function AccountActions(props: PanelProps) {
  const account = props.data.account;
  if (!account) return <HonestEmpty tab="actions" />;
  const recipes = props.profiles.filter((r) => r.account_id === account.id);
  const first = recipes[0] ?? null;
  return (
    <div className="flex flex-col gap-2" data-testid="account-actions">
      <ActionRow
        label="Launch session"
        onClick={
          first
            ? () =>
                props.onLaunch(first, {
                  id: "launch", root: first.default_cwd ?? "", name: "launch", last_activity_ms: null,
                  active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false,
                })
            : undefined
        }
        reason={first ? undefined : "no launch settings on this account yet"}
      />
      <ActionRow label={signedIn(account.login_state) ? "Re-check login" : "Sign in"} onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))} reason="opens Accounts" />
      <ActionRow
        label="Edit renewal date..."
        onClick={() => window.dispatchEvent(new Event("aij:edit-renewal"))}
        reason="jumps to the Overview editor"
      />
    </div>
  );
}

/** Project Overview (§5.5): On-it-now row (full I-01..I-11 via
 * SessionRow), root copy, branch summary, counts, missing-dir banner. */
function ProjectOverview(props: PanelProps) {
  const project = props.data.project;
  if (!project) return <HonestEmpty tab="overview" />;
  const rows = props.sessions.filter((s) => s.project_id === project.id);
  const active = rows.find((s) => s.status === "running" || s.status === "starting") ?? null;
  const branches = [...new Set(rows.map((r) => r.git_branch).filter((b): b is string => b !== null))];
  const dirtyTotal = rows.reduce((sum, r) => sum + (r.git_dirty_count ?? 0), 0);
  return (
    <div className="flex flex-col gap-3" data-testid="project-overview">
      {project.missing ? (
        <div className="flex items-center gap-3 rounded-xl border border-danger/40 bg-danger-soft px-4 py-3" data-testid="project-missing-banner" role="alert">
          <span className="text-[12px] font-medium text-danger">Project directory is missing</span>
          <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted">{project.root}</span>
        </div>
      ) : null}
      {rows.length === 0 ? (
        <Card className="flex flex-col items-center gap-3 p-6 text-center">
          <p className="text-[12px] font-medium text-ink">No sessions in this project yet</p>
          <Button size="sm" variant="primary" onClick={() => window.dispatchEvent(new Event("aij:new-session"))}>
            <IconTerminal width={13} height={13} />
            Start first session
          </Button>
        </Card>
      ) : (
        <>
          <Card className="flex flex-col gap-1 p-3">
            <p className="text-[11px] uppercase tracking-wider text-muted">On it now</p>
            {active ? (
              <div data-testid="project-on-it-now" className="cursor-pointer" onClick={() => props.onOpenSession(active.id)}>
                <SessionRowForCard session={active} />
              </div>
            ) : (
              <p className="text-[11.5px] text-muted">No live session — free to juggle.</p>
            )}
          </Card>
          <Card className="flex flex-col gap-2 p-4">
            <CopyField testId="project-root" label="root" value={project.root} mono />
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-muted" data-testid="project-summary">
              <span>
                branches: <span className="text-ink">{branches.length > 0 ? branches.join(", ") : "none"}</span>
                {dirtyTotal > 0 ? <span className="text-warning"> · {dirtyTotal} dirty</span> : null}
              </span>
              <span>
                accounts used: <span className="text-ink">{project.accounts_used.join(", ") || "none"}</span>
              </span>
              <span>
                sessions: <span className="tabular-nums text-ink">{project.instance_count}</span> · imported sessions:{" "}
                <span className="tabular-nums text-ink">{project.history_sessions}</span>
              </span>
            </div>
          </Card>
        </>
      )}
      {/* AJ-E209 Req.4/5: the per-Work Switch Graph — how the work
          travelled across accounts, with per-node Resume / Switch
          from here / Export pack and failed-edge recovery. */}
      <Card className="flex flex-col gap-2 p-4" data-testid="project-switch-graph-card">
        <p className="text-[11px] uppercase tracking-wider text-muted">How this work travelled</p>
        <SwitchGraph projectId={project.id} onOpenSession={props.onOpenSession} />
      </Card>
    </div>
  );
}

/** SessionRow reuse for the On-it-now card — import late to avoid cycles. */
import { SessionRow } from "./WorkflowRail";
function SessionRowForCard({ session }: { session: InstanceInfo }) {
  return <SessionRow session={session} selected={false} onSelect={() => {}} />;
}

function ProjectActions(props: PanelProps) {
  const project = props.data.project;
  if (!project) return <HonestEmpty tab="actions" />;
  // Ranked by headroom: the signed-in account with the MOST quota left
  // first; `Usual` star marks accounts that already ran in this project.
  const ranked = props.profiles
    .map((recipe) => {
      const worst = props.sessions
        .filter((s) => s.account_id === recipe.account_id && s.quota !== null)
        .reduce<number | null>((min, row) => {
          const left = 100 - row.quota!.used_percentage;
          return min === null ? left : Math.min(min, left);
        }, null);
      const usual = project.accounts_used.includes(recipe.account_label);
      return { recipe, left: worst ?? Number.POSITIVE_INFINITY, usual };
    })
    .sort((a, b) => b.left - a.left);
  return (
    <div className="flex flex-col gap-2" data-testid="project-actions">
      <p className="text-[11px] uppercase tracking-wider text-muted">Launch with another account (ranked by headroom)</p>
      {ranked.map(({ recipe, usual }) => {
        const ready = signedIn(props.accounts.find((a) => a.id === recipe.account_id)?.login_state);
        return (
          <button
            key={recipe.id}
            type="button"
            disabled={!ready}
            onClick={() =>
              props.onLaunch(recipe, {
                id: project.id, root: project.root, name: project.name, last_activity_ms: null,
                active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: project.missing,
              })
            }
            title={ready ? undefined : "Needs sign-in first"}
            className={
              "flex items-center gap-2 rounded-lg border px-3 py-2 text-left text-xs transition-colors " +
              (ready ? "border-hairline hover:border-accent/40 hover:bg-surface" : "border-hairline/50 text-faint")
            }
          >
            <span className="font-medium">{recipe.account_label}</span>
            {usual ? <span className="rounded border border-accent/30 bg-accent-soft px-1 text-[9px] text-accent-hi">Usual</span> : null}
            <span className="ml-auto text-faint">{recipe.name}</span>
          </button>
        );
      })}
      {ranked.length === 0 ? <p className="text-[11.5px] text-muted">No launch settings yet — add an account first.</p> : null}
    </div>
  );
}

/** Account/Project History (§5.4/§5.5): real session timeline grouped
 * Today / Yesterday / Earlier; lineage honest. */
function GroupedHistory(props: PanelProps) {
  const kind = props.kind;
  const rows =
    kind === "account"
      ? props.sessions.filter((s) => s.account_id === props.data.account?.id)
      : props.sessions.filter((s) => s.project_id === props.data.project?.id);
  const dayStart = (offset: number) => {
    const date = new Date(props.now);
    date.setHours(0, 0, 0, 0);
    return date.getTime() - offset * 86_400_000;
  };
  const groups: { label: string; from: number; to: number }[] = [
    { label: "Today", from: dayStart(0), to: Number.POSITIVE_INFINITY },
    { label: "Yesterday", from: dayStart(1), to: dayStart(0) },
    { label: "Earlier", from: Number.NEGATIVE_INFINITY, to: dayStart(1) },
  ];
  return (
    <div className="flex flex-col gap-3" data-testid={`${kind}-history`}>
      {groups.map((group) => {
        const groupRows = rows.filter((r) => r.started_at_ms >= group.from && r.started_at_ms < group.to);
        if (groupRows.length === 0) return null;
        return (
          <Card key={group.label} className="flex flex-col gap-1 p-4">
            <p className="text-[11px] uppercase tracking-wider text-muted">{group.label}</p>
            {groupRows.map((row) => (
              <button key={row.id} type="button" onClick={() => props.onOpenSession(row.id)} className="flex items-center gap-2 py-1 text-left text-xs">
                {/* An account's own history names each session by its place. */}
                <span className="min-w-0 flex-1 truncate font-medium">{kind === "account" ? sessionPlace(row) : row.account_label}</span>
                <span className="text-faint">{since(row.started_at_ms, props.now)} ago</span>
                <Chip tone={statusTone(row.status).tone}>{statusTone(row.status).label}</Chip>
              </button>
            ))}
          </Card>
        );
      })}
      {rows.length === 0 ? (
        <Card className="flex flex-col gap-1 p-4">
          <p className="text-[11.5px] text-muted">No history recorded yet</p>
        </Card>
      ) : null}
    </div>
  );
}


// --- AJ-E409: Quality/Performance tab (D-07 — signals, never a score) ---

function TierTag({ tier }: { tier: "Measured" | "Computed" | "Estimated" }) {
  return (
    <span className="rounded border border-hairline px-1 text-[9px] font-medium uppercase tracking-wide text-muted">
      {tier}
    </span>
  );
}

/// The `Renewal date (optional)` editor (AJ-E411 Req.4): date input +
/// Clear, helper text `Entered by you - AI Juggler does not read billing`,
/// and focus hand-off from Account Actions (`aij:edit-renewal`).
/**
 * AJ-E508 (D-08): the Account launch-settings area's per-tier
 * opt-ins — TWO separate toggles with the exact labels, both off by
 * default, never bundled (ADR-0010's amendment). Deliberately
 * mouse-only (no shortcut could enable automation).
 */
/** `explain` (default on) adds the D-08 rules under the toggles; a list
 * of Accounts that states them once turns it off. */
export function TierToggles(props: { accountId: string; initial: boolean; explain?: boolean }) {
  const [tiers, setTiers] = useState({ tier1: props.initial, tier2: false });
  const [loaded, setLoaded] = useState(false);
  // The tier table is the source of truth (the account row only
  // mirrors Tier 1 for older readers).
  useEffect(() => {
    let alive = true;
    import("../lib/bridge")
      .then((bridge) => bridge.cleanupTiersGet(props.accountId))
      .then((settings) => {
        if (alive) {
          setTiers({ tier1: settings.tier1_opt_in, tier2: settings.tier2_opt_in });
          setLoaded(true);
        }
      })
      .catch(() => {
        /* keep the both-off default on failure — never a silent opt-in */
      });
    return () => {
      alive = false;
    };
  }, [props.accountId]);
  const set = (tier: "tier1" | "tier2", value: boolean) => {
    setTiers((current) => ({ ...current, [tier]: value }));
    void import("../lib/bridge")
      .then((bridge) =>
        bridge.cleanupTiersSet(
          props.accountId,
          tier === "tier1" ? value : tiers.tier1,
          tier === "tier2" ? value : tiers.tier2,
        ),
      )
      .catch(() => {
        /* the checkboxes re-read on the next mount */
      });
  };
  return (
    <span className="flex flex-col gap-1" data-testid="account-tier-toggles">
      <label data-testid="account-cleanup-toggle" className="flex items-center gap-1.5">
        <input
          type="checkbox"
          checked={tiers.tier1}
          disabled={!loaded}
          onChange={(event) => set("tier1", event.target.checked)}
        />
        <span className="text-muted">Automatically tidy left-behind dev servers</span>
      </label>
      <label data-testid="account-tier2-toggle" className="flex items-center gap-1.5">
        <input
          type="checkbox"
          checked={tiers.tier2}
          disabled={!loaded}
          onChange={(event) => set("tier2", event.target.checked)}
        />
        <span className="text-muted">Also tidy after a crash (with notification)</span>
      </label>
      {props.explain !== false && (
        <span className="text-[10px] leading-relaxed text-muted">
          Both off by default — only verified dev servers and watchers of this account's clean
          exits are tidied; shells, MCP servers and databases are never touched automatically
          and every stop is written to the log. Crash tidies wait 5 minutes and ask first
          with an Undo window.
        </span>
      )}
    </span>
  );
}

/** AJ-E610: the account card's engine-driven window rings. */
function AccountWindowRings({ accountId }: { accountId: string }) {
  const [display, setDisplay] = useState<QuotaDisplayData | null>(null);
  useEffect(() => {
    let alive = true;
    quotaDisplay(accountId).then((payload) => {
      if (alive) setDisplay(payload);
    });
    return () => {
      alive = false;
    };
  }, [accountId]);
  if (!display) {
    return (
      <p className="text-xs text-muted" data-testid="account-rings-loading">
        loading quota…
      </p>
    );
  }
  if (display.unavailable_reason) {
    return (
      <p className="text-xs text-muted" data-testid="quota-unavailable">
        {display.unavailable_reason}
      </p>
    );
  }
  return (
    <>
      <WindowRing window={display.five_hour} kind="5h" omittedReason={display.five_hour_omitted_reason} />
      <WindowRing window={display.weekly} kind="weekly" omittedReason={display.weekly_omitted_reason} />
    </>
  );
}

/** AJ-E610/E609: the user-entered flat fee behind the dual view's
 * ratio — explicit save only; no plan-tier inference ever writes it. */
function FeeEditor({ account }: { account: AccountInfo }) {
  const [fee, setFee] = useState("");
  const [status, setStatus] = useState("");
  useEffect(() => {
    setFee(localStorage.getItem(`aij.fee.${account.id}`) ?? "");
  }, [account.id]);
  return (
    <span className="flex items-center gap-1.5" data-testid="fee-editor">
      <label className="text-[11px] text-muted" htmlFor={`fee-${account.id}`}>
        Flat fee $/mo
      </label>
      <input
        id={`fee-${account.id}`}
        value={fee}
        onChange={(event) => setFee(event.target.value)}
        placeholder="200"
        inputMode="decimal"
        className="w-16 rounded border border-hairline bg-canvas px-1.5 py-0.5 text-[11px] text-ink outline-none focus:border-accent/50"
      />
      <button
        type="button"
        className="text-[11px] underline focus:outline-none focus:ring-2 focus:ring-accent"
        onClick={() => {
          const parsed = fee.trim() === "" ? null : Number.parseFloat(fee);
          if (parsed !== null && Number.isNaN(parsed)) {
            setStatus("enter a number");
            return;
          }
          localStorage.setItem(`aij.fee.${account.id}`, fee.trim());
          void accountSetFee(account.id, parsed).then(
            () => setStatus("saved"),
            () => setStatus("save failed"),
          );
        }}
      >
        save
      </button>
      {status && (
        <span role="status" className="text-[10px] text-faint">
          {status}
        </span>
      )}
    </span>
  );
}

function RenewalEditor({ account }: { account: AccountInfo }) {
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const focus = () => {
      if (inputRef.current?.showPicker) {
        inputRef.current.showPicker();
      } else {
        inputRef.current?.focus();
      }
    };
    window.addEventListener("aij:edit-renewal", focus);
    return () => window.removeEventListener("aij:edit-renewal", focus);
  }, []);
  const commit = (next: string | null) => {
    void import("../lib/bridge")
      .then((bridge) => bridge.accountSetRenewal(account.id, next))
      .catch(() => {
        /* invalid input is rejected server-side; the next list refresh restores truth */
      });
  };
  return (
    <span className="flex flex-wrap items-center gap-1.5 text-[11px]" data-testid="renewal-editor">
      <label htmlFor={`renewal-${account.id}`} className="text-muted">
        Renewal date (optional)
      </label>
      <input
        ref={inputRef}
        id={`renewal-${account.id}`}
        type="date"
        value={account.renewal_date ?? ""}
        onChange={(event) => commit(event.target.value === "" ? null : event.target.value)}
        className="rounded-lg border border-hairline bg-surface px-1.5 py-0.5 text-[11px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      />
      <Button
        size="sm"
        variant="secondary"
        onClick={() => commit(null)}
        disabled={account.renewal_date === null}
      >
        Clear
      </Button>
      <span className="text-[10px] text-faint">Entered by you - AI Juggler does not read billing</span>
    </span>
  );
}

function QualityTab(props: PanelProps) {
  const instance = props.data.instance;
  const [quality, setQuality] = useState<import("../lib/bridge").QualitySignalsInfo | null>(null);
  const [resources, setResources] = useState<import("../lib/bridge").ProcessesInfo | null>(null);
  const [asTable, setAsTable] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    if (!instance) return;
    let alive = true;
    import("../lib/bridge")
      .then((bridge) => bridge.instanceQuality(instance.id))
      .then((result) => {
        if (alive) {
          setQuality(result);
          setLoadError(null);
        }
      })
      .catch((cause: unknown) => {
        if (alive) setLoadError(errorText(cause));
      });
    // AJ-E410 completes E409 Req.6: CPU/RAM now come from the process
    // collector's 2 s ring (labelled Measured, footnote `2s sampling`).
    import("../lib/bridge")
      .then((bridge) => bridge.instanceProcesses(instance.id))
      .then((result) => {
        if (alive) setResources(result);
      })
      .catch(() => {
        /* CPU/RAM stays honestly absent — the other signals render */
      });
    return () => {
      alive = false;
    };
  }, [instance?.id]);

  if (!instance) return <HonestEmpty tab="quality" />;

  const signals: { testId: string; label: string; body: React.ReactNode; tier: "Measured" | "Computed" | "Estimated"; reason?: string }[] = [];
  if (loadError) {
    return (
      <Card className="flex flex-col gap-1 p-4" data-testid="quality-tab">
        <p className="text-[11.5px] text-muted">Signals unavailable right now.</p>
        <p className="text-[10px] text-muted">{loadError}</p>
      </Card>
    );
  }
  if (quality) {
    signals.push({
      testId: "quality-tokens",
      label: "Tokens / min",
      tier: "Measured",
      body: quality.tokens_bars ? (
        <div className="flex flex-col gap-1">
          <span className="tabular-nums text-ink">
            {Math.round(quality.avg_tokens_per_min)} avg (5 min) · idle is a real 0
          </span>
          {asTable ? (
            <table className="text-[10px]">
              <tbody>
                <tr>
                  <td className="pr-2 text-muted">minutes ago</td>
                  {[5, 4, 3, 2, 1].map((m) => (
                    <td key={m} className="pr-2 text-muted">{m}</td>
                  ))}
                </tr>
                <tr>
                  <td className="pr-2 text-muted">tokens</td>
                  {quality.tokens_bars.map((bar, i) => (
                    <td key={i} className="pr-2 tabular-nums text-ink">{bar}</td>
                  ))}
                </tr>
              </tbody>
            </table>
          ) : (
            <div className="flex items-end gap-1" aria-hidden="true">
              {quality.tokens_bars.map((bar, i) => (
                <span key={i} className="w-3 rounded-sm bg-accent/70" style={{ height: `${Math.max(3, (bar / Math.max(...quality.tokens_bars!)) * 24)}px` }} />
              ))}
            </div>
          )}
        </div>
      ) : (
        <span className="text-muted">Nothing measured yet</span>
      ),
    });
    const lastSample = resources?.spark.at(-1);
    signals.push({
      testId: "quality-cpu",
      label: "CPU / RAM now",
      tier: "Measured",
      body: lastSample ? (
        <span className="tabular-nums text-ink">
          {lastSample.cpu_pct !== null ? `${Math.round(lastSample.cpu_pct)}% CPU` : "CPU: first sample pending"} ·{" "}
          {(lastSample.rss_bytes / 1_000_000_000).toFixed(1)} GB RSS (process tree sum)
        </span>
      ) : (
        <span className="text-muted">Collecting first samples…</span>
      ),
    });
    signals.push({
      testId: "quality-errors",
      label: "Errors",
      tier: "Measured",
      body: quality.errors_not_measurable ? (
        <span className="text-muted">Not measurable (interactive)</span>
      ) : (
        <span className="tabular-nums text-ink">{quality.error_label}</span>
      ),
    });
    signals.push({
      testId: "quality-context",
      label: "Context",
      tier: "Measured",
      body:
        quality.context_used !== null && quality.context_window ? (
          <span className="tabular-nums text-ink">
            {Math.round((quality.context_used / quality.context_window) * 100)}% ({quality.context_used.toLocaleString()} / {quality.context_window.toLocaleString()})
          </span>
        ) : (
          <span className="text-muted">
            Context: not reported by {providerDisplayName(instance.provider)} · cumulative{" "}
            <span className="tabular-nums text-ink">
              {(quality.cumulative.input + quality.cumulative.output).toLocaleString()}
            </span>{" "}
            tokens (cumulative, not context)
          </span>
        ),
    });
    signals.push({
      testId: "quality-cost",
      label: "Session cost",
      tier: "Estimated",
      body: quality.session_cost_usd !== null ? (
        <span className="tabular-nums text-ink">
          \${quality.session_cost_usd.toFixed(2)} · Est · value-equiv. ({quality.priced_model})
        </span>
      ) : (
        <span className="text-muted">
          tokens only — no pricing row for this model (never $0.00)
        </span>
      ),
    });
    signals.push({
      testId: "quality-forecast",
      label: "Quota forecast",
      tier: "Estimated",
      body: <span className="text-ink">{quality.quota_forecast}</span>,
    });
    if (quality.stuck) {
      signals.push({
        testId: "quality-stuck",
        label: "Stuck?",
        tier: "Computed",
        body: (
          <span className="text-ink">
            {quality.stuck.rule} · {quality.stuck.inputs.join(" · ")}
          </span>
        ),
      });
    }
  }

  return (
    <div className="flex flex-col gap-3" data-testid="quality-tab">
      {signals.length === 0 ? (
        <Card className="flex flex-col gap-1 p-4">
          <p className="text-[11.5px] text-muted">Nothing measured yet — this fills in as the session works.</p>
        </Card>
      ) : (
        <>
          <div className="flex items-center justify-between">
            <p className="text-[10px] text-muted">Measured every 2 seconds</p>
            <button
              type="button"
              onClick={() => setAsTable((v) => !v)}
              aria-pressed={asTable}
              className="rounded-lg border border-hairline px-2 py-0.5 text-[10px] text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              {asTable ? "View as chart" : "View as table"}
            </button>
          </div>
          {signals.map((signal) => (
            <Card key={signal.testId} className="flex flex-col gap-1 p-3" data-testid={signal.testId}>
              <p className="flex items-center gap-2 text-[11px] uppercase tracking-wider text-muted">
                {signal.label} <TierTag tier={signal.tier} />
              </p>
              {signal.body}
            </Card>
          ))}
        </>
      )}
    </div>
  );
}

/// Formats bytes as MB/GB for the RAM column.
function formatRss(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  return `${Math.round(bytes / 1_000_000)} MB`;
}

/// Formats an age in ms as `42m` / `12s` / `2h 3m`.
function formatAge(ms: number): string {
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

/// The Instance panel's Cleanup rows (AJ-E506 Req.6): the global
/// Centre's data, filtered to this Instance — same ProcessRow, same
/// labels, no second data path.
function InstanceCleanupRows(props: { sessionId: string }) {
  const [rows, setRows] = useState<Array<import("./ProcessRow").CleanupRowInfo> | null>(null);
  useEffect(() => {
    let alive = true;
    import("../lib/bridge")
      .then((bridge) => bridge.cleanupInventory("all-managed"))
      .then((data) => {
        if (alive) {
          setRows(
            data.groups.find((group) => group.session === props.sessionId)?.rows ?? [],
          );
        }
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, [props.sessionId]);

  if (rows === null || rows.length === 0) return null;
  return (
    <section className="mt-3" data-testid="instance-cleanup-rows">
      <h4 className="text-[10.5px] font-medium text-ink">Ledger rows (read-only)</h4>
      <ul className="mt-1">
        {rows.map((row) => (
          <ProcessRow key={row.pid} row={row} />
        ))}
      </ul>
    </section>
  );
}

/// The live Processes tab (AJ-E410): the tree table (PID / Process /
/// CPU / RAM / Age / ⑂), the I-07 sparkline, and the orphan section
/// with verified stop actions. Refreshes every 5 s; failures render a
/// banner with a manual Retry — never a silently empty table.
function ProcessesTab(props: PanelProps & { readOnly?: boolean }) {
  const instance = props.data.instance;
  const [info, setInfo] = useState<import("../lib/bridge").ProcessesInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmPid, setConfirmPid] = useState<number | null>(null);
  const [confirmAll, setConfirmAll] = useState(false);
  const [verifyNotes, setVerifyNotes] = useState<Record<number, string>>({});
  const [reloadKey, setReloadKey] = useState(0);
  const readOnly = props.readOnly ?? false;

  // Load shape mirrors QualityTab (top-level import in the effect):
  // the vitest mocker intercepts this form; a wrapped `load()` closure
  // resolved the unmocked module in vitest 5 (see processes-tab tests).
  const bridge = props.processesBridge;
  useEffect(() => {
    if (!instance) return;
    let alive = true;
    let timer: ReturnType<typeof setInterval> | null = null;
    const load = () => {
      bridge
        .load(instance.id)
        .then((result) => {
          if (alive) {
            setInfo(result);
            setError(null);
          }
        })
        .catch((cause: unknown) => {
          if (alive) setError(errorText(cause));
        });
    };
    load();
    timer = setInterval(load, 5_000);
    return () => {
      alive = false;
      if (timer) clearInterval(timer);
    };
  }, [instance?.id, reloadKey, bridge]);

  if (!instance) return <HonestEmpty tab="processes" />;

  // AJ-E510 Req.4: the detached notice for re-adopted Instances — the
  // pre-crash terminal output is gone; the verified stop path (the
  // E507 sheet) is the way to end it.
  const adoptedNotice = instance.adopted ? (
    <Card className="flex flex-col gap-1 p-3" data-testid="instance-detached-notice">
      <p className="text-[11.5px] font-medium text-ink">Running — re-adopted after restart</p>
      <p className="text-[10.5px] text-muted">
        The terminal is detached: output from before the app closed is not available. Stop
        runs through the verified sheet (Terminate… on a row below).
      </p>
    </Card>
  ) : null;

  if (error) {
    return (
      <Card className="flex flex-col gap-2 p-4" data-testid="processes-error" role="alert">
        <p className="text-[11.5px] font-medium text-ink">Process tree read failed</p>
        <p className="text-[10px] break-all text-muted">{error}</p>
        <Button size="sm" variant="secondary" onClick={() => setReloadKey((key) => key + 1)}>
          Retry
        </Button>
      </Card>
    );
  }

  if (info?.unavailable_reason) {
    return (
      <Card className="flex flex-col gap-1 p-4" data-testid="processes-unavailable">
        <p className="text-[11.5px] text-muted">{info.unavailable_reason}</p>
        <p className="text-[10px] text-faint">
          Adopted or desktop-kind sessions have no readable process tree — resources are
          unavailable, never flat zero.
        </p>
      </Card>
    );
  }

  const rows = info?.rows ?? [];
  const orphans = info?.orphans ?? [];
  const spark = info?.spark ?? [];
  const lastSpark = spark.at(-1);

  const stopOne = (pid: number) => {
    setConfirmPid(null);
    bridge
      .stop(instance.id, pid)
      .then((outcome) => {
          setInfo((prev) =>
            prev
              ? {
                  ...prev,
                  orphans: prev.orphans.map((orphan) =>
                    orphan.pid === pid ? { ...orphan, stop_state: outcome.state } : orphan,
                  ),
                }
              : prev,
          );
        })
        .catch(() => {
          /* the 5 s refresh re-reads the row state */
        });
  };

  const stopAll = () => {
    setConfirmAll(false);
    bridge
      .stopAll(instance.id)
      .then((outcomes) => {
          setInfo((prev) =>
            prev
              ? {
                  ...prev,
                  orphans: prev.orphans.map((orphan) => {
                    const outcome = outcomes.find((row) => row.pid === orphan.pid);
                    return outcome ? { ...orphan, stop_state: outcome.state } : orphan;
                  }),
                }
              : prev,
          );
        })
        .catch(() => {
          /* the 5 s refresh re-reads the row states */
        });
  };

  const verifyOne = (pid: number) => {
    bridge
      .verify(instance.id, pid)
      .then((result) => setVerifyNotes((notes) => ({ ...notes, [pid]: result.note })))
      .catch(() => {
        /* verification is manual — failures keep the row as-is */
      });
  };

  return (
    <div className="flex flex-col gap-3" data-testid="processes-tab">
      {adoptedNotice}
      <div className="flex items-center justify-between gap-2">
        <p className="text-[10px] text-muted">2s sampling · 5s refresh · Measured</p>
        {lastSpark && (
          <span className="flex items-end gap-[2px]" aria-hidden="true" data-testid="processes-spark">
            {spark.slice(-30).map((sample, i) => (
              <span
                key={i}
                className="w-[3px] rounded-sm bg-accent/70"
                style={{
                  height: `${Math.max(2, ((sample.cpu_pct ?? 0) / 100) * 18)}px`,
                }}
              />
            ))}
          </span>
        )}
      </div>

      {/* AJ-E804: a scrollable region is reachable by keyboard. */}
      <Card className="flex flex-col gap-1 overflow-x-auto p-3" tabIndex={0} role="region" aria-label="Process table">
        <table className="w-full min-w-[420px] text-left text-[11px]">
          <thead>
            <tr className="text-[10px] uppercase tracking-wider text-muted">
              <th scope="col" className="py-1 pr-2 font-medium">PID</th>
              <th scope="col" className="py-1 pr-2 font-medium">Process</th>
              <th scope="col" className="py-1 pr-2 font-medium">CPU</th>
              <th scope="col" className="py-1 pr-2 font-medium">RAM</th>
              <th scope="col" className="py-1 pr-2 font-medium">Age</th>
              <th scope="col" className="py-1 font-medium">⑂</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={6} className="py-3 text-center text-[11px] text-muted" data-testid="processes-empty">
                  No subprocesses
                </td>
              </tr>
            ) : (
              rows.map((row) => (
                <tr key={row.pid} className="border-t border-hairline">
                  <td className="py-1 pr-2 font-mono text-[10.5px] tabular-nums text-muted">{row.pid}</td>
                  <td className="py-1 pr-2 text-ink">
                    <span aria-hidden="true" className="text-faint">{"│ ".repeat(row.depth)}{row.depth > 0 ? "└ " : ""}</span>
                    {row.name}
                  </td>
                  <td className="py-1 pr-2 tabular-nums text-ink">
                    {row.cpu_pct !== null ? `${Math.round(row.cpu_pct)}%` : <span className="text-faint">— first sample</span>}
                  </td>
                  <td className="py-1 pr-2 tabular-nums text-ink">{formatRss(row.rss_bytes)}</td>
                  <td className="py-1 pr-2 tabular-nums text-muted">{formatAge(row.age_ms)}</td>
                  <td className="py-1 tabular-nums text-muted">{row.child_count}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
        {readOnly && (
          <p className="pt-1 text-[10px] text-faint">
            Read-only — ended conversation; stop actions live on the session&apos;s Processes tab.
          </p>
        )}
      </Card>

      {!readOnly && (
        <Card className="flex flex-col gap-2 p-3" data-testid="processes-orphans">
          <p className="text-[11px] uppercase tracking-wider text-muted" data-testid="orphans-heading">
            {orphans.length > 0 ? `Orphaned - left behind (${orphans.length})` : "No orphaned processes"}
          </p>
          {orphans.length === 0 ? (
            <p className="text-[10px] text-faint">
              A good empty state: nothing this session spawned outlived it.
            </p>
          ) : (
            <>
              {orphans.map((orphan) => (
                <div key={orphan.pid} className="flex flex-col gap-1 border-t border-hairline pt-2" data-testid={`orphan-row-${orphan.pid}`}>
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px]">
                    <span className="font-mono text-[10.5px] tabular-nums text-muted">{orphan.pid}</span>
                    <span className="text-ink">{orphan.name}</span>
                    <span className="tabular-nums text-muted">{formatRss(orphan.rss_bytes)}</span>
                    <span className="tabular-nums text-muted">since exit {formatAge(orphan.age_since_parent_exit_ms)}</span>
                    <span className="text-faint">parent: {orphan.parent_label}</span>
                    {orphan.stop_state && (
                      <span
                        data-testid={`orphan-state-${orphan.pid}`}
                        className={orphan.stop_state === "gone" ? "text-success" : "text-warning"}
                      >
                        {orphan.stop_state === "gone" ? "gone ✓" : orphan.stop_state}
                      </span>
                    )}
                  </div>
                  {verifyNotes[orphan.pid] && (
                    <p className="text-[10px] text-muted" data-testid={`orphan-verify-${orphan.pid}`}>
                      Verify owner: {verifyNotes[orphan.pid]}
                    </p>
                  )}
                  <div className="flex items-center gap-2">
                    <Button size="sm" variant="secondary" onClick={() => verifyOne(orphan.pid)}>
                      Verify owner
                    </Button>
                    {!orphan.stop_state && (
                      <Button size="sm" variant="danger" onClick={() => setConfirmPid(orphan.pid)}>
                        Stop process {orphan.pid}
                      </Button>
                    )}
                  </div>
                  {confirmPid === orphan.pid && (
                    <div className="flex flex-col gap-2 rounded-lg border border-danger/40 bg-danger/5 p-2" data-testid={`orphan-confirm-${orphan.pid}`} role="alertdialog" aria-label={`Stop process ${orphan.pid}`}>
                      <p className="text-[11px] text-ink">
                        Stop process <span className="font-mono">{orphan.pid}</span> ({orphan.name})? SIGTERM
                        first, SIGKILL after a grace period — only this journal-recorded PID.
                      </p>
                      <div className="flex gap-2">
                        <Button size="sm" variant="danger" onClick={() => stopOne(orphan.pid)}>
                          Stop {orphan.pid}
                        </Button>
                        <Button size="sm" variant="secondary" onClick={() => setConfirmPid(null)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              ))}
              {orphans.some((orphan) => !orphan.stop_state) && (
                <div className="flex flex-col gap-2 pt-1">
                  <Button size="sm" variant="danger" onClick={() => setConfirmAll(true)}>
                    Stop all orphans…
                  </Button>
                  {confirmAll && (
                    <div className="flex flex-col gap-2 rounded-lg border border-danger/40 bg-danger/5 p-2" data-testid="orphans-confirm-all" role="alertdialog" aria-label="Stop all orphans">
                      <p className="text-[11px] text-ink">
                        Stop all orphaned processes — {orphans.filter((orphan) => !orphan.stop_state).map((orphan) => orphan.pid).join(", ")}?
                        Each gets the SIGTERM → SIGKILL ladder and a re-read.
                      </p>
                      <div className="flex gap-2">
                        <Button size="sm" variant="danger" onClick={stopAll}>
                          Stop all
                        </Button>
                        <Button size="sm" variant="secondary" onClick={() => setConfirmAll(false)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </>
          )}
          {/* AJ-E506: the SAME Cleanup rows as the global Centre
          (one data path, one label vocabulary — the shared ProcessRow;
          divergence here is how T3 survivors get mislabelled). */}
          <InstanceCleanupRows sessionId={instance.id} />
          {/* AJ-E505: the machine-wide port occupancy map below the
          Session tree — every listener classified, unmanaged rows
          display-only (the F3 collateral guard). */}
          <div className="mt-3">
            <PortInventoryList />
          </div>
        </Card>
      )}
    </div>
  );
}

/// Read-only process rollups for Provider / Account / Project
/// selections (Req.2): every member Instance's tree, labelled with its
/// Instance, with the stop-actions note pointing at the Instance tab.
function AggregateProcesses(props: PanelProps) {
  const kind = props.kind;
  const data = props.data;
  const members = props.sessions.filter((session) => {
    if (kind === "provider") return session.provider === data.provider?.id;
    if (kind === "account") return session.account_id === data.account?.id;
    return session.cwd === data.project?.root;
  });
  const [infos, setInfos] = useState<Record<string, import("../lib/bridge").ProcessesInfo | null>>({});

  useEffect(() => {
    let alive = true;
    let timer: ReturnType<typeof setInterval> | null = null;
    const bridge = props.processesBridge;
    const load = () => {
      void Promise.all(
        members.map((member) =>
          bridge.load(member.id).then((result) => [member.id, result] as const),
        ),
      ).then((entries) => {
        if (alive) setInfos(Object.fromEntries(entries));
      });
    };
    load();
    timer = setInterval(load, 5_000);
    return () => {
      alive = false;
      if (timer) clearInterval(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind, data.provider?.id, data.account?.id, data.project?.root, members.map((m) => m.id).join(",")]);

  const totalRows = members.reduce((sum, member) => sum + (infos[member.id]?.rows.length ?? 0), 0);

  return (
    <div className="flex flex-col gap-3" data-testid="processes-aggregate">
      <p className="text-[10px] text-muted">
        Read-only rollup across {members.length} session{members.length === 1 ? "" : "s"} · stop actions
        live on each session&apos;s Processes tab
      </p>
      {totalRows === 0 ? (
        <Card className="flex flex-col gap-1 p-4">
          <p className="text-[11.5px] text-muted" data-testid="processes-empty">No subprocesses</p>
        </Card>
      ) : (
        members.map((member) => {
          const info = infos[member.id];
          const rows = info?.rows ?? [];
          if (rows.length === 0) return null;
          return (
            <Card key={member.id} className="flex flex-col gap-1 p-3">
              <p className="text-[10px] uppercase tracking-wider text-muted">
                {member.account_label} · {member.cwd?.split("/").pop() || member.cwd || member.id}
              </p>
              <table className="w-full text-left text-[11px]">
                <tbody>
                  {rows.map((row) => (
                    <tr key={`${member.id}-${row.pid}`} className="border-t border-hairline">
                      <td className="py-1 pr-2 font-mono text-[10.5px] tabular-nums text-muted">{row.pid}</td>
                      <td className="py-1 pr-2 text-ink">
                        <span aria-hidden="true" className="text-faint">{"│ ".repeat(row.depth)}{row.depth > 0 ? "└ " : ""}</span>
                        {row.name}
                      </td>
                      <td className="py-1 pr-2 tabular-nums text-ink">
                        {row.cpu_pct !== null ? `${Math.round(row.cpu_pct)}%` : <span className="text-faint">—</span>}
                      </td>
                      <td className="py-1 tabular-nums text-ink">{formatRss(row.rss_bytes)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Card>
          );
        })
      )}
    </div>
  );
}

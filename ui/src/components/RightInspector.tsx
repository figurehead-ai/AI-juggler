import { ToolChip } from "./ToolMark";
import { ParkCard } from "./ParkCard";
import { matches } from "../lib/shortcuts";
import { PageShell } from "../pages/PageShell";
import { InspectorHeader, statusTone, toolRunning } from "../pages/ItemHeader";
import { since } from "../lib/time-words";
import { pageTabs, type PageKind, type PageTabId } from "../pages/registry";
import { ProjectUsage } from "../pages/ProjectUsage";
import { ProjectOverview, ProjectSessions } from "../pages/ProjectPage";
import { AccountOverview, AccountSessions } from "../pages/AccountPage";
import { outside, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { WorktreeGit } from "./WorktreeGit";
import { identityWords } from "../lib/identity";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button, Card, Chip } from "./primitives";
import { TerminalView } from "./TerminalView";
import { DesktopPreview } from "./DesktopPreview";
import { PortInventoryList } from "./PortMap";
import { ProcessRow } from "./ProcessRow";
import { IconProjects, IconTerminal } from "./icons";
import type {
  AccountInfo,
  FocusOutcomeInfo,
  InstanceInfo,
  LaunchRecipeInfo,
  ProjectInfo,
} from "../lib/bridge";
import { AccountRunway, runwayOf } from "./AccountRunway";
import { QuotaBar } from "./primitives";
import { TwoClocks } from "./QuotaRings";
import { quotaDisplay, quotaDisplayAll, switchTargets, type QuotaDisplayData, type SwitchBackSuggestionData, type SwitchTargetData, errorText,
  signedIn,
} from "../lib/bridge";
import { formatBytes } from "./indicator";
import { resourceLine } from "../lib/machine";
import { FOUND_UNCHECKED, providerDisplayName, versionUnchecked } from "../lib/providers";
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
} from "../lib/bridge";
import { RemoteTabBody, type RemoteContext } from "./RemoteTabs";
import { launchSettingsName } from "../lib/account-words";

/**
 * RightInspector — the page of a session, a project, an account or a
 * tool, and of a paired Mac's (AJ-E705). AJ-E1533: every type opens the
 * same kind of page (pages/registry.ts, pages/PageShell.tsx): an
 * identity header, then the type's own tabs, its body first, and only
 * the tabs with something to show. Kept on its old name for its callers.
 */

export type InspectorKind = "instance" | "provider" | "account" | "project" | "machine";

export type InspectorSelection =
  | { kind: "none" }
  | { kind: "login"; id: string }
  | { kind: InspectorKind; id: string };

type EmptyTab = "overview" | "usage" | "history" | "quality" | "processes" | "actions";

/** The honest empty state (D-06), in plain words (D-25). */
function HonestEmpty({ tab }: { tab: EmptyTab }) {
  const copy: Record<EmptyTab, string> = {
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

/** The registry's page type for an inspector selection. */
function pageKindOf(kind: InspectorKind, remote: RemoteContext | null): PageKind {
  if (remote) return remote.instance ? "remote-session" : "machine";
  switch (kind) {
    case "instance":
      return "session";
    case "provider":
      return "tool";
    case "account":
    case "project":
    case "machine":
      return kind;
  }
}

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
  folded = false,
  onToggleFold,
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
  /** AJ-E1504: folded to its header line (⌘⇧I); the open tab stays. */
  folded?: boolean;
  onToggleFold?: () => void;
}) {
  const remote = data.remote ?? null;
  const { all: conversations } = useConversations();

  useEffect(() => {
    if (!remote || !onRemoteBack) return;
    const onKey = (event: KeyboardEvent) => {
      if (matches("remote-back", event)) {
        event.preventDefault();
        onRemoteBack();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [remote, onRemoteBack]);

  // D-25: with nothing selected the work area is Home (screens/Home).
  if (selection.kind === "none" || selection.kind === "login") return null;
  const kind = selection.kind;
  const page = pageKindOf(kind, remote);
  const tabs = pageTabs(page, {
    desktop: data.instance?.kind === "desktop",
    hasUsage: data.project ? projectHasUsage(data.project.id, sessions, conversations) : false,
    hasSessions: data.account
      ? sessions.some((s) => s.account_id === data.account!.id) ||
        outside(conversations).some((c) => c.account_id === data.account!.id)
      : false,
  });
  const panelProps = {
    kind,
    data,
    processesBridge,
    sessions,
    projects,
    accounts,
    profiles,
    terminalTitle,
    now,
    onOpenProject,
    onOpenSession,
    onLaunch,
    onStopped,
    onHandover,
    switchBackSuggestions,
    onSwitchBack,
    onFocusTerminal: () => window.dispatchEvent(new Event("aij:focus-handover")),
    focusBridge,
  };
  return (
    <PageShell
      // Each type keeps its own tab: a new type starts from its own memory.
      key={page}
      kind={page}
      tabs={tabs}
      testId="right-inspector"
      folded={folded}
      focusTabsRequest={focusTabsRequest}
      header={
        <InspectorHeader
          page={page}
          kind={kind}
          data={data}
          sessions={sessions}
          projects={projects}
          now={now}
          primaryAction={primaryAction}
          profiles={profiles}
          onLaunch={onLaunch}
          onOpenProject={onOpenProject}
          onStopped={onStopped}
          onRemoteBack={onRemoteBack}
          onRemoteSelect={onRemoteSelect}
          folded={folded}
          onToggleFold={onToggleFold}
        />
      }
      render={(tab, visible) =>
        remote ? (
          <RemoteTabBody
            tab={tab as InspectorTab}
            remote={remote}
            now={now}
            onSelectInstance={(id) => onRemoteSelect?.(id)}
            onActionDone={() => onRemoteActionDone?.()}
          />
        ) : (
          <InspectorPanel {...panelProps} tab={tab} visible={visible} />
        )
      }
      footer={(tab) => (
        // Sticky with a solid background: a long tab scrolls under it.
        <footer className="sticky bottom-0 z-10 mt-auto flex items-center gap-1.5 border-t border-hairline/60 bg-canvas pt-2 pb-1 text-[10px] text-muted">
          <span>
            Updated{" "}
            {since(remote ? (remote.machine.last_heard_ms ?? now) : (freshness[data.instance?.id ?? ""]?.resources ?? now), now)} ago
          </span>
          <span aria-hidden="true">·</span>
          {/* D-25: a paired Mac's source is worth saying out loud. */}
          <span>{remote ? `from ${remote.machine.label} over the Fleet link` : `from ${tab.source}`}</span>
        </footer>
      )}
    />
  );
}

/** The tab ids a paired Mac's bodies take (RemoteTabs.tsx). */
type InspectorTab = "overview" | "usage" | "history" | "quality" | "processes" | "actions";

/** A project has usage to show when any of its sessions measured some. */
function projectHasUsage(projectId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): boolean {
  const used = (t: { input: number; output: number } | null | undefined) => !!t && t.input + t.output > 0;
  return (
    sessions.some((s) => s.project_id === projectId && used(s.tokens)) ||
    outside(conversations).some((c) => c.project_id === projectId && used(c.tokens))
  );
}

function InspectorPanel(props: PanelProps & { tab: PageTabId }) {
  const { tab, kind } = props;
  switch (kind) {
    case "instance":
      switch (tab) {
        case "terminal":
        case "window":
          return <InstanceOverview {...props} />;
        case "usage":
          // AJ-E1533: Quality joins Usage — one place for what it used
          // and how it went.
          return (
            <>
              <InstanceUsage {...props} />
              <QualityTab {...props} />
            </>
          );
        case "processes":
          return <ProcessesTab {...props} readOnly={false} />;
        default:
          return null;
      }
    case "project":
      switch (tab) {
        case "sessions":
          return props.data.project ? <ProjectSessions {...props} project={props.data.project} /> : null;
        case "overview":
          return props.data.project ? <ProjectOverview {...props} project={props.data.project} /> : null;
        case "usage":
          return <ProjectUsage projectId={props.data.project?.id ?? ""} sessions={props.sessions} />;
        default:
          return null;
      }
    case "account":
      switch (tab) {
        case "overview":
          return props.data.account ? <AccountOverview {...props} account={props.data.account} /> : null;
        case "sessions":
          return props.data.account ? <AccountSessions {...props} account={props.data.account} /> : null;
        default:
          return null;
      }
    case "provider":
      return tab === "overview" ? <ProviderOverview {...props} /> : null;
    case "machine":
      return null;
  }
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
            <ToolChip tool={session.provider} />
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
              <ToolChip tool={session.provider} />
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
                  : "app exited · Relaunch, above, starts it again"}
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
                    <span className="text-muted">{launchSettingsName(recipe.name)}</span>
                    {recipe.model && <span className="ml-auto font-mono text-[10px] text-faint">{recipe.model}</span>}
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
              <tr><td className="text-muted">read from cache</td><td className="text-right tabular-nums text-ink">{tokens.cache_read.toLocaleString()}</td></tr>
              <tr><td className="text-muted">written to cache</td><td className="text-right tabular-nums text-ink">{tokens.cache_write.toLocaleString()}</td></tr>
              <tr className="border-t border-hairline"><td className="text-muted">total (input + output)</td><td className="text-right tabular-nums font-medium text-ink">{(tokens.input + tokens.output).toLocaleString()}</td></tr>
            </tbody>
          </table>
        ) : (
          <p className="text-[11.5px] text-muted">No usage recorded yet</p>
        )}
        <span className="text-[10px] text-muted">
          <TierTag tier="Measured" />
        </span>
      </Card>
      <Card className="flex flex-col gap-2 p-4" data-testid="two-clock-card">
        <p className="text-[11px] uppercase tracking-wider text-muted">This account's limits</p>
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
      if (versionUnchecked(install)) return FOUND_UNCHECKED;
      return "Couldn't check what's installed";
  }
}

function ProviderOverview(props: PanelProps) {
  const provider = props.data.provider;
  const install = useToolInstall(provider?.id ?? null);
  const { all: conversations } = useConversations();
  // One source per fact: each account's line is Runway's own (runwayOf).
  const [quota, setQuota] = useState<QuotaDisplayData[]>([]);
  useEffect(() => {
    let alive = true;
    quotaDisplayAll().then(
      (rows) => alive && setQuota(rows),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [provider?.id]);
  if (!provider) return <HonestEmpty tab="overview" />;
  const providerAccounts = props.accounts.filter((a) => a.provider === provider.id);
  const providerSessions = props.sessions.filter((s) => s.provider === provider.id);
  // One count everywhere: its sessions running now, outside ones too (AJ-E1529).
  const running = toolRunning(provider.id, props.sessions, conversations);
  // Each account's runway, the least left first (recognition over
  // recall); each line is the one Home, Usage and Accounts show.
  const perAccount = providerAccounts
    .map((account) => {
      const display = quota.find((q) => q.account === account.id);
      const sessions = providerSessions.filter((s) => s.account_id === account.id);
      return { account, display, sessions, left: runwayOf(display, sessions).left };
    })
    .sort((a, b) => (a.left ?? Number.POSITIVE_INFINITY) - (b.left ?? Number.POSITIVE_INFINITY));
  return (
    <div className="flex flex-col gap-3" data-testid="provider-overview">
      <Card className="flex flex-col gap-2 p-4">
        <div className="flex flex-wrap items-center gap-2 text-[11.5px]">
          <span className="text-ink">
            {providerAccounts.length} account{providerAccounts.length === 1 ? "" : "s"}
          </span>
          <span className="text-muted">· {running} running</span>
        </div>
        {/* Shown once known; nothing in its place until then (D-25). */}
        {install !== null && (
          <p className="text-[11px] text-muted" data-testid="provider-install">
            {install === "failed" ? "Couldn't check what's installed" : installWords(install)}
          </p>
        )}
      </Card>
      <Card className="flex flex-col gap-1.5 p-4" data-testid="provider-aggregate">
        <p className="text-[11px] uppercase tracking-wider text-muted">Runway, least left first</p>
        {perAccount.length === 0 ? (
          <p className="text-[11.5px] text-muted">No accounts for this tool yet.</p>
        ) : (
          perAccount.map(({ account, display, sessions }) => (
            <div key={account.id} className="flex flex-col gap-1.5 py-1">
              <span className="text-[11.5px]">
                <span className="text-ink">{account.label}</span>
                {!signedIn(account.login_state) ? <span className="text-faint"> · signed out</span> : null}
              </span>
              {/* D-25: a signed-out account with no reading says "signed
                  out" above; its runway would only repeat it. */}
              {display || signedIn(account.login_state) ? <AccountRunway display={display} sessions={sessions} /> : null}
            </div>
          ))
        )}
      </Card>
    </div>
  );
}

// --- AJ-E409: Quality/Performance tab (D-07 — signals, never a score) ---

/** SPEC §6's honesty tiers, each with a plain line beside it (0.0.11
 * walk, Q5): the tag stays, and says what it means. */
const TIER_WORDS: Record<"Measured" | "Computed" | "Estimated", string> = {
  Measured: "read from the session",
  Computed: "worked out from what's measured",
  Estimated: "a best guess",
};

function TierTag({ tier }: { tier: "Measured" | "Computed" | "Estimated" }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className="rounded border border-hairline px-1 text-[9px] font-medium uppercase tracking-wide text-muted">
        {tier}
      </span>
      <span className="text-[10px] font-normal normal-case tracking-normal text-faint" data-testid="tier-words">
        {TIER_WORDS[tier]}
      </span>
    </span>
  );
}

/** A stuck rule's id in words (the rail's own wording). */
function stuckRuleWords(rule: string): string {
  switch (rule) {
    case "stuck_no_output":
      return "No output for a while";
    case "waiting_prompt":
      return "Waiting for your reply";
    case "quota_exit":
      return "Hit its limit";
    default:
      return "Looks stuck";
  }
}

function QualityTab(props: PanelProps) {
  const instance = props.data.instance;
  const [quality, setQuality] = useState<import("../lib/bridge").QualitySignalsInfo | null>(null);
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
            {Math.round(quality.avg_tokens_per_min)} a minute on average over the last 5 minutes
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
    // AJ-E1533: one source per fact — the same live reading the
    // session's header shows, kept current by the live updates.
    const measured = instance.cpu_pct !== null || instance.rss_bytes !== null;
    signals.push({
      testId: "quality-cpu",
      label: "CPU and memory now",
      tier: "Measured",
      body: measured ? (
        <span className="tabular-nums text-ink">{resourceLine(instance.cpu_pct, instance.rss_bytes, instance.child_count)}</span>
      ) : (
        <span className="text-muted">Collecting first samples…</span>
      ),
    });
    signals.push({
      testId: "quality-errors",
      label: "Errors",
      tier: "Measured",
      body: quality.errors_not_measurable ? (
        <span className="text-muted">Can't be counted in an interactive session</span>
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
            {providerDisplayName(instance.provider)} doesn't say how full its context is ·{" "}
            <span className="tabular-nums text-ink">
              {(quality.cumulative.input + quality.cumulative.output).toLocaleString()}
            </span>{" "}
            tokens used in this session so far
          </span>
        ),
    });
    signals.push({
      testId: "quality-cost",
      label: "Session cost",
      tier: "Estimated",
      body: quality.session_cost_usd !== null ? (
        <span className="tabular-nums text-ink">
          about ${quality.session_cost_usd.toFixed(2)} at {quality.priced_model}'s API prices
        </span>
      ) : (
        <span className="text-muted">
          No price known for this model, so no cost is shown
        </span>
      ),
    });
    signals.push({
      testId: "quality-forecast",
      label: "How long its limit lasts",
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
            {stuckRuleWords(quality.stuck.rule)} · {quality.stuck.inputs.join(" · ")}
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
      <p className="text-[11.5px] font-medium text-ink">Running — picked up after restart</p>
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
        <p className="text-[10px] text-muted">Measured every 2 seconds</p>
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
              <th scope="col" className="py-1 pr-2 font-medium">Memory</th>
              <th scope="col" className="py-1 pr-2 font-medium">Age</th>
              <th scope="col" className="py-1 font-medium">Helpers</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={6} className="py-3 text-center text-[11px] text-muted" data-testid="processes-empty">
                  {/* One source per fact: never "none" while the header counts helpers. */}
                  {(instance.child_count ?? 0) > 0 ? "Reading the process tree…" : "No subprocesses"}
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
                    {row.cpu_pct !== null ? `${Math.round(row.cpu_pct)}%` : <span className="text-faint">measuring…</span>}
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
              Nothing this session started is still running after it.
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

import { ToolChip } from "./ToolMark";
import { matches } from "../lib/shortcuts";
import { PageShell } from "../pages/PageShell";
import { InspectorHeader, statusTone, toolRunning } from "../pages/ItemHeader";
import { since } from "../lib/time-words";
import { pageTabs, type PageKind, type PageTabId } from "../pages/registry";
import { ProjectTab, useProjectTabCounts } from "../pages/project/ProjectTabs";
import { AccountOverview, AccountSessions } from "../pages/AccountPage";
import { outside, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { SessionNotes } from "./SessionNotes";
import { ToolUpdateLine } from "./ToolUpdateLine";
import { identityWords } from "../lib/identity";
import { useEffect, useState, type ReactNode } from "react";
import { Button, Card, Chip } from "./primitives";
import { lazyNamed } from "../lib/lazy";

// xterm loads with the first terminal shown, not at start.
const TerminalView = lazyNamed(() => import("./TerminalView"), "TerminalView", true);
// The verified stop sheet loads when a Stop is chosen (lazy-load rule).
const OwnSessionOverview = lazyNamed(() => import("../pages/session/OwnOverview"), "OwnSessionOverview");
const DrivenPanel = lazyNamed(() => import("../pages/session/DrivenPanel"), "DrivenPanel");
const OwnSessionUsage = lazyNamed(() => import("../pages/session/OwnUsage"), "OwnSessionUsage");
import { DesktopPreview } from "./DesktopPreview";
import { ProcessesView } from "../screens/processes/ProcessesView";
import { IconProjects, IconTerminal } from "./icons";
import type { AccountInfo, FocusOutcomeInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";
import { SessionTab, useSessionTabs } from "../pages/session/session-tabs";
import { WhenShown } from "../pages/PageShell";
import { AccountRunway, runwayOf } from "./AccountRunway";
import { QuotaBar } from "./primitives";
import { sharedReads, useSharedRead } from "../lib/shared-read";
import { quotaDisplayAll, type QuotaDisplayData, type SwitchBackSuggestionData, errorText,
  signedIn,
} from "../lib/bridge";
import { resourceLine } from "../lib/machine";
import { modelName } from "./charts/colors";
import { useCached } from "../lib/usage/cached";
import { usageHistory, type UsageHistoryInfo } from "../lib/ipc/usageHistory";
import { installWords, providerDisplayName } from "../lib/providers";
import { useToolInstall } from "../lib/useToolInstall";
import { OwnTerminalOverview } from "../pages/session/OwnTerminalOverview";
import { RemoteTabBody, type RemoteContext } from "./RemoteTabs";
import { launchSettingsName } from "../lib/account-words";
import { memorySize } from "../lib/compact";

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
      <p className="text-label font-medium text-muted">Unavailable</p>
      <p className="max-w-xs text-label leading-relaxed text-muted">{copy[tab]}</p>
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
  const projectCounts = useProjectTabCounts(data.project, sessions);
  const projectUsage = useProjectHasUsage(data.project?.id ?? null, data.project ? workingSetHasUsage(data.project.id, sessions, conversations) : false);

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

  const sessionTabs = useSessionTabs(remote ? null : (data.instance ?? null));
  // D-25: with nothing selected the work area is Home (screens/Home).
  if (selection.kind === "none" || selection.kind === "login") return null;
  const kind = selection.kind;
  const page = pageKindOf(kind, remote);
  const tabs = pageTabs(page, {
    desktop: data.instance?.kind === "desktop",
    ...sessionTabs,
    hasUsage: projectUsage,
    counts: projectCounts,
    hasSessions: data.account
      ? sessions.some((s) => s.account_id === data.account!.id) ||
        outside(conversations).some((c) => c.account_id === data.account!.id)
      : false,
  });
  const panelProps = {
    kind,
    data,
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
      subject={selection.id}
      tabsKnown={page !== "session" || remote !== null || sessionTabs.known}
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
        <footer className="sticky bottom-0 z-10 mt-auto flex items-center gap-1.5 border-t border-hairline/60 bg-canvas pt-2 pb-1 text-label text-muted">
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

/** A project has usage to show when the usage history holds any of it,
 * at any time (0.1.5: its summaries, not the sessions the app holds
 * now); until that answer comes, what the working set says, so the tab
 * doesn't blink. */
function useProjectHasUsage(projectId: string | null, meanwhile: boolean): boolean {
  const result = useCached<UsageHistoryInfo>(
    projectId ? `project-has-usage:${projectId}` : null,
    () => usageHistory({ edges_ms: [0, Date.now()], group_by: "profile", profile: null, project: projectId, top: 0 }),
    60_000,
  );
  if (!projectId) return false;
  const summary = result.current ? result.data : null;
  return summary ? summary.totals.messages > 0 || summary.session_count > 0 || meanwhile : meanwhile;
}

/** What the sessions the app holds now say: any measured some. */
function workingSetHasUsage(projectId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): boolean {
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
        case "overview": // AJ-E2108: where it runs and its last messages.
          return props.data.instance ? <OwnSessionOverview {...props} session={props.data.instance} /> : null;
        case "usage":
          // 0.1.5: the same Usage tab as any session's (its charts and its
          // account's window), then how it's going (AJ-E1533: Quality
          // joins Usage).
          return props.data.instance ? (
            <div className="flex flex-col gap-grid" data-testid="instance-usage">
              <OwnSessionUsage session={props.data.instance} visible />
              <QualityTab {...props} />
            </div>
          ) : (
            <HonestEmpty tab="usage" />
          );
        case "processes":
          return <ProcessesTab {...props} readOnly={false} />;
        case "messages":
        case "tasks":
        case "subagents":
        case "history":
          return props.data.instance ? <SessionTab tab={tab} session={props.data.instance} now={props.now} visible={props.visible} onOpenProject={props.onOpenProject} /> : null;
        default:
          return null;
      }
    case "project": // 0.3: its seven tabs (pages/project/ProjectTabs).
      return props.data.project ? <ProjectTab {...props} project={props.data.project} visible={props.visible} /> : null;
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

type PanelProps = {
  kind: InspectorKind;
  /** True while this panel's tab is the shown one (previews pause
   *  otherwise — AJ-E415 Req.1 "not visible"). */
  visible: boolean;
  data: InspectorData;
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
  // AJ-E414: which focus tier the last Open/Focus attempt achieved —
  // shown verbatim, never overstated. A missing Accessibility grant
  // additionally raises the one-time explainer (Req.3).
  const [focusOutcome, setFocusOutcome] = useState<FocusOutcomeInfo | null>(null);
  const [explainerDismissed, setExplainerDismissed] = useState(
    () => localStorage.getItem("aij.a11y-explainer.dismissed") === "1",
  );
  const { kind, data, sessions, accounts, profiles, terminalTitle } = props;

  if (kind === "instance" && data.instance) {
    const session = data.instance;
    // AJ-E413: a desktop Instance has no terminal — the desktop surface
    // replaces the terminal card, and `Open / Focus window` is primary.
    const isDesktop = session.kind === "desktop";
    const terminalId = session.id;
    const live = session.status === "running" || session.status === "starting";
    const project = props.projects.find((p) => p.id === session.project_id) ?? null;
    // §5.1 Overview fields (every field honest: absent → named empty,
    // never fabricated). Lineage is omitted entirely until the Switch
    // Graph exists (E2) — no fabricated A→B links.
    const fields = (
      <div className={"flex flex-col gap-2 " + (isDesktop ? "" : "border-t border-hairline pt-3")} data-testid="instance-overview-fields">
        {/* The header already says the state, the account and its low
            runway (D-25); this row adds only when it ended and the
            identity check. */}
        <div className="flex flex-wrap items-center gap-1.5 text-label empty:hidden">
          {["finished", "stopped", "failed", "handed_over"].includes(session.status) && session.last_activity_ms ? (
            <span className="text-meta text-muted" data-testid="session-ended-stamp">
              Ended{" "}
              {new Date(session.last_activity_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false })}
            </span>
          ) : null}
          {identityWords(session) && (
            <span data-testid="instance-identity" data-state={identityWords(session)!.state} className="min-w-0">
              {/* A verified sign-in is a chip; the longer sentences wrap as
                  a line (0.3.2: one ran past the card's edge). */}
              {identityWords(session)!.state === "verified" ? (
                <Chip tone="success">{identityWords(session)!.text}</Chip>
              ) : (
                <span className="text-label text-ink-2">{identityWords(session)!.text}</span>
              )}
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
              <p data-testid="instance-model-line" className="text-label text-ink">
                {drift} <span className="text-muted">(observed in transcript)</span>
              </p>
            );
          }
          if (session.running_model === null && session.launch_model === null) {
            return (
              <p data-testid="instance-model-line" className="text-label text-muted">
                Model: not reported by {providerDisplayName(session.provider)}
              </p>
            );
          }
          return null;
        })()}
        <SessionNotes session={session} live={live} now={props.now} />
        <details className="group text-meta" data-testid="instance-details">
          {/* Says what's inside (the review of 0.1.4: an empty strip). */}
          <summary className="w-fit cursor-pointer select-none rounded text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
            Details <span className="text-muted">· launch settings and session id</span>
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
            {/* The tool's own id for the conversation (never "Claude" for every tool). */}
            Conversation id in {providerDisplayName(session.provider)}:{" "}
            <span className="font-mono text-ink">
              {session.conversation_id ?? "not found yet"}
            </span>
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-label">
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
            cost: <span className="text-muted">—</span>
          </span>
          <span data-testid="instance-resources" className="tabular-nums text-muted">
            cpu {session.cpu_pct !== null ? `${Math.round(session.cpu_pct)}%` : "—"} · ram{" "}
            {session.rss_bytes !== null ? memorySize(session.rss_bytes) : "—"} · more processes{" "}
            {session.child_count ?? "—"}
          </span>
        </div>
          </div>
        </details>
      </div>
    );
    // A terminal's session: the page every session has (its terminal the
    // main pane, the side column beside it).
    if (!isDesktop) {
      return (
        <OwnTerminalOverview
          session={session}
          project={project}
          onOpenProject={props.onOpenProject}
          details={fields}
          switchBackSuggestions={props.switchBackSuggestions}
          onSwitchBack={props.onSwitchBack}
          onHandover={props.onHandover}
          terminal={
            // AJ-E2302: a driven session's conversation is its main pane;
            // a terminal one says why it isn't driven, when it fell back.
            session.runtime?.kind === "driven" ? (
              <DrivenPanel session={session} onOpenSession={props.onOpenSession} />
            ) : (
              <div className="flex min-h-0 flex-1 flex-col gap-2">
                {session.runtime?.fell_back && <p className="text-meta text-ink-2" data-testid="runtime-fell-back">{`In its terminal: ${session.runtime.fell_back}.`}</p>}
                <Card className="min-h-[280px] flex-1 overflow-hidden bg-[#05070b] p-0">
                  <TerminalView id={terminalId} title={terminalTitle ?? session.account_label} status={session.status} rule={session.attention?.rule ?? null} />
                </Card>
                {!live && <p className="text-meta text-ink-2">{`ran ${since(session.started_at_ms, Date.now())} · scrollback replayed`}</p>}
              </div>
            )
          }
        />
      );
    }
    return (
      <div className="flex flex-1 flex-col gap-3">
        <Card className="flex flex-col gap-2 p-3">{fields}</Card>
        {project && (
          <button
            type="button"
            onClick={() => props.onOpenProject(project.id)}
            className="flex w-fit items-center gap-1.5 rounded-md px-2 py-1 text-label text-muted hover:bg-surface hover:text-ink"
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
              <span className="text-meta text-muted">
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
              className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-label tabular-nums text-muted"
              data-testid="desktop-resources"
            >
              <span>
                cpu {session.cpu_pct !== null ? `${Math.round(session.cpu_pct)}%` : "—"} · ram{" "}
                {session.rss_bytes !== null ? memorySize(session.rss_bytes) : "—"} · children{" "}
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
                <p className="text-meta leading-relaxed text-ink">
                  {session.isolation_label}
                </p>
              </div>
            )}
            {focusOutcome !== null && (
              <p className="text-meta text-ink-2" data-testid="desktop-focus-outcome">
                {focusOutcome.outcome}
                {focusOutcome.window_title !== null ? (
                  <span className="text-muted"> · window “{focusOutcome.window_title}”</span>
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
                <p className="text-meta leading-relaxed text-ink">
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
              <p className="text-label text-muted" data-testid="desktop-state-line">
                {session.status === "failed"
                  ? "app crashed · exit details on the Processes tab"
                  : "app exited · Relaunch, above, starts it again"}
              </p>
            )}
            <p className="text-label text-muted">
              The window is the app's own; Juggler tracks the process, never embeds it.
              Focus matches this session's recorded window — a stale title or a collision is
              reported, never guessed to a wrong window.
            </p>
          </Card>
        ) : null}
      </div>
    );
  }

  if (kind === "project" && data.project) {
    const project = data.project;
    const history = sessions.filter((s) => s.project_id === project.id);
    return (
      <div className="flex flex-col gap-3">
        <p className="font-mono text-label text-muted">{project.root}</p>
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          <Card className="flex flex-col gap-3 p-4">
            <p className="text-section font-semibold text-ink">On it now</p>
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
                <span className="ml-auto flex items-center gap-1 text-label text-muted">
                  open <IconTerminal width={12} height={12} />
                </span>
              </button>
            ) : (
              <p className="text-xs text-ink-2">No live session — free to juggle.</p>
            )}
            <p className="mt-2 text-section font-semibold text-ink">
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
                    title={ready ? undefined : "Needs sign-in first: sign in from its account on Accounts"}
                    onClick={() => props.onLaunch(recipe, project)}
                    className={
                      "flex items-center gap-2 rounded-lg border border-hairline px-3 py-2 text-left text-xs transition-colors " +
                      (ready ? "hover:border-accent/40 hover:bg-surface" : "border-hairline/50 opacity-50")
                    }
                  >
                    <IconTerminal width={13} height={13} />
                    <span className="font-medium">{recipe.account_label}</span>
                    <span className="text-muted">·</span>
                    <span className="text-muted">{launchSettingsName(recipe.name)}</span>
                    {recipe.model && <span className="ml-auto font-mono text-label text-muted">{recipe.model}</span>}
                    {!ready && <Chip tone="neutral">sign in first</Chip>}
                  </button>
                );
              })}
              {profiles.length === 0 && (
                <p className="text-xs text-ink-2">No launch settings yet — add an account first.</p>
              )}
            </div>
          </Card>
          <Card className="flex flex-col gap-2 p-4">
            <p className="text-section font-semibold text-ink">Sessions in this project</p>
            {history.length === 0 ? (
              <p className="text-xs text-ink-2">Nothing recorded yet.</p>
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
          <p className="text-section font-semibold text-ink">Account</p>
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-meta">
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
          <p className="text-section font-semibold text-ink">Sessions on this account</p>
          {rows.length === 0 ? (
            <p className="text-xs text-ink-2">No sessions recorded yet.</p>
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
          <p className="text-section font-semibold text-ink">Tool</p>
          <p className="font-mono text-meta">{data.provider.id}</p>
          <p className="text-meta text-ink-2">
            {providerAccounts.length} account{providerAccounts.length === 1 ? "" : "s"} ·{" "}
            {providerAccounts.filter((a) => signedIn(a.login_state)).length} signed in
          </p>
        </Card>
        <Card className="flex flex-col gap-2 p-4">
          <p className="text-section font-semibold text-ink">Accounts</p>
          {providerAccounts.map((account) => (
            <p key={account.id} className="flex items-center gap-2 py-1 text-xs">
              <span className="min-w-0 flex-1 truncate font-medium">{account.label}</span>
              <Chip tone={signedIn(account.login_state) ? "success" : "neutral"}>
                {signedIn(account.login_state) ? "signed in" : "signed out"}
              </Chip>
            </p>
          ))}
          {providerAccounts.length === 0 && (
            <p className="text-xs text-ink-2">No accounts for this tool yet.</p>
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
        className="shrink-0 rounded px-1 py-1.5 text-label text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        {copied ? "copied" : "copy"}
      </button>
    </span>
  );
}

// --- AJ-E408: Provider / Account / Project content -----------------------

export { installWords } from "../lib/providers";

/** Provider Overview (§5.3): counts, worst-first aggregate, honest
 * capability/version badges. */
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
        <div className="flex flex-wrap items-center gap-2 text-meta">
          <span className="text-ink">
            {providerAccounts.length} account{providerAccounts.length === 1 ? "" : "s"}
          </span>
          <span className="text-muted">· {running} running</span>
        </div>
        {/* Shown once known; nothing in its place until then (D-25). */}
        {install !== null && (
          <p className="text-label text-muted" data-testid="provider-install">
            {install === "failed" ? "Couldn't check what's installed" : installWords(install)}
          </p>
        )}
        {/* AJ-E1611: a newer version, and how to update it yourself. */}
        <ToolUpdateLine tool={provider.id} name={providerDisplayName(provider.id)} />
      </Card>
      <Card className="flex flex-col gap-1.5 p-4" data-testid="provider-aggregate">
        <p className="text-section font-semibold text-ink">Runway, least left first</p>
        {perAccount.length === 0 ? (
          <p className="text-meta text-ink-2">No accounts for this tool yet.</p>
        ) : (
          perAccount.map(({ account, display, sessions }) => (
            <div key={account.id} className="flex flex-col gap-1.5 py-1">
              <span className="text-meta">
                <span className="text-ink">{account.label}</span>
                {!signedIn(account.login_state) ? <span className="text-muted"> · signed out</span> : null}
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
    <span className="mt-auto pt-1 text-label text-muted" data-testid="tier-words" data-tier={tier}>
      {TIER_WORDS[tier]}
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
    case "quota_exit": case "limit_hit": // ended, or open at its limit (0.3)
      return "Hit its limit";
    default:
      return "Looks stuck";
  }
}

/** 0.3.4: each session's last signals (or why they couldn't be read),
 * kept so its Usage tab opens with them (the card below the charts grew
 * in a read later, pushing the page); read when the tab shows, again
 * when older than `QUALITY_FRESH_MS`. */
type QualityRead = { signals: import("../lib/bridge").QualitySignalsInfo | null; error: null } | { signals: null; error: string };
const qualityOf = sharedReads<QualityRead>(
  (sessionId) =>
    import("../lib/bridge")
      .then((bridge) => bridge.instanceQuality(sessionId))
      .then(
        (signals): QualityRead => ({ signals, error: null }),
        (cause: unknown): QualityRead => ({ signals: null, error: errorText(cause) }),
      ),
  32,
);
const QUALITY_FRESH_MS = 15_000;

function QualityTab(props: PanelProps) {
  const instance = props.data.instance;
  const read = useSharedRead(instance ? qualityOf(instance.id) : null, { wanted: props.visible, maxAgeMs: QUALITY_FRESH_MS });
  const quality = read?.signals ?? null;
  const loadError = read?.error ?? null;
  const [asTable, setAsTable] = useState(false);

  if (!instance) return <HonestEmpty tab="quality" />;

  const signals: { testId: string; label: string; body: React.ReactNode; tier: "Measured" | "Computed" | "Estimated"; reason?: string }[] = [];
  if (loadError) {
    return (
      <Card className="flex flex-col gap-1 p-4" data-testid="quality-tab">
        <p className="text-meta text-ink-2">Signals unavailable right now.</p>
        <p className="text-label text-muted">{loadError}</p>
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
            <table className="text-label">
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
            tokens in and out in this session so far
          </span>
        ),
    });
    signals.push({
      testId: "quality-cost",
      label: "Session cost",
      tier: "Estimated",
      body: quality.session_cost_usd !== null ? (
        <span className="tabular-nums text-ink">
          ~${quality.session_cost_usd.toFixed(2)}{quality.priced_model ? ` for ${modelName(quality.priced_model)}` : ""} · what the API would charge, not your bill
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
        label: "Probably stuck",
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
    <section className="flex flex-col gap-3" data-testid="quality-tab" aria-labelledby="quality-title">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h3 id="quality-title" className="text-section font-semibold text-ink">
          How it's going
        </h3>
        {signals.length > 0 && (
          <span className="flex items-center gap-3">
            <span className="text-meta text-muted">every 2 seconds</span>
            <button
              type="button"
              onClick={() => setAsTable((v) => !v)}
              aria-pressed={asTable}
              className="rounded-lg border border-hairline px-2 py-0.5 text-label text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              {asTable ? "View as chart" : "View as table"}
            </button>
          </span>
        )}
      </div>
      {signals.length === 0 ? (
        <Card className="flex flex-col gap-1 p-5">
          <p className="text-body text-ink-2">Nothing measured yet — this fills in as the session works.</p>
        </Card>
      ) : (
        // Cards sized to what they say, as many to a row as fit.
        <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,300px),1fr))] gap-grid">
          {signals.map((signal) => (
            <Card key={signal.testId} className="flex flex-col gap-1.5 p-5 text-body" data-testid={signal.testId}>
              <p className="text-meta font-medium text-ink-2">{signal.label}</p>
              {signal.body}
              <TierTag tier={signal.tier} />
            </Card>
          ))}
        </div>
      )}
    </section>
  );
}

/// AJ-E2404/E2407: a session's Processes tab is the Processes view for
/// that session alone (its tree, what it left behind, its history): one
/// model and one stop path with the Processes page, never a second list.
function ProcessesTab(props: PanelProps & { readOnly?: boolean }) {
  const instance = props.data.instance;
  if (!instance) return <HonestEmpty tab="processes" />;
  return (
    <div className="flex flex-col gap-3" data-testid="processes-tab">
      {instance.adopted && (
        // AJ-E510 Req.4: the pre-crash terminal output is gone; stopping
        // it is the verified sheet's (Stop… on its row below).
        <Card className="flex flex-col gap-1 p-3" data-testid="instance-detached-notice">
          <p className="text-meta font-medium text-ink">Running — picked up after restart</p>
          <p className="text-label text-muted">
            The terminal is detached: output from before the app closed is not available. Stop it with Stop… on its row below.
          </p>
        </Card>
      )}
      {/* Read once the tab is opened, not with the page (0.3.4). */}
      <WhenShown visible={props.visible}>
        <ProcessesView sessionId={instance.id} onOpenSession={props.onOpenSession} height="calc(100vh - 22rem)" testId="session-processes" />
      </WhenShown>
    </div>
  );
}

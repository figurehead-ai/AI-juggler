import { ToolChip } from "./components/ToolMark";
import { AppActions } from "./components/AppActions";
import { useAppKeys } from "./lib/app-keys";
import { ConversationPage } from "./components/ConversationPage";
import { ConversationsProvider } from "./lib/conversations";
import { onConversationsChanged } from "./lib/ipc/conversations";
import { withMarks, withPalette } from "./lib/palette";
import { useFirstRun } from "./lib/first-run";
import { useLiveDeltas } from "./lib/live-deltas";
import { MainNav } from "./components/MainNav";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button, Card } from "./components/primitives";
import { WorkflowRail, type Freshness } from "./components/WorkflowRail";
import { GoToPalette } from "./components/GoToPalette";
import { PortWarningSheet } from "./components/PortMap";
import { RecoverySummaryDialog } from "./components/RecoverySummaryDialog";
import { UsageStatsReconsent } from "./components/UsageStatsReconsent";
import { CrashReportPrompt } from "./components/CrashReports";
import { CleanupCentre, focusCleanupFilter } from "./screens/CleanupCentre";
import { toolFound } from "./lib/providers";
import { launchPortWarning, type PortWarningInfo,} from "./lib/bridge";
import { GLOBAL_NAV, MORE_NAV, PRIMARY_NAV, globalId, type GlobalId } from "./lib/nav";
import { ThisMacScreen } from "./screens/ThisMacScreen";
import { PrivacyScreen } from "./screens/PrivacyScreen";
import { IconJuggler, IconTerminal } from "./components/icons";
import { AttentionAnnouncer, RenewalAnnouncer } from "./components/indicator";
import { LegendPopover } from "./components/LegendPopover";
import {
  DENSITY_EVENT,
  nextDensity,
  readDensity,
  storeDensity,
  type Density,
} from "./lib/density";
import {
  fleetRemote,
  fleetRemoteFocus,
  type RemoteCommandOutcomeInfo,
  type RemoteMachineInfo,
} from "./lib/bridge";
import { providerDisplayName } from "./lib/providers";
import { primaryToolId } from "./lib/tools";
import { NARROW_RAIL_PX, useNarrowWindow } from "./lib/window";
import { readInspectorFolded, storeInspectorFolded } from "./lib/shortcuts";
import { ShortcutSheet } from "./components/ShortcutSheet";
import { holdStop, sessionLabel } from "./lib/stop";
import { ToastHost } from "./components/Toast";
import { QuitSheet } from "./components/QuitSheet";
import { cleanupAlerts, fleetPeers, statsEvent, statsGauge } from "./lib/bridge";

/** AJ-E1103: the schema's screen names (History isn't counted; Home and
 * the retired Overview aren't either — the schema keeps the name). */
const STAT_SCREENS: Partial<Record<string, string>> = {
  accounts: "accounts",
  usage: "usage",
  fleet: "fleet",
  cleanup: "cleanup",
  settings: "settings",
};


/** AJ-E1103: the board became interactive once, after launch. */
let boardInteractiveCounted = false;
import { ProfilesScreen } from "./screens/Profiles";
import { History } from "./screens/History";
import { AddProviderSheet } from "./screens/AddProviderSheet";
import { FirstRun } from "./screens/FirstRun";
import { HomeScreen, placeOf } from "./screens/Home";
import { UsageScreen } from "./screens/Usage";
import { LimitDialog } from "./components/LimitDialog";
import { NewSessionModal } from "./screens/NewSession";
import { NewProjectSheet } from "./screens/NewProjectSheet";
import { FleetScreen } from "./screens/FleetScreen";
import { SettingsScreen } from "./screens/Settings";
import { FleetPromptDialog } from "./components/FleetPromptDialog";
import { IdentityMismatchDialog } from "./components/IdentityMismatchDialog";
import { IsolationOfferDialog } from "./components/IsolationOfferDialog";
import { funnelEvent } from "./lib/funnel";
import { SessionPanel } from "./screens/WorkPanels";
import { NotificationRegion } from "./components/NotificationRegion";
import { ScreenBoundary, ScrollingScreen } from "./components/ScreenBoundary";
import { SwitchFlow, showSwitchUndoToast } from "./components/SwitchFlow";
import { SwitchBackFlow } from "./components/SwitchBackFlow";
import { RightInspector } from "./components/RightInspector";
import { ThisMacContext, useThisMac } from "./lib/machine";
import { MachineStatusLine } from "./components/MachineStatusLine";
import {
  accountsList,
  providersDiscover,
  type SwitchBackSuggestionData,
  launchRecipesList,
  onSessionsUpdated,
  onTerminalExit,
  projectsBackfill,
  projectsList,
  instancesList,
  sessionHandover,
  switchBackSuggestions,
  onFolderDropped,
  launchSession,
  appSettingGet,
  desktopFocus,
  goToHotkeySet,
  onGoToHotkey,
  type LaunchRecipeInfo,
  fetchAppInfo,
  onAppReady,
  type AccountInfo,
  type AppInfo,
  type ProjectInfo,
  type InstanceInfo,
} from "./lib/bridge";

/**
 * App shell (ADR-0007, design iteration 5): a workflow navigator rail
 * (Projects / Instances / Sessions — the list lives in the rail) plus the
 * detail panel where the selected item loads, session terminals embedded.
 * Global oversight (Overview, Accounts, Usage, Fleet) sits top-right.
 */

type Selection =
  | { kind: "none" }
  | { kind: "project"; id: string }
  | { kind: "session"; id: string }
  | { kind: "login"; id: string }
  | { kind: "account"; id: string }
  | { kind: "provider"; id: string }
  | { kind: "conversation"; accountId: string; id: string; projectId: string | null } // AJ-E1529
  // AJ-E705: a paired machine (instanceId null) or an Instance on it.
  | { kind: "remote"; machineId: string; instanceId: string | null };

export default function App() {
  // Density (AJ-E403): persisted in `aij.density`; the legacy
  // `aij.rail.collapsed` boolean migrates into `minimal` on first read.
  const [density, setDensity] = useState<Density>(() => readDensity());
  // D-25: density lives in Settings › Appearance now; follow its changes.
  useEffect(() => {
    const follow = () => setDensity(readDensity());
    window.addEventListener(DENSITY_EVENT, follow);
    return () => window.removeEventListener(DENSITY_EVENT, follow);
  }, []);
  // AJ-E1103: the display preferences, as this week's gauges.
  useEffect(() => {
    statsGauge("ui.density", density);
  }, [density]);
  useEffect(() => {
    const report = () => void import("./theme").then((theme) => statsGauge("ui.theme", theme.storedTheme()));
    report();
    window.addEventListener("aij:theme-changed", report);
    return () => window.removeEventListener("aij:theme-changed", report);
  }, []);
  const cycleStoredDensity = useCallback(() => {
    setDensity((current) => {
      const next = nextDensity(current);
      storeDensity(next);
      return next;
    });
    window.setTimeout(() => window.dispatchEvent(new Event(DENSITY_EVENT)), 0);
  }, []);
  const setDensityAndStore = useCallback((next: Density) => {
    setDensity(next);
    storeDensity(next);
    window.dispatchEvent(new Event(DENSITY_EVENT));
  }, []);
  // D-25: below 900 px the rail folds to its icon column by itself and
  // comes back when the window widens. In a narrow window ⌘B, the logo,
  // the density shortcut and a double-click on the rail only open or
  // close the list for now — they never write Settings › Appearance,
  // whatever is stored. Changing the density is for Settings and for a
  // wide window.
  const narrow = useNarrowWindow(NARROW_RAIL_PX);
  const [narrowOpen, setNarrowOpen] = useState(false);
  useEffect(() => {
    if (!narrow) setNarrowOpen(false);
  }, [narrow]);
  // Open in a narrow window: the stored density, or Comfortable when the
  // stored one is Minimal (the icons are what "closed" already shows).
  const railDensity: Density = narrow
    ? narrowOpen
      ? density === "minimal"
        ? "comfortable"
        : density
      : "minimal"
    : density;
  const toggleRail = useCallback(() => {
    if (narrow) setNarrowOpen((open) => !open);
    else setDensityAndStore(density === "minimal" ? "comfortable" : "minimal");
  }, [narrow, density, setDensityAndStore]);
  const cycleDensity = useCallback(() => {
    if (narrow) setNarrowOpen((open) => !open);
    else cycleStoredDensity();
  }, [narrow, cycleStoredDensity]);

  // The stale-data tick (AJ-E403 Req.5): ONE shared 15 s timer for the
  // whole app — staleness captions recompute against `now` without any
  // per-value timers.
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 15_000);
    return () => window.clearInterval(id);
  }, []);

  // Freshness per Instance id: when each value class was last sampled.
  // Seeded at list load and refreshed by the resource/quota ticks; the
  // `__AIJ_SAMPLED_AT__` hook exists for the visual fixtures (stale-row
  // captures) exactly like `__AIJ_FIXTURE__`.
  const [freshness, setFreshness] = useState<Freshness>({});
  const baselineFreshness = useCallback((ids: string[]): Freshness => {
    const forced = (window as unknown as { __AIJ_SAMPLED_AT__?: Record<string, number> })
      .__AIJ_SAMPLED_AT__;
    const at = Date.now();
    const map: Freshness = {};
    for (const id of ids) {
      map[id] = {
        resources: forced?.[id] ?? at,
        quota: forced?.[id] ?? at,
      };
    }
    return map;
  }, []);
  const [selection, setSelection] = useState<Selection>({ kind: "none" });
  const [global, setGlobal] = useState<GlobalId | null>(null);
  // D-25: Cleanup and Fleet surface only when they're relevant.
  const [cleanupCount, setCleanupCount] = useState(0);
  const [pairedCount, setPairedCount] = useState(0);
  useEffect(() => {
    let alive = true;
    const load = () => {
      void cleanupAlerts().then((data) => alive && setCleanupCount(data.alerts.length), () => null);
      void fleetPeers().then((peers) => alive && setPairedCount(peers.length), () => null);
    };
    load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  // AJ-E1103: which screens get opened (and the Cleanup Centre).
  useEffect(() => {
    const screen = global ? STAT_SCREENS[global] : undefined;
    if (screen) statsEvent("ui.screen_opened", { screen });
    if (global === "cleanup") statsEvent("cleanup_centre.opened");
  }, [global]);
  // AJ-E1203: a Cloud chip / the onboarding card land on Account & Cloud.
  const [settingsFocusCloud, setSettingsFocusCloud] = useState(false);
  // D-25: a link elsewhere (Accounts' pack-secrets line) opens one section.
  const [settingsSection, setSettingsSection] = useState<string | undefined>(undefined);
  // AJ-E1524: Home (the welcome while no account is in).
  const openHome = () => {
    setGlobal(null);
    setSelection({ kind: "none" });
  };
  const openCloudSettings = () => {
    setSettingsFocusCloud(true);
    setGlobal("settings");
  };
  // AJ-E1010: the update notice's Review lands on Settings.
  useEffect(() => {
    const open = (event: Event) => {
      setSettingsFocusCloud(false);
      setSettingsSection((event as CustomEvent<{ section?: string } | undefined>).detail?.section);
      setGlobal("settings");
    };
    window.addEventListener("aij:open-settings", open);
    return () => window.removeEventListener("aij:open-settings", open);
  }, []);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [sessions, setSessions] = useState<InstanceInfo[]>([]);
  // AJ-E1007: archived Instances leave the rail (History keeps them).
  // AJ-E1502: a forgotten project, and its (ended) sessions, leave the
  // rail and New session until it is used again; History keeps them.
  const shownProjects = useMemo(() => projects.filter((project) => !project.forgotten), [projects]);
  const railSessions = useMemo(() => {
    const forgotten = new Set(projects.filter((project) => project.forgotten).map((project) => project.id));
    return sessions.filter((row) => row.lifecycle !== "archived" && !(row.project_id !== null && forgotten.has(row.project_id)));
  }, [sessions, projects]);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[]>([]);
  // AJ-E1503 Req.4: until the first load answers, lists show row-shaped
  // placeholders, never a false "nothing here".
  const [loaded, setLoaded] = useState(false);
  const [newSessionDir, setNewSessionDir] = useState<string | null>(null);
  const [loginTerminals, setLoginTerminals] = useState<
    { id: string; title: string; ended: boolean }[]
  >([]);
  const [newSession, setNewSession] = useState(false);
  const closeNewSession = () => {
    setNewSession(false);
    setNewSessionDir(null);
  };
  // AJ-E205: the executing switch flow (freeze dialog → stages →
  // source-safe failure banner). Null = closed.
  // D-25: Accounts › Add account reuses first run's steps, for this tool
  // (AJ-E1023: chosen first only when more than one is installed).
  const [addAccounts, setAddAccounts] = useState<string | null>(null);
  // AJ-E1524: the welcome's "Add a fresh account" opens on that step.
  const [addFresh, setAddFresh] = useState(false);
  const [chooseTool, setChooseTool] = useState(false);
  const startAddAccount = useCallback((fresh = false) => {
    setAddFresh(fresh);
    void providersDiscover().then(
      (rows) => {
        // Found counts, even when its version couldn't be read (0.0.11).
        const ready = rows.filter(toolFound);
        if (ready.length > 1) setChooseTool(true);
        else setAddAccounts(ready[0]?.provider ?? primaryToolId() ?? "");
      },
      () => setAddAccounts(primaryToolId() ?? ""),
    );
  }, []);
  const [accountsNonce, setAccountsNonce] = useState(0);
  // D-25: the account-limit decision, for this session id.
  const [limitFor, setLimitFor] = useState<string | null>(null);
  // The tool chip leaves the header when its view opens: focus follows.
  const [inspectorFocus, setInspectorFocus] = useState(0);
  const [switchFlow, setSwitchFlow] = useState<{ sessionId: string; targetProfileId: string; targetLabel: string } | null>(null);
  // AJ-E207: the suggested returns (reset-trigger cards) + the
  // switch-back review modal. Null = closed; suggest-only.
  const [switchBackCards, setSwitchBackCards] = useState<SwitchBackSuggestionData[]>([]);
  const [switchBack, setSwitchBack] = useState<string | null>(null);
  // AJ-E505: the pre-launch port collision (advisor informs, never
  // blocks — Launch always proceeds on "Launch anyway").
  const [portWarning, setPortWarning] = useState<{
    warning: PortWarningInfo;
    proceed: () => void;
  } | null>(null);
  // AJ-E414: the "Go to instance…" palette — summoned by the global
  // hotkey (backend event) or in-app ⌘K; the binding is rebindable.
  const [goToOpen, setGoToOpen] = useState(false);
  const [goToHotkey, setGoToHotkey] = useState("Alt+Space");
  // D-25: a fresh install opens on first run (what's here → sign in →
  // computer access); a fresh account and a folder import are its steps.
  const [firstRun, setFirstRun, firstRunAtAccess, setFirstRunAtAccess] = useFirstRun();
  // AJ-E305 flow (b): the New project sheet. Null = closed; a string
  // pre-selects that directory (the Summary's Start here).
  const [newProjectDir, setNewProjectDir] = useState<string | null>(null);
  // AJ-E510: the ONE-time crash-recovery summary — shown until
  // dismissed; a second relaunch (no new findings) shows nothing.
  const [recovery, setRecovery] = useState<
    import("./lib/bridge").RecoverySummaryData | null
  >(null);
  useEffect(() => {
    void import("./lib/bridge")
      .then((bridge) => bridge.recoverySummaryGet())
      .then((summary) => setRecovery(summary))
      .catch(() => setRecovery(null));
  }, []);

  useEffect(() => {
    void fetchAppInfo().then(setInfo);
    return onAppReady(setInfo);
  }, []);

  // Inspector action events (AJ-E408): Add-account / sign-in cards and
  // the Start-first-session CTA route to the real surfaces.
  useEffect(() => {
    const openAccounts = () => setGlobal("accounts");
    const newSession = () => setNewSession(true);
    const openScreen = (event: Event) => setGlobal((current) => globalId((event as CustomEvent<{ id?: unknown }>).detail?.id) ?? current);
    window.addEventListener("aij:open-accounts", openAccounts);
    window.addEventListener("aij:new-session", newSession);
    window.addEventListener("aij:open-screen", openScreen);
    return () => {
      window.removeEventListener("aij:open-accounts", openAccounts);
      window.removeEventListener("aij:new-session", newSession);
      window.removeEventListener("aij:open-screen", openScreen);
    };
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [projectRows, sessionRows, accountRows, profileRows, backSuggestions] =
        await Promise.all([
          projectsList(),
          instancesList(),
          accountsList(),
          launchRecipesList(),
          switchBackSuggestions().catch(() => []),
        ]);
      setProjects(withMarks(projectRows)); // AJ-E1532: each project its mark
      setSessions(sessionRows);
      setAccounts(withPalette(accountRows)); // AJ-E1532: each account its own colour
      setProfiles(profileRows);
      setSwitchBackCards(backSuggestions);
      if (!boardInteractiveCounted) {
        boardInteractiveCounted = true;
        statsEvent("perf.board_interactive_ms", {}, performance.now());
      }
      setFreshness((current) => {
        // Keep known sample times; new rows get the load baseline.
        const baseline = baselineFreshness(sessionRows.map((row) => row.id));
        const next: Freshness = { ...baseline };
        for (const [id, sampled] of Object.entries(current)) {
          if (next[id]) next[id] = sampled;
        }
        return next;
      });
    } catch {
      // preview mode may lack backend mocks for some calls — keep last data
    } finally {
      setLoaded(true);
    }
  }, [baselineFreshness]);

  // Golden path day one: surface the user's REAL project history from
  // imported transcripts (idempotent backend scan). AJ-E1529: the backend
  // backfills again when the files change (an import, a new folder) and
  // says so; the lists are read again then.
  useEffect(() => {
    void projectsBackfill().then(() => void refresh());
    return onConversationsChanged(() => void refresh());
  }, [refresh]);

  // Drop a folder anywhere → New session pre-filled with it.
  useEffect(() => {
    return onFolderDropped((path) => {
      setNewSessionDir(path);
      setNewSession(true);
    });
  }, []);

  useEffect(() => {
    void refresh();
    return onSessionsUpdated(() => void refresh());
  }, [refresh]);

  // AJ-E402/E401: the 5 s resource and 30 s quota ticks, merged by id.
  useLiveDeltas(setSessions, setFreshness);

  // Login terminals end → mark ended (kept for scrollback until closed).
  useEffect(() => {
    return onTerminalExit((id) => {
      setLoginTerminals((current) =>
        current.map((t) => (t.id === id ? { ...t, ended: true } : t)),
      );
      void refresh();
    });
  }, [refresh]);

  const selectSession = useCallback((id: string) => {
    setGlobal(null);
    setSelection({ kind: "session", id });
  }, []);

  const selectProject = useCallback((id: string) => {
    setGlobal(null);
    setSelection({ kind: "project", id });
  }, []);

  const selectLogin = useCallback((id: string) => {
    setGlobal(null);
    setSelection({ kind: "login", id });
  }, []);

  // Login flows (Accounts screen) hand their terminal here; it joins the
  // Sessions rail list and gets selected.
  const openLoginTerminal = useCallback((id: string, title: string) => {
    setLoginTerminals((current) =>
      current.some((t) => t.id === id) ? current : [...current, { id, title, ended: false }],
    );
    setGlobal(null); // the panel must show the terminal, not stay on Accounts
    setSelection({ kind: "login", id });
  }, []);

  // AJ-E705: what paired machines report, polled while one is selected.
  const [remoteMachines, setRemoteMachines] = useState<RemoteMachineInfo[] | null>(null);
  const [remoteNow, setRemoteNow] = useState(() => Date.now());
  const remoteSelected = selection.kind === "remote";
  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const list = await fleetRemote();
        if (alive) {
          setRemoteMachines(list);
          setRemoteNow(Date.now());
        }
      } catch {
        // A slow or offline peer never blocks the board.
      }
    };
    void load();
    // A local read of what peers reported: fast while one is open.
    const timer = window.setInterval(() => void load(), remoteSelected ? 2000 : 5000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [remoteSelected]);
  // AJ-E711: the last palette-started remote focus, shown in Actions.
  const [remoteFocusOutcome, setRemoteFocusOutcome] = useState<{
    machineId: string;
    instanceId: string;
    outcome: RemoteCommandOutcomeInfo;
  } | null>(null);
  const inspectRemote = (machineId: string, instanceId: string | null) => {
    setGlobal(null);
    setSelection({ kind: "remote", machineId, instanceId });
  };

  const selectedSession = useMemo(
    () =>
      selection.kind === "session"
        ? (sessions.find((s) => s.id === selection.id) ?? null)
        : null,
    [selection, sessions],
  );
  const selectedProject = useMemo(
    () =>
      selection.kind === "project"
        ? (projects.find((p) => p.id === selection.id) ?? null)
        : null,
    [selection, projects],
  );
  const selectedLogin = useMemo(
    () =>
      selection.kind === "login"
        ? (loginTerminals.find((t) => t.id === selection.id) ?? null)
        : null,
    [selection, loginTerminals],
  );

  const selectedAccount = useMemo(
    () => (selection.kind === "account" ? (accounts.find((a) => a.id === selection.id) ?? null) : null),
    [selection, accounts],
  );
  const selectedProvider = useMemo(() => {
    if (selection.kind !== "provider") return null;
    const providerAccounts = accounts.filter((a) => a.provider === selection.id);
    return { id: selection.id, name: providerDisplayName(selection.id), accounts: providerAccounts };
  }, [selection, accounts]);


  // AJ-E1504: the `?` sheet, and the inspector folded to one line
  // (⌘⇧I), remembered for this window across launches.
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [inspectorFolded, setInspectorFolded] = useState(() => readInspectorFolded());
  const toggleInspector = useCallback(() => {
    setInspectorFolded((folded) => {
      storeInspectorFolded(!folded);
      return !folded;
    });
  }, []);

  // AJ-E1503 Req.2: Stop is held 5 s with Undo, then sent. The ref keeps
  // ⌘. on the session selected now (the key listener is set up once).
  const stopSession = useCallback(
    (session: InstanceInfo) => holdStop({ id: session.id, label: sessionLabel(session, projects) }, () => void refresh()),
    [projects, refresh],
  );
  const stopSelectedRef = useRef<() => void>(() => {});
  stopSelectedRef.current = () => {
    if (selectedSession && (selectedSession.status === "running" || selectedSession.status === "starting")) {
      stopSession(selectedSession);
    }
  };

  // AJ-E1504: App's shortcuts (lib/app-keys.ts, from the one registry).
  useAppKeys({
    openShortcuts: () => setShortcutsOpen(true),
    togglePalette: () => setGoToOpen((openNow) => !openNow),
    newProject: () => setNewProjectDir((current) => current ?? ""),
    toggleRail,
    toggleInspector,
    cycleDensity,
    goHome: () => { setGlobal(null); setSelection({ kind: "none" }); },
    goTo: setGlobal,
    openSettings: () => { setSettingsFocusCloud(false); setSettingsSection(undefined); setGlobal("settings"); },
    onCleanup: () => global === "cleanup",
    focusCleanupFilter,
    stopSelected: () => stopSelectedRef.current(),
  });

  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => {
      if (stored !== null) setGoToHotkey(stored);
    });
    return onGoToHotkey(() => setGoToOpen(true));
  }, []);

  // The Provider of what's selected — never a fixed label.
  const remoteInstanceProvider =
    selection.kind === "remote" && selection.instanceId
      ? (remoteMachines
          ?.find((machine) => machine.machine_id === selection.machineId)
          ?.instances.find((row) => row.id === selection.instanceId)?.provider ?? null)
      : null;
  // E14: This Mac — one subscription for Home, the rail and the header.
  const thisMacInfo = useThisMac(sessions);
  const contextProviderId =
    global !== null
      ? null
      : (selectedSession?.provider ??
        selectedAccount?.provider ??
        remoteInstanceProvider ??
        (selection.kind === "provider" ? selection.id : null));
  const contextProvider = contextProviderId ? providerDisplayName(contextProviderId) : null;

  const contextTitle =
    global !== null
      ? (GLOBAL_NAV.find((g) => g.id === global)?.label ?? "")
      : selection.kind === "remote"
        ? `Fleet · ${remoteMachines?.find((m) => m.machine_id === selection.machineId)?.label ?? "machine"}`
        : selection.kind === "project"
        ? (selectedProject?.name ?? "Project")
        : selection.kind === "provider"
        ? (contextProvider ?? "Tool")
        : selection.kind === "account"
        ? (selectedAccount?.label ?? "Account")
        : selectedSession
          ? // D-25: the session's project (the view below names its place).
            (projects.find((p) => p.id === selectedSession.project_id)?.name ?? "Session")
          : selectedLogin
            ? selectedLogin.title
            : selection.kind === "none"
              ? "Home"
              : // AJ-E1534: a session outside AI Juggler, like one of its own, by its project.
                (projects.find((p) => selection.kind === "conversation" && p.id === selection.projectId)?.name ?? "Session");

  return (
    <ThisMacContext.Provider value={thisMacInfo}>
    <ConversationsProvider>
    <AppActions
      selectConversation={(conversation) => { setGlobal(null); setSelection({ kind: "conversation", accountId: conversation.account_id, id: conversation.id, projectId: conversation.project_id }); }}
      selectSession={selectSession}
      selectProject={selectProject}
      selectAccount={(id) => { setGlobal(null); setSelection({ kind: "account", id }); }}
      selectTool={(id) => { setGlobal(null); setSelection({ kind: "provider", id }); }}
      openAccounts={() => setGlobal("accounts")}
      stopSession={stopSession}
      newSessionIn={(project) => { setNewSessionDir(project.root); setNewSession(true); }}
      refresh={() => void refresh()}
    >
    <div className="relative flex h-screen w-screen overflow-hidden">
      <AttentionAnnouncer sessions={sessions} />
      <RenewalAnnouncer accounts={accounts} />
      <WorkflowRail
        density={railDensity}
        onCycleDensity={cycleDensity}
        projects={shownProjects}
        sessions={railSessions}
        onChanged={() => void refresh()}
        accounts={accounts}
        loginTerminals={loginTerminals}
        selectedId={selection.kind === "none" || selection.kind === "remote" ? null : selection.id}
        onSelectSession={(id) =>
          loginTerminals.some((t) => t.id === id) ? selectLogin(id) : selectSession(id)
        }
        onSelectProject={selectProject}
        onNewSession={() => setNewSession(true)}
        onNewProject={() => setNewProjectDir((current) => current ?? "")}
        freshness={freshness}
        now={now}
        remoteMachines={remoteMachines ?? []}
        selectedRemote={
          selection.kind === "remote" ? { machineId: selection.machineId, instanceId: selection.instanceId } : null
        }
        onSelectRemote={inspectRemote}
        loading={!loaded}
        header={(minimal) => (
          <div
            className={
              "flex h-14 shrink-0 items-center gap-2.5 overflow-hidden " +
              (minimal ? "justify-center px-0" : "px-4")
            }
          >
            <button
              type="button"
              title={minimal ? "Show the list (⌘B)" : "Icons only (⌘B)"}
              onClick={toggleRail}
              className="shrink-0 rounded-md p-1 text-accent hover:bg-surface"
            >
              <span className="block drop-shadow-[0_0_10px_#7c7cf466]">
                <IconJuggler width={22} height={22} />
              </span>
            </button>
            {!minimal && (
              <span className="whitespace-nowrap text-[15px] font-semibold tracking-tight">
                AI Juggler
              </span>
            )}
            {/* D-23: which edition this window belongs to, when it isn't
                AI Juggler itself (Nightly and Dev run side by side). */}
            {info && info.edition !== "main" && (
              <span
                className="shrink-0 rounded-full border border-accent/40 bg-accent-soft px-1.5 py-0.5 text-[10px] font-semibold text-ink"
                data-testid="edition-badge"
                title={`${info.name}: its own data, separate from AI Juggler`}
              >
                {info.edition === "nightly" ? "Nightly" : "Dev"}
              </span>
            )}
          </div>
        )}
        footer={(minimal) =>
          minimal ? (
            <MachineStatusLine minimal onOpen={() => setGlobal("thismac")} />
          ) : (
            <div className="flex flex-col gap-1.5 px-3 py-2.5">
              <div className="flex min-w-0 items-center gap-2 overflow-hidden">
                <MachineStatusLine minimal={false} onOpen={() => setGlobal("thismac")} />
                {/* The rail-footer `?` legend popover (AJ-E403 Req.6,
                    surface 1 of 3 — identical content everywhere). */}
                <LegendPopover />
              </div>
              <div className="truncate font-mono text-[10px] text-faint">
                v{info?.version ?? "0.0.0"}
              </div>
            </div>
          )
        }
      />

      {/* Main column */}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Glass status bar: work context left, global oversight right */}
        <header
          data-tauri-drag-region
          // AJ-E804 (WCAG 1.4.10): at narrow widths and 200 % zoom the
          // menus wrap onto a second row instead of being clipped. The
          // title has a zero basis, so it truncates first and a wide
          // window keeps the single row.
          className="relative z-30 flex min-h-14 shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline bg-surface/50 px-5 py-2 backdrop-blur-xl"
        >
          <div className="flex min-w-0 flex-1 basis-0 items-center gap-3">
            <h1 className="min-w-0 truncate text-sm font-semibold tracking-wide">{contextTitle}</h1>
            {/* The tool's own view (what's installed, its accounts and
                their quota): one click from a session that runs on it. */}
            {contextProvider && contextProviderId && selection.kind !== "provider" && (
              <button
                type="button"
                onClick={() => {
                  setGlobal(null);
                  setSelection({ kind: "provider", id: contextProviderId });
                  setInspectorFocus((n) => n + 1);
                }}
                aria-label={`About ${contextProvider}`}
                title={`About ${contextProvider}`}
                className="rounded-full outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="context-tool"
              >
                <ToolChip tool={contextProviderId} tone="accent" />
              </button>
            )}
          </div>

          <div className="ml-auto flex min-w-0 max-w-full flex-wrap items-center justify-end gap-0.5 xl:gap-1.5" data-tauri-drag-region={false}>
            <MainNav
              primary={PRIMARY_NAV}
              more={MORE_NAV}
              global={global}
              page={selection.kind}
              pairedCount={pairedCount}
              cleanupCount={cleanupCount}
              toolIds={[...new Set(accounts.map((account) => account.provider))]}
              onPick={(id) => {
                if (PRIMARY_NAV.some((item) => item.id === id)) {
                  setSettingsFocusCloud(false);
                  setSettingsSection(undefined);
                }
                if (id === "home") {
                  setGlobal(null);
                  setSelection({ kind: "none" });
                } else setGlobal(id);
              }}
              onOpenTool={(id) => { setGlobal(null); setSelection({ kind: "provider", id }); }}
            />
            <Button size="sm" variant="primary" onClick={() => setNewSession(true)}>
              <IconTerminal width={14} height={14} />
              New session
            </Button>
          </div>
        </header>

        {/* Content: detail panel (work mode) or global screen. tabIndex
         * -1: programmatic focus target when overlays (AJ-E510
         * recovery summary) dismiss and return focus to the board. */}
        <main tabIndex={-1} className="min-h-0 flex-1 overflow-hidden p-5 outline-none">
          {global === "cleanup" ? (
            <div className="h-full">
              <ScreenBoundary key="cleanup" screen="cleanup" label="Cleanup">
                <CleanupCentre />
              </ScreenBoundary>
            </div>
          ) : global === "accounts" ? (
            <ScrollingScreen key="accounts" screen="accounts" label="Accounts">
              <ProfilesScreen
                key={accountsNonce}
                sessions={sessions}
                onAddAccounts={() => startAddAccount()}
                onAccountsChanged={() => void refresh()}
                openTerminal={openLoginTerminal}
                onInspectAccount={(id) => {
                  setGlobal(null);
                  setSelection({ kind: "account", id });
                }}
              />
            </ScrollingScreen>
          ) : global === "history" ? (
            <ScrollingScreen key="history" screen="history" label="History">
              <History sessions={sessions} projects={projects} onChanged={() => void refresh()} />
            </ScrollingScreen>
          ) : global === "usage" ? (
            <ScrollingScreen key="usage" screen="usage" label="Usage">
              <UsageScreen accounts={accounts} sessions={sessions} onAddAccounts={() => setFirstRun(true)} onOpenHome={openHome} />
            </ScrollingScreen>
          ) : global === "fleet" ? (
            <ScrollingScreen key="fleet" screen="fleet" label="Fleet">
              <FleetScreen
                onOpenCloud={openCloudSettings}
                onInspectRemote={inspectRemote}
                onOpenHome={accounts.length === 0 && loaded ? openHome : undefined}
              />
            </ScrollingScreen>
          ) : global === "thismac" ? (
            <ScrollingScreen key="thismac" screen="this-mac-page" label="This Mac">
              <ThisMacScreen
                nameOf={(id) => {
                  const session = sessions.find((s) => s.id === id);
                  return session ? placeOf(session, projects).title : "A session";
                }}
                onOpenSession={selectSession}
                onOpenCleanup={() => setGlobal("cleanup")}
              />
            </ScrollingScreen>
          ) : global === "privacy" ? (
            <ScrollingScreen key="privacy" screen="privacy" label="Privacy">
              <PrivacyScreen />
            </ScrollingScreen>
          ) : global === "settings" ? (
            <ScrollingScreen key="settings" screen="settings" label="Settings">
              <SettingsScreen key={settingsSection ?? "top"} focusCloud={settingsFocusCloud} focusSection={settingsSection} />
            </ScrollingScreen>
          ) : selectedLogin ? (
            <SessionPanel
              session={null}
              project={null}
              loginTerminal={selectedLogin}
              profiles={profiles}
              accounts={accounts}
              onOpenProject={selectProject}
              onStopped={() => void refresh()}
              onHandover={async () => {}}
            />
          ) : selection.kind === "remote" ? (
            (() => {
              const machine =
                remoteMachines?.find((entry) => entry.machine_id === selection.machineId) ?? null;
              if (!machine) {
                return (
                  <Card className="flex flex-col items-start gap-2 p-5 text-xs" data-testid="remote-missing">
                    <p>{remoteMachines === null ? "Loading…" : "That machine is no longer paired with this Mac."}</p>
                    <Button size="sm" variant="secondary" onClick={() => setGlobal("fleet")}>
                      Back to Fleet
                    </Button>
                  </Card>
                );
              }
              const instance = selection.instanceId
                ? (machine.instances.find((row) => row.id === selection.instanceId) ?? null)
                : null;
              return (
                <ScreenBoundary key="inspector" screen="inspector" label="This view">
                  <RightInspector
                    folded={inspectorFolded}
                    onToggleFold={toggleInspector}
                    selection={
                      instance
                        ? { kind: "instance", id: instance.id }
                        : { kind: "machine", id: machine.machine_id }
                    }
                    data={{
                      remote: {
                        machine,
                        instance,
                        lastOutcome:
                          remoteFocusOutcome &&
                          remoteFocusOutcome.machineId === machine.machine_id &&
                          remoteFocusOutcome.instanceId === instance?.id
                            ? remoteFocusOutcome.outcome
                            : null,
                      },
                    }}
                    sessions={sessions}
                    projects={projects}
                    accounts={accounts}
                    profiles={profiles}
                    terminalTitle={null}
                    freshness={freshness}
                    now={remoteNow}
                    onOpenProject={selectProject}
                    onOpenSession={selectSession}
                    onLaunch={() => {}}
                    onStopped={() => void refresh()}
                    onHandover={() => {}}
                    onNew={() => setNewSession(true)}
                    onRemoteBack={() => setGlobal("fleet")}
                    onRemoteSelect={(instanceId) => inspectRemote(machine.machine_id, instanceId)}
                    onRemoteActionDone={() => setRemoteNow(Date.now())}
                  />
                </ScreenBoundary>
              );
            })()
          ) : (
            // The region takes its own space above the screen and pushes
            // it down — never over it, at any width (the 0.0.12 report).
            <div className="flex h-full min-h-0 flex-col">
              {/* AJ-E611: the alert/notification region — polite
                  live-region, badge-carrying copy, reset-queue rows
                  with an explicit Resume click (the click opens the
                  picker; nothing ever launches by itself). */}
              <div className="shrink-0" data-testid="notification-band">
                <NotificationRegion
                  onQueueResume={() => setNewSession(true)}
                  onOpenRemote={(machineId) => inspectRemote(machineId, null)}
                  hideMachineAlerts={selection.kind === "none"}
                />
              </div>
              {selection.kind === "none" ? (
                <div className="relative min-h-0 flex-1 overflow-y-auto" data-testid="home-scroll">
                  <ScreenBoundary key="home" screen="home" label="Home">
                    <HomeScreen
                      loading={!loaded}
                      sessions={sessions}
                      accounts={accounts}
                      projects={projects}
                      now={now}
                      onOpenSession={selectSession}
                      onNewSession={() => setNewSession(true)}
                      // No account yet: "Find my accounts" comes back to first
                      // run (every tool at once), as its Not now promises.
                      onAddAccounts={() => setFirstRun(true)}
                      onFreshAccount={() => startAddAccount(true)}
                      onLimit={setLimitFor}
                      openLoginTerminal={openLoginTerminal}
                      onAccountsChanged={() => void refresh()}
                      onChanged={() => void refresh()}
                      thisMac={thisMacInfo}
                      onOpenCleanup={() => setGlobal("cleanup")}
                      onOpenThisMac={() => setGlobal("thismac")}
                    />
                  </ScreenBoundary>
                </div>
              ) : selection.kind === "conversation" ? (
                <ConversationPage accountId={selection.accountId} id={selection.id} now={now} projects={projects} />
              ) : (
              <div className="min-h-0 flex-1">
              <RightInspector
              folded={inspectorFolded}
              onToggleFold={toggleInspector}
              focusTabsRequest={inspectorFocus}
              selection={
                selection.kind === "session"
                  ? { kind: "instance", id: selection.id }
                  : selection
              }
              data={{
                instance: selectedSession,
                project: selectedProject,
                account: selectedAccount,
                provider: selectedProvider ? { id: selectedProvider.id, name: selectedProvider.name } : null,
                machineId: "this-mac",
              }}
              sessions={sessions}
              projects={projects}
              accounts={accounts}
              profiles={profiles}
              terminalTitle={selectedSession ? `${selectedSession.account_label} · session` : null}
              freshness={freshness}
              now={now}
              primaryAction={
                selectedSession && (selectedSession.status === "running" || selectedSession.status === "starting") ? (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => stopSession(selectedSession)}
                    data-testid="inspector-stop"
                  >
                    Stop
                  </Button>
                ) : null
              }
              onOpenProject={selectProject}
              onOpenSession={selectSession}
              onLaunch={async (recipe, project) => {
                const doLaunch = async () => {
                  const result = await launchSession(recipe.id, project.root);
                  if (result) {
                    void refresh();
                    selectSession(result.session_id);
                  }
                };
                // AJ-E505 Req.4: warn BEFORE spawn when the project's
                // learned dev port is held by a managed orphan; the
                // launch itself is never blocked (SPEC §5.2 advisor).
                try {
                  const warning = await launchPortWarning(project.id ?? null);
                  if (warning) {
                    setPortWarning({ warning, proceed: () => void doLaunch() });
                    return;
                  }
                } catch {
                  // The pre-check is advisory: probe failures launch.
                }
                await doLaunch();
              }}
              onStopped={() => void refresh()}
              switchBackSuggestions={switchBackCards}
              onSwitchBack={(sessionId) => setSwitchBack(sessionId)}
              onHandover={(sessionId, targetProfileId, targetLabel) => {
                void sessionHandover; // superseded by the E205 flow
                setSwitchFlow({ sessionId, targetProfileId, targetLabel });
              }}
              onNew={() => setNewSession(true)}
            />
              </div>
              )}
            </div>
          )}
        </main>
      </div>

      {portWarning && (
        <div className="pointer-events-auto absolute inset-x-0 top-3 z-40 mx-auto w-fit max-w-[560px]">
          <PortWarningSheet
            warning={portWarning.warning}
            onFreePort={() => setPortWarning(null)}
            onLaunchAnyway={() => {
              const proceed = portWarning.proceed;
              setPortWarning(null);
              proceed();
            }}
          />
        </div>
      )}

      <UsageStatsReconsent />
      <CrashReportPrompt />
      {recovery !== null && (
        <RecoverySummaryDialog
          summary={recovery}
          onDismiss={() => {
            setRecovery(null);
            void import("./lib/bridge").then((bridge) => bridge.recoverySummaryDismiss());
          }}
        />
      )}

      {/* AJ-E709: "Ask each time" — a paired machine waits for this Mac's user. */}
      <FleetPromptDialog />
      <IdentityMismatchDialog />
      <IsolationOfferDialog />
      <GoToPalette
        open={goToOpen}
        onClose={() => setGoToOpen(false)}
        sessions={sessions}
        onFocusWindow={(instance) => {
          void desktopFocus(instance.id).then(() => {
            void refresh();
            selectSession(instance.id);
          });
        }}
        onOpenSession={selectSession}
        remoteMachines={remoteMachines ?? []}
        onFocusRemote={(machineId, instanceId) => {
          void fleetRemoteFocus(machineId, instanceId).then((outcome) => {
            setRemoteFocusOutcome({ machineId, instanceId, outcome });
            inspectRemote(machineId, instanceId);
          });
        }}
        onOpenRemote={inspectRemote}
        hotkey={goToHotkey}
        onHotkeyChange={async (combo) => {
          await goToHotkeySet(combo);
          setGoToHotkey(combo);
        }}
      />

      {firstRun && (
        <FirstRun
          resumeAtAccess={firstRunAtAccess}
          openLoginTerminal={openLoginTerminal}
          onDone={() => {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            setFirstRunAtAccess(false);
            setFirstRun(false);
            void refresh();
          }}
          onAccountsChanged={() => void refresh()}
        />
      )}

      {chooseTool && (
        <AddProviderSheet
          onClose={() => setChooseTool(false)}
          onChoose={(provider) => {
            setChooseTool(false);
            setAddAccounts(provider);
          }}
        />
      )}

      {addAccounts !== null && (
        <FirstRun
          mode="add"
          provider={addAccounts}
          startAt={addFresh ? "fresh" : "found"}
          openLoginTerminal={openLoginTerminal}
          onDone={() => {
            setAddAccounts(null);
            setAccountsNonce((n) => n + 1);
            void refresh();
          }}
          onAccountsChanged={() => void refresh()}
        />
      )}

      {limitFor && sessions.some((s) => s.id === limitFor) && (
        <LimitDialog
          session={sessions.find((s) => s.id === limitFor)!}
          sessions={sessions}
          now={now}
          onClose={() => setLimitFor(null)}
          onParked={() => void refresh()}
          onChooseAnother={() => {
            setLimitFor(null);
            selectSession(limitFor);
          }}
          onContinue={(target) => {
            setLimitFor(null);
            setSwitchFlow({ sessionId: limitFor, targetProfileId: target.recipe_id, targetLabel: target.account_label });
          }}
        />
      )}
      {switchFlow && (
        <SwitchFlow
          sessionId={switchFlow.sessionId}
          targetProfileId={switchFlow.targetProfileId}
          targetLabel={switchFlow.targetLabel}
          onClose={() => setSwitchFlow(null)}
          onSwitched={(sessionId, undo) => {
            setSwitchFlow(null);
            void refresh();
            selectSession(sessionId);
            // AJ-E206: a completed 1-click switch leaves the 10s undo
            // window open (the graph keeps the edge as the handle).
            if (undo) {
              showSwitchUndoToast({
                edgeId: undo.edgeId,
                targetLabel: switchFlow.targetLabel,
                onUndone: (undoneId) => {
                  void refresh();
                  selectSession(undoneId);
                },
              });
            }
          }}
        />
      )}
      {switchBack && (
        <SwitchBackFlow
          sessionId={switchBack}
          onClose={() => setSwitchBack(null)}
          onSwitched={(sessionId) => {
            setSwitchBack(null);
            void refresh();
            selectSession(sessionId);
          }}
        />
      )}
      {/* AJ-E1503: the one feedback system, and the quit warning. */}
      <ToastHost />
      {shortcutsOpen && <ShortcutSheet onClose={() => setShortcutsOpen(false)} />}
      <QuitSheet />
      {newSession && (
        <NewSessionModal
          projects={shownProjects}
          accounts={accounts}
          sessions={sessions}
          initialDirectory={newSessionDir}
          openLoginTerminal={openLoginTerminal}
          onClose={closeNewSession}
          onLaunched={(sessionId) => {
            // AJ-E309 Req.6: once per install (the backend dedupes and
            // computes seconds_since_install itself).
            funnelEvent("first_launch");
            closeNewSession();
            void refresh();
            selectSession(sessionId);
          }}
          onManageLaunchSettings={() => {
            closeNewSession();
            setGlobal("accounts");
          }}
        />
      )}

      {/* AJ-E305 flow (b): New project — 2 steps, ≤ 3 clicks, no
          typed paths; an empty preset means "no directory yet". */}
      {newProjectDir !== null && (
        <NewProjectSheet
          projects={shownProjects}
          accounts={accounts}
          initialDirectory={newProjectDir === "" ? null : newProjectDir}
          onClose={() => setNewProjectDir(null)}
          onLaunched={(sessionId) => {
            funnelEvent("first_launch");
            setNewProjectDir(null);
            void refresh();
            selectSession(sessionId);
          }}
          openLoginTerminal={openLoginTerminal}
          onAccountsChanged={() => void refresh()}
          onManageLaunchSettings={() => {
            setNewProjectDir(null);
            setGlobal("accounts");
          }}
        />
      )}
    </div>
    </AppActions>
    </ConversationsProvider>
    </ThisMacContext.Provider>
  );
}


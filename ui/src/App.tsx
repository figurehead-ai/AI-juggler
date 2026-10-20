import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Button, Card, Chip } from "./components/primitives";
import { WorkflowRail, type Freshness } from "./components/WorkflowRail";
import { GoToPalette } from "./components/GoToPalette";
import { PortWarningSheet } from "./components/PortMap";
import { RecoverySummaryDialog } from "./components/RecoverySummaryDialog";
import { UsageStatsReconsent } from "./components/UsageStatsReconsent";
import { CrashReportPrompt } from "./components/CrashReports";
import { CleanupCentre, focusCleanupFilter } from "./screens/CleanupCentre";
import { launchPortWarning, type PortWarningInfo,} from "./lib/bridge";
import { IconJuggler, IconTerminal } from "./components/icons";
import { AttentionAnnouncer, IndicatorLegend, RenewalAnnouncer } from "./components/indicator";
import {
  DENSITY_EVENT,
  isDensityShortcut as isDensityShortcutEvent,
  nextDensity,
  readDensity,
  storeDensity,
  type Density,
} from "./lib/density";
import {
  hasLiveBackend,
  fleetRemote,
  fleetRemoteFocus,
  type RemoteCommandOutcomeInfo,
  type RemoteMachineInfo,
} from "./lib/bridge";
import { isHandoverShortcut, isStopShortcut } from "./lib/shortcuts";
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

/** AJ-E1103: one keyboard shortcut used (the schema's names). */
function shortcutUsed(shortcut: string): void {
  statsEvent("ui.shortcut_used", { shortcut });
}

/** AJ-E1103: the board became interactive once, after launch. */
let boardInteractiveCounted = false;
import { ProfilesScreen } from "./screens/Profiles";
import { History } from "./screens/History";
import { OnboardingWizard } from "./screens/OnboardingWizard";
import { FirstRun } from "./screens/FirstRun";
import { HomeScreen } from "./screens/Home";
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
import { SwitchFlow, SwitchUndoToast } from "./components/SwitchFlow";
import { SwitchBackFlow } from "./components/SwitchBackFlow";
import { RightInspector } from "./components/RightInspector";
import {
  accountsList,
  type SwitchBackSuggestionData,
  launchRecipesList,
  onSessionsQuota,
  onSessionsResources,
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

const GLOBAL_NAV = [
  // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
  // AJ-E1007: every ended Instance — search, archive, replay.
  { id: "history", label: "History" },
  { id: "accounts", label: "Accounts" },
  { id: "usage", label: "Usage" },
  { id: "fleet", label: "Fleet" },
  // AJ-E506: the Cleanup Centre is a GLOBAL surface (ADR-0007) —
  // never a rail peer of Projects/Instances/Sessions.
  { id: "cleanup", label: "Cleanup" },
  // AJ-E1203: Appearance + the optional Account & Cloud (⌘,).
  { id: "settings", label: "Settings" },
] as const;

type GlobalId = (typeof GLOBAL_NAV)[number]["id"];

/** D-25: the header's four destinations; the rest are one click away
 * under More (Fleet joins the header once a Mac is paired; Cleanup shows
 * as a pill while something is left behind). */
const PRIMARY_NAV: Array<{ id: GlobalId | "home"; label: string }> = [
  { id: "home", label: "Home" },
  { id: "accounts", label: "Accounts" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];
const MORE_NAV: Array<{ id: GlobalId; label: string; hint: string }> = [
  { id: "usage", label: "Usage", hint: "Quota, cost and the two clocks per account" },
  { id: "fleet", label: "Fleet", hint: "Your other Macs" },
  { id: "cleanup", label: "Cleanup", hint: "Processes left behind, and the audit log" },
];

/** D-25: below this width the rail folds to icons by itself. */
const NARROW_RAIL_PX = 900;

/** True while the window is narrower than `px` (follows resizes). */
function useNarrowWindow(px: number): boolean {
  const query = `(max-width: ${px - 1}px)`;
  const read = () => typeof window.matchMedia === "function" && window.matchMedia(query).matches;
  const [narrow, setNarrow] = useState(read);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const list = window.matchMedia(query);
    const follow = () => setNarrow(list.matches);
    follow();
    list.addEventListener("change", follow);
    return () => list.removeEventListener("change", follow);
  }, [query]);
  return narrow;
}

/** Display names for the header's Provider chip. */
const PROVIDER_NAMES: Record<string, string> = {
  "claude-code": "Claude Code",
  codex: "Codex",
  "gemini-cli": "Gemini CLI",
  zcode: "ZCode",
};

type Selection =
  | { kind: "none" }
  | { kind: "project"; id: string }
  | { kind: "session"; id: string }
  | { kind: "login"; id: string }
  | { kind: "account"; id: string }
  | { kind: "provider"; id: string }
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
  const cycleDensity = useCallback(() => {
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
  // comes back when the window widens; the stored density never changes.
  // ⌘B (or the logo) opens it for now while narrow.
  const narrow = useNarrowWindow(NARROW_RAIL_PX);
  const [narrowOpen, setNarrowOpen] = useState(false);
  useEffect(() => {
    if (!narrow) setNarrowOpen(false);
  }, [narrow]);
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
  const [moreOpen, setMoreOpen] = useState(false);
  const moreRef = useRef<HTMLDivElement | null>(null);
  // The More menu closes on a click elsewhere or Escape, like any menu.
  useEffect(() => {
    if (!moreOpen) return;
    const onDown = (event: MouseEvent) => {
      if (moreRef.current && !moreRef.current.contains(event.target as Node)) setMoreOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setMoreOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [moreOpen]);
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
  const railSessions = useMemo(() => sessions.filter((row) => row.lifecycle !== "archived"), [sessions]);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[]>([]);
  const [newSessionDir, setNewSessionDir] = useState<string | null>(null);
  const [loginTerminals, setLoginTerminals] = useState<
    { id: string; title: string; ended: boolean }[]
  >([]);
  const [newSession, setNewSession] = useState(false);
  // AJ-E205: the executing switch flow (freeze dialog → stages →
  // source-safe failure banner). Null = closed.
  // D-25: Accounts › Add account reuses first run's three steps.
  const [addAccounts, setAddAccounts] = useState(false);
  // The wizard opened from Accounts is locked to its Provider (AJ-E306).
  const [wizardLocked, setWizardLocked] = useState(false);
  const [accountsNonce, setAccountsNonce] = useState(0);
  // D-25: the account-limit decision, for this session id.
  const [limitFor, setLimitFor] = useState<string | null>(null);
  const [switchFlow, setSwitchFlow] = useState<{ sessionId: string; targetProfileId: string; targetLabel: string } | null>(null);
  // AJ-E206: the 10-second undo window after a 1-click switch (the
  // edge is the undo handle). Null = no window open.
  const [undoWindow, setUndoWindow] = useState<{ edgeId: string; targetLabel: string } | null>(null);
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
  const [wizard, setWizard] = useState(false);
  // D-25: a fresh install opens on first run (what's here → sign in →
  // computer access); the wizard stays for Accounts › Add account.
  const [firstRun, setFirstRun] = useState(false);
  // AJ-E305: [Add another account] reopens the wizard fresh — the
  // nonce remounts it (no stale step state).
  const [wizardNonce, setWizardNonce] = useState(0);
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
    window.addEventListener("aij:open-accounts", openAccounts);
    window.addEventListener("aij:new-session", newSession);
    return () => {
      window.removeEventListener("aij:open-accounts", openAccounts);
      window.removeEventListener("aij:new-session", newSession);
    };
  }, []);

  // First run: zero accounts → first run opens by itself (spec §12.1,
  // D-25), once per dismissal.
  useEffect(() => {
    if (localStorage.getItem("aij.onboarding.dismissed") === "1") return;
    // Test seam (browser preview only): open the Add-account wizard
    // instead, as if the user clicked Add account — its gallery and
    // accessibility captures start from a fresh install.
    const seam = (window as unknown as { __AIJ_FIRST_RUN__?: string }).__AIJ_FIRST_RUN__;
    void accountsList().then((accounts) => {
      if (accounts.length === 0) {
        if (seam === "wizard") setWizard(true);
        else setFirstRun(true);
      }
    });
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
      setProjects(projectRows);
      setSessions(sessionRows);
      setAccounts(accountRows);
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
    }
  }, [baselineFreshness]);

  // Golden path day one: surface the user's REAL project history from
  // imported transcripts (idempotent backend scan).
  useEffect(() => {
    void projectsBackfill().then(() => void refresh());
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

  // AJ-E402/E401 read-model ticks: merge the coalesced 5 s resource and
  // 30 s quota deltas into the rows by id (no full refetch per tick) —
  // this is what drives the I-07 sparkline history and quota countdowns.
  // Each delta also refreshes the sampled-at timestamp per id (stale rule).
  useEffect(() => {
    const stopResources = onSessionsResources((deltas) => {
      const at = Date.now();
      setSessions((current) =>
        current.map((row) => {
          const delta = deltas.find((d) => d.id === row.id);
          return delta
            ? {
                ...row,
                cpu_pct: delta.cpu_pct,
                rss_bytes: delta.rss_bytes,
                child_count: delta.child_count,
              }
            : row;
        }),
      );
      setFreshness((current) => {
        const next = { ...current };
        for (const delta of deltas) {
          if (next[delta.id]) next[delta.id] = { ...next[delta.id]!, resources: at };
        }
        return next;
      });
    });
    const stopQuota = onSessionsQuota((deltas) => {
      const at = Date.now();
      setSessions((current) =>
        current.map((row) => {
          const delta = deltas.find((d) => d.id === row.id);
          return delta ? { ...row, quota: delta.quota } : row;
        }),
      );
      setFreshness((current) => {
        const next = { ...current };
        for (const delta of deltas) {
          if (next[delta.id]) next[delta.id] = { ...next[delta.id]!, quota: at };
        }
        return next;
      });
    });
    return () => {
      stopResources();
      stopQuota();
    };
  }, []);

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
    return { id: selection.id, name: PROVIDER_NAMES[selection.id] ?? selection.id, accounts: providerAccounts };
  }, [selection, accounts]);


  // ⌘B legacy: toggles minimal↔comfortable (the old collapse, kept for
  // muscle memory). ⌘N starts a New project (AJ-E305 flow (b)). Density
  // cycle: Ctrl+D everywhere; Cmd+D only inside the Tauri app — the web
  // preview must not swallow the browser bookmark shortcut (AJ-E403 Req.4).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
        event.preventDefault();
        shortcutUsed("toggle_rail");
        toggleRail();
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        shortcutUsed("new_session");
        setNewProjectDir((current) => current ?? "");
      }
      // ⌘. stops the selected live Instance (AJ-E407 Actions); ⌘⇧H
      // focuses the hand-over section (RightInspector switches tabs).
      if (isStopShortcut(event)) {
        event.preventDefault();
        shortcutUsed("other");
        if (
          selectedSession &&
          (selectedSession.status === "running" || selectedSession.status === "starting")
        ) {
          void import("./lib/bridge").then(async ({ terminalKill }) => {
            await terminalKill(selectedSession.id);
            void refresh();
          });
        }
      }
      if (isHandoverShortcut(event)) {
        event.preventDefault();
        shortcutUsed("switch");
        window.dispatchEvent(new Event("aij:focus-handover"));
      }
      // ⌘K/Ctrl+K focuses the rail search (AJ-E404 Req.4); Esc clears it
      // inside the input and focus stays there.
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        window.dispatchEvent(new Event("aij:focus-search"));
      }
      if (isDensityShortcutEvent(event, hasLiveBackend())) {
        event.preventDefault();
        shortcutUsed("other");
        cycleDensity();
      }
      if (
        (event.metaKey || event.ctrlKey) &&
        event.shiftKey &&
        event.key.toLowerCase() === "l"
      ) {
        event.preventDefault();
        shortcutUsed("other");
        void import("./theme").then((theme) => theme.cycleTheme());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cycleDensity, toggleRail]);

  useEffect(() => {
    void appSettingGet("go_to_hotkey").then((stored) => {
      if (stored !== null) setGoToHotkey(stored);
    });
    return onGoToHotkey(() => setGoToOpen(true));
  }, []);

  // ⌘K/Ctrl+K toggles the palette in-app (the global binding covers
  // the over-any-app case; both land on the same list).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        shortcutUsed("command_palette");
        setGoToOpen((openNow) => !openNow);
      }
      // AJ-E704: ⌘⇧F opens Fleet from anywhere.
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        shortcutUsed("other");
        setGlobal("fleet");
      }
      // AJ-E1203: ⌘, opens Settings (the macOS convention).
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key === ",") {
        event.preventDefault();
        shortcutUsed("other");
        setSettingsFocusCloud(false);
        setSettingsSection(undefined);
        setGlobal("settings");
      }
      // AJ-E506: ⌘⇧C opens the Cleanup Centre; ⌘F focuses its filter.
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "c") {
        event.preventDefault();
        shortcutUsed("other");
        setGlobal("cleanup");
      }
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "f" && global === "cleanup") {
        event.preventDefault();
        shortcutUsed("search");
        focusCleanupFilter();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [global]);

  // The Provider of what's selected — never a fixed label.
  const remoteInstanceProvider =
    selection.kind === "remote" && selection.instanceId
      ? (remoteMachines
          ?.find((machine) => machine.machine_id === selection.machineId)
          ?.instances.find((row) => row.id === selection.instanceId)?.provider ?? null)
      : null;
  const contextProviderId =
    global !== null
      ? null
      : (selectedSession?.provider ??
        remoteInstanceProvider ??
        (selection.kind === "provider" ? selection.id : null));
  const contextProvider = contextProviderId ? (PROVIDER_NAMES[contextProviderId] ?? contextProviderId) : null;

  const contextTitle =
    global !== null
      ? (GLOBAL_NAV.find((g) => g.id === global)?.label ?? "")
      : selection.kind === "remote"
        ? `Fleet · ${remoteMachines?.find((m) => m.machine_id === selection.machineId)?.label ?? "machine"}`
        : selection.kind === "project"
        ? (selectedProject?.name ?? "Project")
        : selectedSession
          ? // D-25: the session's project (the view below names its place).
            (projects.find((p) => p.id === selectedSession.project_id)?.name ?? "Session")
          : selectedLogin
            ? selectedLogin.title
            : "Home";

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <AttentionAnnouncer sessions={sessions} />
      <RenewalAnnouncer accounts={accounts} />
      <WorkflowRail
        density={railDensity}
        onCycleDensity={cycleDensity}
        projects={projects}
        sessions={railSessions}
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
            <div className="flex justify-center py-2" title="this-mac · healthy">
              <span className="dot-live size-1.5 rounded-full bg-success" />
            </div>
          ) : (
            <div className="flex flex-col gap-1.5 px-3 py-2.5">
              <div className="flex min-w-0 items-center gap-2 overflow-hidden">
                <span className="dot-live size-1.5 shrink-0 rounded-full bg-success" />
                <span className="min-w-0 shrink truncate text-[11px]">this-mac · healthy</span>
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
            {contextProvider && <Chip tone="accent">{contextProvider}</Chip>}
          </div>

          <div className="ml-auto flex min-w-0 max-w-full flex-wrap items-center justify-end gap-0.5 xl:gap-1.5" data-tauri-drag-region={false}>
            <nav aria-label="Main" className="flex flex-wrap items-center gap-0.5 xl:gap-1">
              {PRIMARY_NAV.map((item) => {
                const current = item.id === "home" ? global === null && selection.kind === "none" : global === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    aria-current={current ? "page" : undefined}
                    onClick={() => {
                      setSettingsFocusCloud(false);
                      setSettingsSection(undefined);
                      setMoreOpen(false);
                      if (item.id === "home") {
                        setGlobal(null);
                        setSelection({ kind: "none" });
                      } else {
                        setGlobal(item.id);
                      }
                    }}
                    className={
                      "rounded-md px-2 py-1.5 text-xs font-medium transition-colors xl:px-2.5 " +
                      (current ? "bg-surface text-ink sheen" : "text-muted hover:bg-surface/60 hover:text-ink")
                    }
                  >
                    {item.label}
                  </button>
                );
              })}
              {pairedCount > 0 && (
                <button
                  type="button"
                  aria-current={global === "fleet" ? "page" : undefined}
                  onClick={() => {
                    setMoreOpen(false);
                    setGlobal("fleet");
                  }}
                  className={
                    "rounded-md px-2 py-1.5 text-xs font-medium transition-colors xl:px-2.5 " +
                    (global === "fleet" ? "bg-surface text-ink sheen" : "text-muted hover:bg-surface/60 hover:text-ink")
                  }
                >
                  Fleet
                </button>
              )}
              {cleanupCount > 0 && global !== "cleanup" && (
                <button
                  type="button"
                  onClick={() => {
                    setMoreOpen(false);
                    setGlobal("cleanup");
                  }}
                  className="ml-1 rounded-full border border-warning/40 bg-warning-soft px-2.5 py-1 text-[11px] font-medium text-ink"
                  data-testid="nav-cleanup-pill"
                >
                  {cleanupCount === 1 ? "1 thing to clean up" : `${cleanupCount} things to clean up`}
                </button>
              )}
              <div className="relative" ref={moreRef}>
                <button
                  type="button"
                  aria-expanded={moreOpen}
                  aria-controls="nav-more"
                  onClick={() => setMoreOpen((open) => !open)}
                  className={
                    "rounded-md px-2 py-1.5 text-xs font-medium transition-colors xl:px-2.5 " +
                    (MORE_NAV.some((item) => item.id === global) ? "bg-surface text-ink sheen" : "text-muted hover:bg-surface/60 hover:text-ink")
                  }
                >
                  More
                </button>
                {moreOpen && (
                  <div
                    id="nav-more"
                    className="absolute right-0 top-full z-40 mt-1.5 flex w-72 flex-col rounded-xl border border-hairline bg-surface p-1.5 shadow-xl"
                    onKeyDown={(event) => {
                      if (event.key === "Escape") setMoreOpen(false);
                    }}
                  >
                    {MORE_NAV.map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        aria-current={global === item.id ? "page" : undefined}
                        onClick={() => {
                          setMoreOpen(false);
                          setGlobal(item.id);
                        }}
                        className="flex flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left hover:bg-surface-2"
                      >
                        <span className="text-xs font-medium text-ink">{item.label}</span>
                        <span className="text-[11px] text-muted">{item.hint}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </nav>
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
              <CleanupCentre />
            </div>
          ) : global === "accounts" ? (
            <div className="h-full overflow-y-auto">
              <ProfilesScreen
                key={accountsNonce}
                sessions={sessions}
                onAddAccounts={() => setAddAccounts(true)}
                openTerminal={openLoginTerminal}
                onLaunchFirstSession={(startHere) => {
                  setNewSession(true);
                  setNewSessionDir(startHere);
                }}
                onInspectAccount={(id) => {
                  setGlobal(null);
                  setSelection({ kind: "account", id });
                }}
              />
            </div>
          ) : global === "history" ? (
            <div className="h-full overflow-y-auto">
              <History sessions={sessions} projects={projects} onChanged={() => void refresh()} />
            </div>
          ) : global === "usage" ? (
            <div className="h-full overflow-y-auto">
              <UsageScreen accounts={accounts} sessions={sessions} onAddAccounts={() => setFirstRun(true)} />
            </div>
          ) : global === "fleet" ? (
            <div className="h-full overflow-y-auto">
              <FleetScreen onOpenCloud={openCloudSettings} onInspectRemote={inspectRemote} />
            </div>
          ) : global === "settings" ? (
            <div className="h-full overflow-y-auto">
              <SettingsScreen key={settingsSection ?? "top"} focusCloud={settingsFocusCloud} focusSection={settingsSection} />
            </div>
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
                <RightInspector
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
              );
            })()
          ) : (
            <>
              {/* AJ-E611: the alert/notification region — polite
                  live-region, badge-carrying copy, reset-queue rows
                  with an explicit Resume click (the click opens the
                  picker; nothing ever launches by itself). */}
              <NotificationRegion
                onQueueResume={() => setNewSession(true)}
                onOpenRemote={(machineId) => inspectRemote(machineId, null)}
              />
              {selection.kind === "none" ? (
                <div className="h-full overflow-y-auto">
                  <HomeScreen
                    sessions={sessions}
                    accounts={accounts}
                    projects={projects}
                    now={now}
                    onOpenSession={selectSession}
                    onNewSession={() => setNewSession(true)}
                    onAddAccounts={() => setFirstRun(true)}
                    onLimit={setLimitFor}
                    openLoginTerminal={openLoginTerminal}
                    onAccountsChanged={() => void refresh()}
                    onChanged={() => void refresh()}
                  />
                </div>
              ) : (
              <RightInspector
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
                    onClick={async () => {
                      const { terminalKill } = await import("./lib/bridge");
                      await terminalKill(selectedSession.id);
                      void refresh();
                    }}
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
              )}
            </>
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
          openLoginTerminal={openLoginTerminal}
          onDone={() => {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            setFirstRun(false);
            void refresh();
          }}
          onAddFresh={() => {
            setFirstRun(false);
            setWizard(true);
          }}
          onImportFolder={() => {
            setFirstRun(false);
            setWizard(true);
          }}
        />
      )}

      {addAccounts && (
        <FirstRun
          mode="add"
          openLoginTerminal={openLoginTerminal}
          onDone={() => {
            setAddAccounts(false);
            setAccountsNonce((n) => n + 1);
            void refresh();
          }}
          onAddFresh={() => {
            setAddAccounts(false);
            setWizardLocked(true);
            setWizard(true);
          }}
          onImportFolder={() => {
            setAddAccounts(false);
            setWizardLocked(true);
            setWizard(true);
          }}
        />
      )}

      {wizard && (
        <OnboardingWizard
          key={wizardNonce}
          provider="claude-code"
          locked={wizardLocked}
          onClose={() => {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            setWizard(false);
            setWizardLocked(false);
            setAccountsNonce((n) => n + 1);
          }}
          onAccountCreated={() => void refresh()}
          onConnectCloud={openCloudSettings}
          openLoginTerminal={openLoginTerminal}
          onLaunchFirstSession={(startHere) => {
            setWizard(false);
            setNewSession(true);
            setNewSessionDir(startHere);
          }}
          onAddAnother={() => {
            setWizard(false);
            setWizardNonce((nonce) => nonce + 1);
            setWizard(true);
          }}
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
              setUndoWindow({
                edgeId: undo.edgeId,
                targetLabel: switchFlow.targetLabel,
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
      {undoWindow && (
        <SwitchUndoToast
          edgeId={undoWindow.edgeId}
          targetLabel={undoWindow.targetLabel}
          onDismiss={() => setUndoWindow(null)}
          onUndone={(sessionId) => {
            setUndoWindow(null);
            void refresh();
            selectSession(sessionId);
          }}
        />
      )}
      {newSession && (
        <NewSessionModal
          projects={projects}
          accounts={accounts}
          initialDirectory={newSessionDir}
          openLoginTerminal={openLoginTerminal}
          onClose={() => {
            setNewSession(false);
            setNewSessionDir(null);
          }}
          onLaunched={(sessionId) => {
            // AJ-E309 Req.6: once per install (the backend dedupes and
            // computes seconds_since_install itself).
            funnelEvent("first_launch");
            setNewSession(false);
            setNewSessionDir(null);
            void refresh();
            selectSession(sessionId);
          }}
          onManageLaunchSettings={() => {
            setNewSession(false);
            setNewSessionDir(null);
            setGlobal("accounts");
          }}
        />
      )}

      {/* AJ-E305 flow (b): New project — 2 steps, ≤ 3 clicks, no
          typed paths; an empty preset means "no directory yet". */}
      {newProjectDir !== null && (
        <NewProjectSheet
          projects={projects}
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
  );
}

/** The `?` legend popover button + panel (AJ-E403 Req.6). The panel is
 * fixed-positioned above the button and portalled to the body: the
 * footer row clips overflow and the rail is its own stacking layer, so
 * a panel placed inside it opened out of sight (0.0.6 dogfood). */
function LegendPopover() {
  const [open, setOpen] = useState(false);
  const [place, setPlace] = useState<{ left: number; bottom: number; maxHeight: number } | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const toggle = () => {
    if (open) {
      setOpen(false);
      return;
    }
    const rect = buttonRef.current?.getBoundingClientRect();
    if (rect) {
      const width = 320;
      setPlace({
        left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
        bottom: window.innerHeight - rect.top + 8,
        maxHeight: Math.max(160, rect.top - 16),
      });
    }
    setOpen(true);
  };
  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!panelRef.current?.contains(target) && !buttonRef.current?.contains(target)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);
  return (
    <span className="ml-auto shrink-0">
      <button
        ref={buttonRef}
        type="button"
        aria-expanded={open}
        aria-controls="indicator-legend"
        aria-label="Indicator legend"
        onClick={toggle}
        // A 24 px target around the 16 px glyph (WCAG 2.5.8), same layout.
        className="group -my-1 flex p-1 outline-none"
      >
        <span className="flex size-4 items-center justify-center rounded-full border border-hairline text-[9px] font-bold text-muted transition-colors group-hover:bg-surface group-hover:text-ink group-focus-visible:ring-1 group-focus-visible:ring-accent/60">
          ?
        </span>
      </button>
      {open && place && createPortal(
        <div
          ref={panelRef}
          id="indicator-legend"
          role="dialog"
          aria-label="What the icons mean"
          data-testid="indicator-legend"
          style={{ left: place.left, bottom: place.bottom, maxHeight: place.maxHeight }}
          className="fixed z-50 w-80 overflow-y-auto rounded-xl border border-hairline bg-surface p-3 shadow-card"
        >
          <IndicatorLegend />
        </div>,
        // At the top of the page: the rail is its own stacking layer, so
        // anything placed inside it paints under the work area.
        document.body,
      )}
    </span>
  );
}

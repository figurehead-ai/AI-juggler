import { useCallback, useEffect, useMemo, useState } from "react";
import { Button, Card, Chip, QuotaBar, QuotaRing, StatCard } from "./components/primitives";
import { SessionCard, WorkflowRail, type Freshness, type WorkTab } from "./components/WorkflowRail";
import { GoToPalette } from "./components/GoToPalette";
import { PortWarningSheet } from "./components/PortMap";
import { RecoverySummaryDialog } from "./components/RecoverySummaryDialog";
import { CleanupCentre, focusCleanupFilter } from "./screens/CleanupCentre";
import { launchPortWarning, type PortWarningInfo,} from "./lib/bridge";
import {
  IconFleet,
  IconJuggler,
  IconTerminal,
  IconUsage,
} from "./components/icons";
import { AttentionAnnouncer, IndicatorLegend, RenewalAnnouncer } from "./components/indicator";
import {
  DENSITY_ORDER,
  isDensityShortcut as isDensityShortcutEvent,
  nextDensity,
  readDensity,
  storeDensity,
  type Density,
} from "./lib/density";
import { hasLiveBackend } from "./lib/bridge";
import { isHandoverShortcut, isStopShortcut } from "./lib/shortcuts";
import { ProfilesScreen } from "./screens/Profiles";
import { OnboardingWizard } from "./screens/OnboardingWizard";
import { NewSessionModal } from "./screens/NewSession";
import { NewProjectSheet } from "./screens/NewProjectSheet";
import { funnelEvent } from "./lib/funnel";
import { ReadyHome } from "./screens/ReadyHome";
import { SessionPanel } from "./screens/WorkPanels";
import { CostMatrix } from "./components/CostMatrix";
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
  dashboardStats,
  fetchAppInfo,
  onAppReady,
  type AccountInfo,
  type AppInfo,
  type ProjectInfo,
  type InstanceInfo,
  type StatsInfo,
} from "./lib/bridge";

/**
 * App shell (ADR-0007, design iteration 5): a workflow navigator rail
 * (Projects / Instances / Sessions — the list lives in the rail) plus the
 * detail panel where the selected item loads, session terminals embedded.
 * Global oversight (Overview, Accounts, Usage, Fleet) sits top-right.
 */

const GLOBAL_NAV = [
  { id: "overview", label: "Overview" },
  { id: "accounts", label: "Accounts" },
  { id: "usage", label: "Usage" },
  { id: "fleet", label: "Fleet" },
  // AJ-E506: the Cleanup Centre is a GLOBAL surface (ADR-0007) —
  // never a rail peer of Projects/Instances/Sessions.
  { id: "cleanup", label: "Cleanup" },
] as const;

type GlobalId = (typeof GLOBAL_NAV)[number]["id"];

type Selection =
  | { kind: "none" }
  | { kind: "project"; id: string }
  | { kind: "session"; id: string }
  | { kind: "login"; id: string }
  | { kind: "account"; id: string }
  | { kind: "provider"; id: string };

export default function App() {
  const [tab, setTab] = useState<WorkTab>("projects");
  // Density (AJ-E403): persisted in `aij.density`; the legacy
  // `aij.rail.collapsed` boolean migrates into `minimal` on first read.
  const [density, setDensity] = useState<Density>(() => readDensity());
  const cycleDensity = useCallback(() => {
    setDensity((current) => {
      const next = nextDensity(current);
      storeDensity(next);
      return next;
    });
  }, []);
  const setDensityAndStore = useCallback((next: Density) => {
    setDensity(next);
    storeDensity(next);
  }, []);

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
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [sessions, setSessions] = useState<InstanceInfo[]>([]);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[]>([]);
  const [readySkipped, setReadySkipped] = useState(
    () => sessionStorage.getItem("aij.ready.skipped") === "1",
  );
  const [newSessionDir, setNewSessionDir] = useState<string | null>(null);
  const [loginTerminals, setLoginTerminals] = useState<
    { id: string; title: string; ended: boolean }[]
  >([]);
  const [newSession, setNewSession] = useState(false);
  // AJ-E205: the executing switch flow (freeze dialog → stages →
  // source-safe failure banner). Null = closed.
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

  // First run: zero accounts → the onboarding wizard opens by itself
  // (spec §12.1), once per dismissal.
  useEffect(() => {
    if (localStorage.getItem("aij.onboarding.dismissed") === "1") return;
    void accountsList().then((accounts) => {
      if (accounts.length === 0) setWizard(true);
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
    setTab("sessions");
    setGlobal(null); // the panel must show the terminal, not stay on Accounts
    setSelection({ kind: "login", id });
  }, []);

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
  const showReadyHome =
    global === null &&
    accounts.length > 0 &&
    accounts.some((a) => a.login_state !== "logged_in") &&
    !readySkipped;

  const selectedAccount = useMemo(
    () => (selection.kind === "account" ? (accounts.find((a) => a.id === selection.id) ?? null) : null),
    [selection, accounts],
  );
  const selectedProvider = useMemo(() => {
    if (selection.kind !== "provider") return null;
    const providerAccounts = accounts.filter((a) => a.provider === selection.id);
    return { id: selection.id, name: selection.id, accounts: providerAccounts };
  }, [selection, accounts]);


  // ⌘B legacy: toggles minimal↔comfortable (the old collapse, kept for
  // muscle memory). ⌘N starts a New project (AJ-E305 flow (b)). Density
  // cycle: Ctrl+D everywhere; Cmd+D only inside the Tauri app — the web
  // preview must not swallow the browser bookmark shortcut (AJ-E403 Req.4).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
        event.preventDefault();
        setDensityAndStore(density === "minimal" ? "comfortable" : "minimal");
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setNewProjectDir((current) => current ?? "");
      }
      // ⌘. stops the selected live Instance (AJ-E407 Actions); ⌘⇧H
      // focuses the hand-over section (RightInspector switches tabs).
      if (isStopShortcut(event)) {
        event.preventDefault();
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
        cycleDensity();
      }
      if (
        (event.metaKey || event.ctrlKey) &&
        event.shiftKey &&
        event.key.toLowerCase() === "l"
      ) {
        event.preventDefault();
        void import("./theme").then((theme) => theme.cycleTheme());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cycleDensity, density, setDensityAndStore]);

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
        setGoToOpen((openNow) => !openNow);
      }
      // AJ-E506: ⌘⇧C opens the Cleanup Centre; ⌘F focuses its filter.
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "c") {
        event.preventDefault();
        setGlobal("cleanup");
      }
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "f" && global === "cleanup") {
        event.preventDefault();
        focusCleanupFilter();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [global]);

  const contextTitle =
    global !== null
      ? (GLOBAL_NAV.find((g) => g.id === global)?.label ?? "")
      : selection.kind === "project"
        ? (selectedProject?.name ?? "Project")
        : selectedSession
          ? selectedSession.account_label
          : selectedLogin
            ? selectedLogin.title
            : tab === "projects"
              ? "Projects"
              : tab === "instances"
                ? "Instances"
                : "Sessions";

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <AttentionAnnouncer sessions={sessions} />
      <RenewalAnnouncer accounts={accounts} />
      <WorkflowRail
        tab={tab}
        onTab={setTab}
        density={density}
        onCycleDensity={cycleDensity}
        projects={projects}
        sessions={sessions}
        accounts={accounts}
        loginTerminals={loginTerminals}
        selectedId={selection.kind === "none" ? null : selection.id}
        onSelectSession={(id) =>
          loginTerminals.some((t) => t.id === id) ? selectLogin(id) : selectSession(id)
        }
        onSelectProject={selectProject}
        onNewSession={() => setNewSession(true)}
        onNewProject={() => setNewProjectDir((current) => current ?? "")}
        freshness={freshness}
        now={now}
        header={(minimal) => (
          <div
            className={
              "flex h-14 shrink-0 items-center gap-2.5 overflow-hidden " +
              (minimal ? "justify-center px-0" : "px-4")
            }
          >
            <button
              type="button"
              title={minimal ? "Comfortable (⌘B)" : "Minimal (⌘B)"}
              onClick={() => setDensityAndStore(density === "minimal" ? "comfortable" : "minimal")}
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
          className="flex h-14 shrink-0 items-center gap-3 border-b border-hairline bg-surface/50 px-5 backdrop-blur-xl"
        >
          <h1 className="text-sm font-semibold capitalize tracking-wide">{contextTitle}</h1>
          <Chip tone="accent">Claude Code</Chip>

          <div className="ml-auto flex items-center gap-1.5" data-tauri-drag-region={false}>
            {/* Density segmented control (AJ-E403 Req.4). The inspector
                and panels never change with density — only the rail. */}
            <div
              role="group"
              aria-label="Rail density"
              className="flex items-center rounded-lg border border-hairline bg-surface/60 p-0.5"
            >
              {DENSITY_ORDER.map((mode) => (
                <button
                  key={mode}
                  type="button"
                  aria-pressed={density === mode}
                  onClick={() => setDensityAndStore(mode)}
                  title={`${mode} rail${mode === "minimal" ? " (Ctrl+D cycles)" : ""}`}
                  className={
                    "rounded-md px-2 py-1 text-[10.5px] font-medium capitalize transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
                    (density === mode
                      ? "bg-accent-soft text-accent-hi"
                      : "text-muted hover:bg-surface hover:text-ink")
                  }
                >
                  {mode === "comfortable" ? "Comfort" : mode === "compact" ? "Compact" : "Minimal"}
                </button>
              ))}
            </div>
            {GLOBAL_NAV.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setGlobal(item.id)}
                className={
                  "rounded-md px-2.5 py-1.5 text-[11px] font-medium transition-colors " +
                  (global === item.id
                    ? "bg-surface text-ink sheen"
                    : "text-muted hover:bg-surface/60 hover:text-ink")
                }
              >
                {item.label}
              </button>
            ))}
            <Button size="sm" variant="primary" onClick={() => setNewSession(true)}>
              <IconTerminal width={14} height={14} />
              New
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
          ) : global === "overview" ? (
            <div className="h-full overflow-y-auto">
              <Overview
                sessions={sessions}
                projects={projects}
                onSelectSession={selectSession}
                onInspectProvider={(provider) => {
                  setGlobal(null);
                  setSelection({ kind: "provider", id: provider });
                }}
              />
            </div>
          ) : global === "usage" ? (
            <ComingSoon screen="Usage" />
          ) : global === "fleet" ? (
            <ComingSoon screen="Fleet" />
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
          ) : showReadyHome && selection.kind === "none" ? (
            <ReadyHome
              accounts={accounts}
              openTerminal={openLoginTerminal}
              onSkip={() => {
                sessionStorage.setItem("aij.ready.skipped", "1");
                setReadySkipped(true);
              }}
            />
          ) : (
            <>
              {/* AJ-E611: the alert/notification region — polite
                  live-region, badge-carrying copy, reset-queue rows
                  with an explicit Resume click (the click opens the
                  picker; nothing ever launches by itself). */}
              <NotificationRegion onQueueResume={() => setNewSession(true)} />
              {/* AJ-E609: the Usage cost matrix — per-session model
                  attribution, the dual subscription view, and the CSV
                  export (rides above the inspector's content). */}
              <div className="max-h-64 overflow-y-auto">
                <CostMatrix />
              </div>
              <RightInspector
              selection={
                selection.kind === "session"
                  ? { kind: "instance", id: selection.id }
                  : selection.kind === "none"
                    ? { kind: "none" }
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
              terminalTitle={selectedSession ? `${selectedSession.account_label} · instance` : null}
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

      {recovery !== null && (
        <RecoverySummaryDialog
          summary={recovery}
          onDismiss={() => {
            setRecovery(null);
            void import("./lib/bridge").then((bridge) => bridge.recoverySummaryDismiss());
          }}
        />
      )}

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
        hotkey={goToHotkey}
        onHotkeyChange={async (combo) => {
          await goToHotkeySet(combo);
          setGoToHotkey(combo);
        }}
      />

      {wizard && (
        <OnboardingWizard
          key={wizardNonce}
          provider="claude-code"
          onClose={() => {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            setWizard(false);
          }}
          onAccountCreated={() => void refresh()}
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
            setTab("sessions");
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
            setTab("sessions");
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

/** The `?` legend popover button + panel (AJ-E403 Req.6). */
function LegendPopover() {
  const [open, setOpen] = useState(false);
  return (
    <span className="relative ml-auto shrink-0">
      <button
        type="button"
        aria-expanded={open}
        aria-label="Indicator legend"
        onClick={() => setOpen((value) => !value)}
        className="flex size-4 items-center justify-center rounded-full border border-hairline text-[9px] font-bold text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        ?
      </button>
      {open && (
        <span className="absolute bottom-full left-0 z-40 mb-2 block w-64 rounded-xl border border-hairline bg-surface p-3 shadow-card">
          <IndicatorLegend />
        </span>
      )}
    </span>
  );
}

function Overview({
  sessions,
  projects,
  onSelectSession,
  onInspectProvider,
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onSelectSession: (id: string) => void;
  onInspectProvider: (provider: string) => void;
}) {
  const [stats, setStats] = useState<StatsInfo | null>(null);
  useEffect(() => {
    void dashboardStats().then(setStats);
  }, []);
  void stats;
  const live = sessions.filter((s) => s.status === "running" || s.status === "starting");

  return (
    <div className="flex flex-col gap-6">
      {/* AJ-E501: the ledger-degraded banner (the Cleanup Centre stub's
          only surface until AJ-E506 builds the Centre). */}
      {stats?.ledger_degraded === true && (
        <div
          role="alert"
          data-testid="ledger-degraded-banner"
          className="flex items-center gap-2.5 rounded-lg border border-warning/25 bg-warning-soft px-4 py-3 text-[12.5px] text-ink"
        >
          <span aria-hidden>⚠</span>
          Process tracking degraded — cleanup is read-only until restart.
        </div>
      )}
      <div className="grid grid-cols-2 gap-4 xl:grid-cols-4">
        <StatCard
          label="Live instances"
          value={String(live.length)}
          hint={`${sessions.length} total recorded`}
          tone="live"
          icon={<IconTerminal width={15} height={15} />}
        />
        <StatCard
          label="Projects"
          value={String(projects.length)}
          hint="directories juggled"
          icon={<IconJuggler width={15} height={15} />}
        />
        <StatCard
          label="Machine"
          value="OK"
          hint="RAM · CPU · disk nominal (live gauges: M3)"
          tone="success"
          icon={<IconFleet width={15} height={15} />}
        />
        <StatCard
          label="Fleet"
          value="1"
          hint="this-mac only"
          icon={<IconFleet width={15} height={15} />}
        />
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <Card className="p-5 xl:col-span-2">
          <div className="mb-4 flex items-center justify-between">
            <div className="flex items-start gap-2">
              <div>
                <h2 className="text-sm font-semibold">Live instances</h2>
                <p className="text-xs text-muted">Select one to open its panel</p>
              </div>
              {/* Overview-header legend (AJ-E403 Req.6, surface 2 of 3). */}
              <LegendPopover />
            </div>
            <Chip tone="neutral" dot>
              {live.length} live
            </Chip>
          </div>
          {live.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-hairline py-10">
              <span className="flex size-10 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi">
                <IconTerminal width={19} height={19} />
              </span>
              <p className="text-sm text-muted">
                Nothing running — start one from the rail's New session.
              </p>
            </div>
          ) : (
            /* The card variant (AJ-E402 Req.4): identical I-01..I-11 to the
               rail row — no number exists here the rail cannot explain. */
            <div className="flex flex-col divide-y divide-hairline/50">
              {live.map((s) => (
                <SessionCard key={s.id} session={s} onSelect={onSelectSession} />
              ))}
            </div>
          )}
        </Card>

        <Card className="flex flex-col items-start justify-center gap-3 p-5">
          <h2 className="self-start text-sm font-semibold">Quota windows</h2>
          {/* Honest I-05 (AJ-E402 Req.3): the fabricated percent={0} rings
              are gone — each live Account renders its measured quota, and
              with no feed the exact `Quota: no data` empty state shows. */}
          {live.some((s) => s.quota !== null) ? (
            <div className="flex w-full flex-col gap-3 pt-1">
              {live
                .filter((s) => s.quota !== null)
                .map((s) => (
                  <button
                    key={s.id}
                    type="button"
                    onClick={() => onInspectProvider(s.provider)}
                    className="flex flex-col items-start gap-1 rounded-lg outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
                  >
                    <span className="text-[11px] text-muted hover:text-ink">
                      {s.account_label} · {s.provider}
                    </span>
                    <QuotaBar quota={s.quota} />
                  </button>
                ))}
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3 self-stretch py-2">
              <div className="flex items-center justify-around gap-4">
                <QuotaRing percent={null} label="5h" />
                <QuotaRing percent={null} label="week" />
              </div>
              <p className="text-center text-[11px] text-faint">
                {live.length === 0
                  ? "No live instances — quota shows per account once one runs"
                  : "Quota: no data — feeds land with the usage engine"}
              </p>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}

function ComingSoon({ screen }: { screen: string }) {
  return (
    <Card className="flex h-full flex-col items-center justify-center gap-2 py-16">
      <span className="text-sm font-medium">{screen}</span>
      <span className="flex items-center gap-1.5 text-xs text-muted">
        {screen === "Usage" ? (
          <IconUsage width={13} height={13} />
        ) : (
          <IconFleet width={13} height={13} />
        )}
        This screen lands in the M2+ build-out — see docs/plan/EPICS.md
      </span>
    </Card>
  );
}

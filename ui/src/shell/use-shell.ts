/**
 * AJ-E2018: the shell's state in one place: where the window is, the
 * working set, what's open over it, and the keys and outside requests
 * that move between them. App renders it; the registries (./screens,
 * ./pages, ./Overlays) read it as `Shell`.
 */
import { isActiveStatus } from "../components/indicator";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { cliAppId } from "../lib/app-name";
import { useAppKeys } from "../lib/app-keys";
import { OPEN_ACCOUNT } from "../lib/accounts-page";
import { OPEN_APP, useApps } from "../lib/apps";
import { accountName, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { useFloorLogos } from "../lib/floor-logos";
import { useThisMac } from "../lib/machine";
import { GLOBAL_NAV, focusPageSoon, globalId } from "../lib/nav";
import { setCentreOpen, toggleCentre } from "../lib/notices-store";
import { routeOpen, useOutsideRequests } from "../lib/outside";
import { showTasksHeldBy } from "../lib/tasks-holder";
import { useWindowTitle } from "../lib/window-title";
import { providerDisplayName } from "../lib/providers";
import { holdStop, sessionLabel } from "../lib/stop";
import { useWindowFolderDrop } from "../lib/use-folder-drop";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { useAppData } from "./use-app-data";
import { useDialogs, useRemoteMachines } from "./use-dialogs";
import { useNavigation } from "./use-navigation";
import { useNow, useRailDensity } from "./use-rail";
import { usePools } from "../lib/pools";
import { poolProject } from "../lib/new-session-choice";
import { offerFollows } from "../lib/new-session-sheet";

export type Shell = ReturnType<typeof useShell>;

export function useShell() {
  const rail = useRailDensity();
  const now = useNow();
  const nav = useNavigation();
  const { selection, global, setGlobal, setSelection, openPage, selectSession, inspectRemote } = nav;
  const data = useAppData(selection, openPage);
  const { sessions, projects, accounts, loginTerminals, refresh } = data;
  const dialogs = useDialogs();
  const { newSheet, setGoToOpen, setShortcutsOpen, setLimitFor, setSwitchBack } = dialogs;
  // A pool's page is titled by its name, as a project's is (the E28 walk);
  // its New session starts in its project (the rc27 walk).
  const pools = usePools();
  const hereDir = useMemo(() => {
    if (selection.kind !== "pool") return null;
    const id = poolProject(pools?.pools.find((pool) => pool.id === selection.id));
    return projects.find((project) => project.id === id)?.root ?? null;
  }, [selection, pools, projects]);
  /** New session from the header, the rail or ⌘N: on a pool's page, in
   * its project (the existing order's "the one it was opened for"). */
  const newSessionHere = useCallback(() => newSheet.show({ dir: hereDir }), [newSheet, hereDir]);

  // Inspector action events (AJ-E408): Add-account / sign-in cards and
  // the Start-first-session CTA route to the real surfaces.
  useEffect(() => {
    const openAccounts = () => setGlobal("accounts");
    const openScreen = (event: Event) => { setGlobal((current) => globalId((event as CustomEvent<{ id?: unknown }>).detail?.id) ?? current); focusPageSoon(); };
    // AJ-E2503/E2507: a notice's Continue elsewhere; your menu's shortcuts.
    const limit = (event: Event) => setLimitFor((event as CustomEvent<{ sessionId?: string }>).detail?.sessionId ?? null);
    const shortcuts = () => setShortcutsOpen(true);
    const page = (kind: "app" | "subscription") => (event: Event) => { const id = (event as CustomEvent<{ id?: string }>).detail?.id; if (id) { setGlobal(null); setSelection({ kind, id }); } };
    const on: [string, EventListener][] = [[OPEN_APP, page("app")], [OPEN_ACCOUNT, page("subscription")], ["aij:open-accounts", openAccounts], ["aij:open-screen", openScreen], ["aij:limit", limit], ["aij:switch-back", (event: Event) => setSwitchBack((event as CustomEvent<{ sessionId?: string }>).detail?.sessionId ?? null)], ["aij:open-shortcuts", shortcuts]];
    on.forEach(([name, run]) => window.addEventListener(name, run));
    return () => on.forEach(([name, run]) => window.removeEventListener(name, run));
    // State setters only: set up once.
  }, [setGlobal, setSelection, setLimitFor, setSwitchBack, setShortcutsOpen]);

  // D-96 F: "Start another" stands while the user reads the session it
  // started; working elsewhere ends it (the 0.4.1 persona walk).
  useEffect(() => offerFollows(global === null && selection.kind === "session" ? selection.id : null), [selection, global]);

  // A dropped folder → New session with it (an open New project sheet takes it).
  useWindowFolderDrop((path) => newSheet.show({ dir: path }), dialogs.newProjectDir !== null);

  const remote = useRemoteMachines(selection.kind === "remote");

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
    // D-62: a CLI App's page is its folder's (⌘K offers the folder's actions).
    () => (selection.kind === "app" && selection.id.startsWith("cli:") ? (accounts.find((a) => cliAppId(a.id) === selection.id) ?? null) : null),
    [selection, accounts],
  );
  const selectedProvider = useMemo(() => {
    if (selection.kind !== "provider") return null;
    const providerAccounts = accounts.filter((a) => a.provider === selection.id);
    return { id: selection.id, name: providerDisplayName(selection.id), accounts: providerAccounts };
  }, [selection, accounts]);

  // AJ-E1503 Req.2: Stop is held 5 s with Undo, then sent. The ref keeps
  // ⌘. on the session selected now (the key listener is set up once).
  const stopSession = useCallback(
    (session: InstanceInfo) => holdStop({ id: session.id, label: sessionLabel(session, projects) }, () => void refresh()),
    [projects, refresh],
  );
  const stopSelectedRef = useRef<() => void>(() => {});
  stopSelectedRef.current = () => {
    if (selectedSession && isActiveStatus(selectedSession.status)) {
      stopSession(selectedSession);
    }
  };

  // AJ-E1504: App's shortcuts (lib/app-keys.ts, from the one registry).
  useAppKeys({
    openShortcuts: () => setShortcutsOpen(true),
    togglePalette: () => setGoToOpen((openNow) => !openNow),
    toggleNotifications: toggleCentre,
    newSession: newSessionHere, newProject: dialogs.newProject,
    toggleRail: rail.toggleRail,
    toggleInspector: dialogs.toggleInspector,
    cycleDensity: rail.cycleDensity,
    goHome: nav.openHome,
    back: nav.back,
    forward: nav.forward,
    goTo: setGlobal,
    openSettings: nav.openSettings,
    onCleanup: () => global === "cleanup",
    focusCleanupFilter: () => document.querySelector<HTMLElement>('[data-testid="process-search"]')?.focus(),
    stopSelected: () => stopSelectedRef.current(),
  });

  // AJ-E414, AJ-E1510: the go-to hotkey, and a clicked notification.
  useFloorLogos(); // AJ-E3508: the Floor's logos, drawn once each (only after the Floor opens)
  useOutsideRequests({ hotkey: dialogs.setGoToHotkey, goTo: () => setGoToOpen(true), open: (open) => routeOpen(open, { remote: inspectRemote, session: selectSession, centre: () => setCentreOpen(true), page: setGlobal, app: (id) => openPage({ kind: "app", id }), account: (id) => openPage({ kind: "subscription", id }), tasks: (appId) => (showTasksHeldBy(`app:${appId}`), setGlobal("tasks")), conversation: (accountId, id) => openPage({ kind: "conversation", accountId, id, projectId: null }), pool: (id) => openPage({ kind: "pool", id }), settings: (section) => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section } })) }) });

  // E14: This Mac — one subscription for Home, the rail and the header.
  const thisMacInfo = useThisMac(sessions);

  // An App's and an Account's page by theirs (the E32 walk).
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const accountRow = selection.kind === "subscription" ? subscriptions?.find((row) => row.account_id === selection.id) : undefined;
  const contextTitle = titleOf({ global, selection, remoteLabel: selection.kind === "remote" ? remote.remoteMachines?.find((m) => m.machine_id === selection.machineId)?.label : undefined, selectedProject, selectedSession, selectedLogin, projects, pools: pools?.pools ?? [], apps: apps ?? [], accountRow });
  useWindowTitle(contextTitle); // AJ-E2803 R4: "AI Juggler · Projects"

  return {
    ...rail, now, ...nav, ...data, ...dialogs, ...remote,
    selectedSession, selectedProject, selectedLogin, selectedAccount, selectedProvider,
    stopSession, thisMacInfo, contextTitle, newSessionHere,
  };
}

/** A pool's page title: its name, a project's own pool by its project
 * (as its card says it). */
function poolTitle(pool: { kind: string; name: string; project_id: string | null } | undefined, projects: { id: string; name: string }[]): string {
  if (!pool) return "Pool";
  return pool.kind === "default" ? (projects.find((project) => project.id === pool.project_id)?.name ?? pool.name) : pool.name;
}

/** The window's title after "AI Juggler · ": the screen, or the page's own
 * name (a project, a pool, an App, an Account), else what it's about. */
function titleOf(t: {
  global: string | null;
  selection: ReturnType<typeof useNavigation>["selection"];
  remoteLabel: string | undefined;
  selectedProject: { name: string } | null;
  selectedSession: InstanceInfo | null;
  selectedLogin: { title: string } | null;
  projects: { id: string; name: string }[];
  pools: { id: string; kind: string; name: string; project_id: string | null }[];
  apps: { id: string; name: string }[];
  accountRow: SubscriptionInfo | undefined;
}): string {
  const { selection, projects } = t;
  if (t.global !== null) return GLOBAL_NAV.find((g) => g.id === t.global)?.label ?? "";
  switch (selection.kind) {
    case "remote":
      return `Fleet · ${t.remoteLabel ?? "machine"}`;
    case "project":
      return t.selectedProject?.name ?? "Project";
    case "provider":
      return providerDisplayName(selection.id);
    case "app":
      return t.apps.find((app) => app.id === selection.id)?.name ?? "App";
    case "subscription":
      return t.accountRow ? accountName(t.accountRow) : "Account";
    default:
      break;
  }
  // D-25: a session by its project (the view below names its place).
  if (t.selectedSession) return projects.find((p) => p.id === t.selectedSession!.project_id)?.name ?? "Session";
  if (t.selectedLogin) return t.selectedLogin.title;
  if (selection.kind === "none") return "Home";
  if (selection.kind === "pool") return poolTitle(t.pools.find((pool) => pool.id === selection.id), projects);
  // AJ-E1534: a session outside AI Juggler, like one of its own, by its project.
  return projects.find((p) => selection.kind === "conversation" && p.id === selection.projectId)?.name ?? "Session";
}

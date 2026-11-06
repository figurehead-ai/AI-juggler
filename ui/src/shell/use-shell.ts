/**
 * AJ-E2018: the shell's state in one place: where the window is, the
 * working set, what's open over it, and the keys and outside requests
 * that move between them. App renders it; the registries (./screens,
 * ./pages, ./Overlays) read it as `Shell`.
 */
import { useCallback, useEffect, useMemo, useRef } from "react";
import { cliAppId } from "../lib/app-name";
import { useAppKeys } from "../lib/app-keys";
import { OPEN_ACCOUNT } from "../lib/accounts-page";
import { OPEN_APP } from "../lib/apps";
import { useFloorLogos } from "../lib/floor-logos";
import { useThisMac } from "../lib/machine";
import { GLOBAL_NAV, focusPageSoon, globalId } from "../lib/nav";
import { setCentreOpen, toggleCentre } from "../lib/notices-store";
import { routeOpen, useOutsideRequests } from "../lib/outside";
import { providerDisplayName } from "../lib/providers";
import { holdStop, sessionLabel } from "../lib/stop";
import { useWindowFolderDrop } from "../lib/use-folder-drop";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { useAppData } from "./use-app-data";
import { useDialogs, useRemoteMachines } from "./use-dialogs";
import { useNavigation } from "./use-navigation";
import { useNow, useRailDensity } from "./use-rail";

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
    if (selectedSession && (selectedSession.status === "running" || selectedSession.status === "starting")) {
      stopSession(selectedSession);
    }
  };

  // AJ-E1504: App's shortcuts (lib/app-keys.ts, from the one registry).
  useAppKeys({
    openShortcuts: () => setShortcutsOpen(true),
    togglePalette: () => setGoToOpen((openNow) => !openNow),
    toggleNotifications: toggleCentre,
    newSession: () => newSheet.show(), newProject: dialogs.newProject,
    toggleRail: rail.toggleRail,
    toggleInspector: dialogs.toggleInspector,
    cycleDensity: rail.cycleDensity,
    goHome: nav.openHome,
    goTo: setGlobal,
    openSettings: nav.openSettings,
    onCleanup: () => global === "cleanup",
    focusCleanupFilter: () => document.querySelector<HTMLElement>('[data-testid="process-search"]')?.focus(),
    stopSelected: () => stopSelectedRef.current(),
  });

  // AJ-E414, AJ-E1510: the go-to hotkey, and a clicked notification.
  useFloorLogos(); // AJ-E3508: the Floor's logos, drawn once each (only after the Floor opens)
  useOutsideRequests({ hotkey: dialogs.setGoToHotkey, goTo: () => setGoToOpen(true), open: (open) => routeOpen(open, { remote: inspectRemote, session: selectSession, centre: () => setCentreOpen(true), page: setGlobal, app: (id) => openPage({ kind: "app", id }), conversation: (accountId, id) => openPage({ kind: "conversation", accountId, id, projectId: null }), pool: (id) => openPage({ kind: "pool", id }), settings: (section) => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section } })) }) });

  // E14: This Mac — one subscription for Home, the rail and the header.
  const thisMacInfo = useThisMac(sessions);

  const contextTitle =
    global !== null
      ? (GLOBAL_NAV.find((g) => g.id === global)?.label ?? "")
      : selection.kind === "remote"
        ? `Fleet · ${remote.remoteMachines?.find((m) => m.machine_id === selection.machineId)?.label ?? "machine"}`
        : selection.kind === "project"
        ? (selectedProject?.name ?? "Project")
        : selection.kind === "provider"
        ? providerDisplayName(selection.id)
        : selection.kind === "app" || selection.kind === "subscription"
        ? (selection.kind === "app" ? "App" : "Account")
        : selectedSession
          ? // D-25: the session's project (the view below names its place).
            (projects.find((p) => p.id === selectedSession.project_id)?.name ?? "Session")
          : selectedLogin
            ? selectedLogin.title
            : selection.kind === "none"
              ? "Home"
              : // AJ-E1534: a session outside AI Juggler, like one of its own, by its project.
                (selection.kind === "pool" ? "Pool" : (projects.find((p) => selection.kind === "conversation" && p.id === selection.projectId)?.name ?? "Session"));

  return {
    ...rail, now, ...nav, ...data, ...dialogs, ...remote,
    selectedSession, selectedProject, selectedLogin, selectedAccount, selectedProvider,
    stopSession, thisMacInfo, contextTitle,
  };
}

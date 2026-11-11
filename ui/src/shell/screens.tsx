/**
 * AJ-E2018 — the shell's registry of global screens (ADR-0019 §3): each
 * header door is one entry here, rendered in the main column in place of
 * the work area. A new screen is one entry and its own folder.
 */
import type { JSX } from "react";
import { cliAppId, cliAppName } from "../lib/app-name";
import { GLOBAL_NAV, type GlobalId } from "../lib/nav";
import { ScreenBoundary, ScrollingScreen } from "../components/ScreenBoundary";
import { ProfilesScreen } from "../screens/Profiles";
import { FleetScreen, History, NavScreen, ProcessesScreen, SettingsScreen, ThisMacPage, UsageScreen } from "../lazyParts";
import type { Shell } from "./use-shell";

/** A global screen: what the main column shows while it is open. */
export type ScreenEntry = (shell: Shell) => JSX.Element | null;

/** 0.3: every project, pool and task, and the Floor's list. */
const navScreen = (id: "projects" | "pools" | "tasks" | "floor"): ScreenEntry => (shell) => (
  <ScrollingScreen key={id} screen={id} label={GLOBAL_NAV.find((entry) => entry.id === id)!.label}>
    <NavScreen id={id} projects={shell.shownProjects} sessions={shell.sessions} accounts={shell.accounts} loading={!shell.loaded} now={shell.now} onNewProject={shell.newProject} />
  </ScrollingScreen>
);

export const SCREENS: Partial<Record<GlobalId, ScreenEntry>> = {
  projects: navScreen("projects"),
  pools: navScreen("pools"),
  tasks: navScreen("tasks"),
  floor: navScreen("floor"),
  cleanup: (shell) => (
    <ScrollingScreen key="cleanup" screen="cleanup" label="Processes">
      <ProcessesScreen
        onOpenSession={shell.selectSession}
        onOpenConversation={(accountId, id) => shell.openPage({ kind: "conversation", accountId, id, projectId: null })}
        accounts={shell.accounts.map((account) => ({ id: account.id, label: cliAppName(account.provider, account.label) }))}
        projects={shell.projects.map((project) => ({ id: project.id, label: project.name }))}
      />
    </ScrollingScreen>
  ),
  accounts: (shell) => (
    <ScrollingScreen key="accounts" screen="accounts" label="Accounts">
      <ProfilesScreen
        key={shell.accountsNonce}
        knownAccounts={shell.loaded ? shell.accounts : null}
        knownProfiles={shell.loaded ? shell.profiles : null}
        sessions={shell.sessions}
        onAddAccounts={() => shell.startAddAccount()}
        // D-97 (3): the empty page's Find my apps is first run's scan.
        onFindApps={() => shell.setFirstRun(true)}
        onAccountsChanged={shell.accountsChanged}
        openTerminal={shell.openLoginTerminal}
        onInspectAccount={(id) => shell.openPage({ kind: "app", id: cliAppId(id) })}
      />
    </ScrollingScreen>
  ),
  history: (shell) => (
    <ScrollingScreen key="history" screen="history" label="History" subject={shell.historyQuery}>
      <History key={shell.historyQuery} initialQuery={shell.historyQuery} sessions={shell.sessions} projects={shell.projects} accounts={shell.accounts} onChanged={() => void shell.refresh()} />
    </ScrollingScreen>
  ),
  usage: (shell) => (
    <ScrollingScreen key="usage" screen="usage" label="Usage">
      <UsageScreen accounts={shell.accounts} sessions={shell.sessions} projects={shell.projects} onAddAccounts={() => shell.setFirstRun(true)} onOpenHome={shell.openHome} />
    </ScrollingScreen>
  ),
  fleet: (shell) => (
    <ScrollingScreen key="fleet" screen="fleet" label="Fleet">
      <FleetScreen
        onOpenCloud={shell.openCloudSettings}
        onInspectRemote={shell.inspectRemote}
        onOpenHome={shell.accounts.length === 0 && shell.loaded ? shell.openHome : undefined}
      />
    </ScrollingScreen>
  ),
  // AJ-E3104: a page with its own tabs, which scroll themselves.
  thismac: (shell) => (
    <ScreenBoundary key="thismac" screen="this-mac-page" label="This Mac">
      <ThisMacPage sessions={shell.railSessions} projects={shell.projects} accounts={shell.accounts} now={shell.now} onOpenSession={shell.selectSession} onOpenProject={shell.selectProject} onOpenCleanup={() => shell.setGlobal("cleanup")} />
    </ScreenBoundary>
  ),
  // Settings › Privacy, a frame from now (lib/nav's redirect).
  privacy: () => null,
  settings: (shell) => (
    <ScrollingScreen key="settings" screen="settings" label="Settings" subject={shell.settingsSection ?? ""}>
      <SettingsScreen key={shell.settingsSection ?? "top"} focusCloud={shell.settingsFocusCloud} focusSection={shell.settingsSection} />
    </ScrollingScreen>
  ),
};

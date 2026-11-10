/**
 * AJ-E2018 — what the shell opens over the work area: the port warning,
 * the prompts that wait for their moment, the palette, first run and Add
 * account, the limit and switch flows, and the New session and New
 * project sheets.
 */
import type { JSX } from "react";
import { CrashReportPrompt } from "../components/CrashReports";
import { FleetPromptDialog } from "../components/FleetPromptDialog";
import { IdentityMismatchDialog } from "../components/IdentityMismatchDialog";
import { IsolationOfferDialog } from "../components/IsolationOfferDialog";
import { paletteSelection } from "../components/PaletteActions";
import { PoolSheetHost } from "../components/PoolSheetHost";
import { QuitSheet } from "../components/QuitSheet";
import { RecoverySummaryDialog } from "../components/RecoverySummaryDialog";
import { ReportBugHost } from "../components/ReportBugSheet";
import { RestartAppHost } from "../components/RestartApp";
import { ToastHost } from "../components/Toast";
import { UsageStatsReconsent } from "../components/UsageStatsReconsent";
import { desktopFocus } from "../lib/ipc/desktop";
import { fleetRemoteFocus } from "../lib/ipc/fleet-remote";
import { goToHotkeySet } from "../lib/ipc/settings";
import { funnelEvent } from "../lib/funnel";
import { lazyNamed } from "../lib/lazy";
import { setCentreOpen } from "../lib/notices-store";
import { AddProviderSheet, FirstRun, LimitDialog, NewProjectSheet, NewSessionModal, PortWarningSheet, ShortcutSheet, SwitchBackFlow, SwitchFlow } from "../lazyParts";
import type { Shell } from "./use-shell";

const GoToPalette = lazyNamed(() => import("../components/GoToPalette"), "GoToPalette", true); // AJ-E2901: out of the first bundle, fetched first after paint (⌘K is the key used soonest)

export function ShellOverlays({ shell }: { shell: Shell }): JSX.Element {
  const { portWarning, recovery, sessions, projects, accounts, refresh, selectSession, inspectRemote, switchFlow, limitFor, newSheet, selection } = shell;
  return (
    <>
      {portWarning && (
        <div className="pointer-events-auto absolute inset-x-0 top-3 z-40 mx-auto w-fit max-w-[560px]">
          <PortWarningSheet
            warning={portWarning.warning}
            onFreePort={() => shell.setPortWarning(null)}
            onLaunchAnyway={() => {
              const proceed = portWarning.proceed;
              shell.setPortWarning(null);
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
            shell.setRecovery(null);
            void import("../lib/ipc/cleanup").then((bridge) => bridge.recoverySummaryDismiss());
          }}
        />
      )}

      {/* AJ-E709: "Ask each time" — a paired machine waits for this Mac's user. */}
      <FleetPromptDialog />
      <IdentityMismatchDialog />
      <IsolationOfferDialog />
      <GoToPalette
        open={shell.goToOpen}
        onClose={() => shell.setGoToOpen(false)}
        sessions={sessions}
        onFocusWindow={(instance) => {
          void desktopFocus(instance.id).then(() => {
            void refresh();
            selectSession(instance.id);
          });
        }}
        onOpenSession={selectSession}
        remoteMachines={shell.remoteMachines ?? []}
        onFocusRemote={(machineId, instanceId) => {
          void fleetRemoteFocus(machineId, instanceId).then((outcome) => {
            shell.setRemoteFocusOutcome({ machineId, instanceId, outcome });
            inspectRemote(machineId, instanceId);
          });
        }}
        onOpenRemote={inspectRemote}
        hotkey={shell.goToHotkey}
        onHotkeyChange={async (combo) => (await goToHotkeySet(combo), shell.setGoToHotkey(combo))}
        selected={paletteSelection(shell.selectedSession, shell.selectedProject, shell.selectedAccount, shell.selectedProvider?.id ?? null)}
        selectedConversation={selection.kind === "conversation" ? { accountId: selection.accountId, id: selection.id } : null}
        app={{
          openScreen: (id) => (id === "home" ? shell.openHome() : id === "settings" ? shell.openSettings() : shell.setGlobal(id)),
          newSession: () => newSheet.show(), newProject: shell.newProject, notifications: () => setCentreOpen(true),
          shortcuts: () => shell.setShortcutsOpen(true), toggleRail: shell.toggleRail, searchHistory: (words) => (shell.setHistoryQuery(words), shell.setGlobal("history")), fleet: shell.pairedCount > 0,
        }}
        projects={projects} accounts={accounts}
        onChanged={() => void refresh()}
      />

      {shell.firstRun && (
        <FirstRun
          resumeAtAccess={shell.firstRunAtAccess}
          openLoginTerminal={shell.openLoginTerminal}
          onDone={() => {
            localStorage.setItem("aij.onboarding.dismissed", "1");
            shell.setFirstRunAtAccess(false);
            shell.setFirstRun(false);
            void refresh();
          }}
          onAccountsChanged={shell.accountsChanged}
        />
      )}

      {shell.chooseTool && (
        <AddProviderSheet
          onClose={() => shell.setChooseTool(false)}
          onChoose={(provider) => {
            shell.setChooseTool(false);
            shell.setAddAccounts(provider);
          }}
          onServiceAdded={() => (shell.setChooseTool(false), shell.setAccountsNonce((n) => n + 1), shell.accountsChanged())}
        />
      )}

      {shell.addAccounts !== null && (
        <FirstRun
          mode="add"
          provider={shell.addAccounts}
          startAt={shell.addFresh ? "fresh" : "found"}
          openLoginTerminal={shell.openLoginTerminal}
          onDone={() => {
            shell.setAddAccounts(null);
            shell.setAccountsNonce((n) => n + 1);
            shell.accountsChanged();
          }}
          onAccountsChanged={shell.accountsChanged}
        />
      )}

      {limitFor && sessions.some((s) => s.id === limitFor) && (
        <LimitDialog
          session={sessions.find((s) => s.id === limitFor)!}
          sessions={sessions}
          now={shell.now}
          onClose={() => shell.setLimitFor(null)}
          onParked={() => void refresh()}
          onChooseAnother={() => {
            shell.setLimitFor(null);
            selectSession(limitFor);
          }}
          onContinue={(target) => {
            shell.setLimitFor(null);
            shell.setSwitchFlow({ sessionId: limitFor, targetProfileId: target.recipe_id, targetLabel: target.account_label });
          }}
        />
      )}
      {switchFlow && (
        <SwitchFlow
          sessionId={switchFlow.sessionId}
          targetProfileId={switchFlow.targetProfileId}
          targetLabel={switchFlow.targetLabel}
          onClose={() => shell.setSwitchFlow(null)}
          onSwitched={(sessionId, undo) => {
            shell.setSwitchFlow(null);
            void refresh();
            selectSession(sessionId);
            // AJ-E206: a completed 1-click switch leaves the 10s undo
            // window open (the graph keeps the edge as the handle).
            if (undo) {
              const targetLabel = switchFlow.targetLabel;
              void import("../components/SwitchFlow").then((module) =>
                module.showSwitchUndoToast({
                  edgeId: undo.edgeId,
                  targetLabel,
                  onUndone: (undoneId) => {
                    void refresh();
                    selectSession(undoneId);
                  },
                }),
              );
            }
          }}
        />
      )}
      {shell.switchBack && (
        <SwitchBackFlow
          sessionId={shell.switchBack}
          onClose={() => shell.setSwitchBack(null)}
          onSwitched={(sessionId) => {
            shell.setSwitchBack(null);
            void refresh();
            selectSession(sessionId);
          }}
        />
      )}
      {/* AJ-E1503: the one feedback system, and the quit warning. */}
      <ToastHost />
      <RestartAppHost />
      {shell.shortcutsOpen && <ShortcutSheet fleet={shell.pairedCount > 0} onClose={() => shell.setShortcutsOpen(false)} />}
      <QuitSheet />
      <PoolSheetHost sessions={shell.railSessions} projects={projects} />
      <ReportBugHost />
      {newSheet.open && (
        <NewSessionModal
          projects={shell.shownProjects}
          accounts={accounts}
          sessions={sessions}
          initialDirectory={newSheet.dir}
          initialAccount={newSheet.account} recipes={shell.profiles}
          openLoginTerminal={shell.openLoginTerminal}
          onProjectsChanged={() => void refresh()}
          onClose={newSheet.close}
          onLaunched={(sessionId) => {
            // AJ-E309 Req.6: once per install (the backend dedupes and
            // computes seconds_since_install itself).
            funnelEvent("first_launch");
            newSheet.close();
            void refresh();
            selectSession(sessionId);
          }}
          onManageLaunchSettings={() => {
            newSheet.close();
            shell.setGlobal("accounts");
          }}
        />
      )}

      {/* AJ-E305 flow (b): New project — 2 steps, ≤ 3 clicks, no
          typed paths; an empty preset means "no directory yet". */}
      {shell.newProjectDir !== null && (
        <NewProjectSheet
          projects={shell.shownProjects}
          accounts={accounts}
          initialDirectory={shell.newProjectDir === "" ? null : shell.newProjectDir}
          onClose={() => shell.setNewProjectDir(null)}
          onLaunched={(sessionId) => {
            funnelEvent("first_launch");
            shell.setNewProjectDir(null);
            void refresh();
            selectSession(sessionId);
          }}
          openLoginTerminal={shell.openLoginTerminal}
          onAccountsChanged={shell.accountsChanged}
          onManageLaunchSettings={() => {
            shell.setNewProjectDir(null);
            shell.setGlobal("accounts");
          }}
        />
      )}
    </>
  );
}

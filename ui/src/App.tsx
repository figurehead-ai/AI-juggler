import { sayPainted } from "./lib/ipc/app-lifecycle";
import { useEffect } from "react";
import { AppActions } from "./components/AppActions";
import { AppHeader } from "./components/AppHeader";
import { LegendPopover } from "./components/LegendPopover";
import { MachineStatusLine } from "./components/MachineStatusLine";
import { NoticeOpeners } from "./components/NoticeOpeners";
import { OfflineBanner } from "./components/OfflineBanner";
import { RailHeader } from "./components/RailHeader";
import { SkipLink } from "./components/SkipLink";
import { WorkflowRail } from "./components/WorkflowRail";
import { AttentionAnnouncer, RenewalAnnouncer } from "./components/indicator";
import { cliAppId } from "./lib/app-name";
import { ConversationsProvider } from "./lib/conversations";
import { prefetchLazy } from "./lib/lazy";
import { ThisMacContext } from "./lib/machine";
import { PRIMARY_NAV } from "./lib/nav";
import { useScrollMemory } from "./lib/scroll-memory";
import { ShellOverlays } from "./shell/Overlays";
import { workArea } from "./shell/pages";
import { SCREENS } from "./shell/screens";
import { useShell } from "./shell/use-shell";

/**
 * App shell (ADR-0007, design iteration 5): a workflow navigator rail
 * (Projects / Instances / Sessions — the list lives in the rail) plus the
 * detail panel where the selected item loads, session terminals embedded.
 * Global oversight (Overview, Accounts, Usage, Fleet) sits top-right.
 *
 * AJ-E2018: the state lives in `shell/use-shell` and its hooks; what the
 * main column shows comes from the registries (`shell/screens` for the
 * header's screens, `shell/pages` for an item's page), and what opens over
 * it from `shell/Overlays`.
 */
export default function App() {
  // The rest of the app, fetched once the first window has painted.
  useEffect(() => {
    prefetchLazy();
    sayPainted();
  }, []);
  const shell = useShell();
  const { selection, global, setGlobal, openPage, sessions, accounts, projects, loaded, refresh, newSheet, thisMacInfo, info } = shell;
  // 0.3.4: Home opens where it was left.
  const homeScroll = useScrollMemory("home");
  const screen = global !== null ? SCREENS[global] : undefined;

  return (
    <ThisMacContext.Provider value={thisMacInfo}>
    <ConversationsProvider want={selection.kind === "conversation" ? { accountId: selection.accountId, id: selection.id } : null}>
    <AppActions
      selectConversation={(conversation) => openPage({ kind: "conversation", accountId: conversation.account_id, id: conversation.id, projectId: conversation.project_id })}
      selectSession={shell.selectSession}
      selectProject={shell.selectProject}
      selectAccount={(id) => openPage({ kind: "app", id: cliAppId(id) })} // D-62: a folder is its CLI App
      selectTool={(id) => { openPage({ kind: "provider", id }); shell.setInspectorFocus((n) => n + 1); }} openToolSettings={shell.openToolInSettings} // AJ-E3104: its page; focus follows
      selectPool={(id) => openPage({ kind: "pool", id })}
      openAccounts={() => setGlobal("accounts")} openPools={() => setGlobal("pools")} openHome={shell.openHome}
      stopSession={shell.stopSession}
      newSessionIn={(project) => newSheet.show({ dir: project.root })}
      refresh={() => void refresh()}
    >
    <div className="relative flex h-screen w-screen overflow-hidden">
      <SkipLink />
      <AttentionAnnouncer sessions={sessions} />
      {/* AJ-E2502: the menu-bar item's asks (Needs you is the backend's, D-49). */}
      <NoticeOpeners ready={loaded} sessions={sessions} accounts={accounts} projects={projects} thisMac={thisMacInfo} />
      <RenewalAnnouncer accounts={accounts} />
      <WorkflowRail
        density={shell.railDensity}
        onCycleDensity={shell.cycleDensity}
        projects={shell.shownProjects}
        sessions={shell.railSessions}
        onChanged={() => void refresh()}
        accounts={accounts}
        loginTerminals={shell.loginTerminals}
        selectedId={selection.kind === "none" || selection.kind === "remote" ? null : selection.id}
        onSelectSession={(id) =>
          shell.loginTerminals.some((t) => t.id === id) ? shell.selectLogin(id) : shell.selectSession(id)
        }
        onSelectProject={shell.selectProject}
        onNewSession={shell.newSessionHere}
        onNewProject={shell.newProject}
        freshness={shell.freshness}
        now={shell.now}
        remoteMachines={shell.remoteMachines ?? []}
        selectedRemote={
          selection.kind === "remote" ? { machineId: selection.machineId, instanceId: selection.instanceId } : null
        }
        onSelectRemote={shell.inspectRemote}
        loading={!loaded}
        onShowList={shell.toggleRail}
        header={(minimal) => <RailHeader minimal={minimal} info={info} onToggle={shell.toggleRail} />}
        footer={(minimal) =>
          minimal ? (
            <MachineStatusLine minimal onOpen={() => setGlobal("thismac")} />
          ) : (
            <div className="flex flex-col gap-1.5 px-3 py-2.5">
              <div className="flex min-w-0 items-center gap-2 overflow-hidden">
                {/* The rail-footer `?` legend popover (AJ-E403 Req.6,
                    surface 1 of 3 — identical content everywhere). */}
                <LegendPopover />
              </div>
              <div className="truncate font-mono text-label text-muted">
                v{info?.version ?? "0.0.0"}
              </div>
            </div>
          )
        }
      />

      {/* Main column */}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Glass status bar: work context left, global oversight right */}
        <AppHeader
          title={shell.contextTitle}
          global={global}
          page={selection.kind}
          pairedCount={shell.pairedCount}
          cleanupCount={shell.cleanupCount}
          sessions={sessions}
          onOpenThisMac={() => setGlobal("thismac")}
          onNewSession={shell.newSessionHere}
          back={{ canBack: shell.canBack, canForward: shell.canForward, onBack: shell.back, onForward: shell.forward }}
          onPick={(id) => {
            if (PRIMARY_NAV.some((item) => item.id === id)) shell.clearSettingsFocus();
            if (id === "home") shell.openHome();
            else setGlobal(id);
          }}
        />

        {/* Content: detail panel (work mode) or global screen. tabIndex
         * -1: programmatic focus target when overlays (AJ-E510
         * recovery summary) dismiss and return focus to the board. */}
        <main tabIndex={-1} className="flex min-h-0 flex-1 flex-col overflow-hidden p-5 outline-none">
          <OfflineBanner onRetry={() => void refresh()} />
          <div className="min-h-0 flex-1">
            {screen ? screen(shell) : workArea(shell, { homeScroll })}
          </div>
        </main>
      </div>

      <ShellOverlays shell={shell} />
    </div>
    </AppActions>
    </ConversationsProvider>
    </ThisMacContext.Provider>
  );
}

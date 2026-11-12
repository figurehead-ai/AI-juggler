/**
 * AJ-E2018 — the shell's registry of item pages: what the work area shows
 * for the selection (Home, a session outside AI Juggler, an App or an
 * Account, a pool, and the inspector for the rest), under the
 * notification band. A sign-in's terminal and a paired Mac's machine have
 * their own pages, without the band.
 */
import { processRunning } from "../lib/session-process";
import { useStopHeld } from "../lib/stop";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { JSX } from "react";
import { Button, Card } from "../components/primitives";
import { NotificationRegion } from "../components/NotificationRegion";
import { RightInspector, type InspectorSelection } from "../components/RightInspector";
import { ScreenBoundary } from "../components/ScreenBoundary";
import { launchSession } from "../lib/ipc/launch";
import { launchPortWarning } from "../lib/ipc/kill";
import { sessionHandover } from "../lib/ipc/switch-preflight";
import { EntityPage } from "../pages/EntityPage";
import { HomeScreen } from "../screens/Home";
import { ConversationPage, PoolPage, SessionPanel } from "../lazyParts";
import type { Selection } from "./selection";
import type { Shell } from "./use-shell";
import { CLEAN_UP_VIEW } from "../lib/landing";

/** Home's scroll position, kept between visits (lib/scroll-memory). */
type Extra = { homeScroll: (element: HTMLDivElement | null) => (() => void) | undefined };

/** An item's page in the work area (null for a selection of another kind). */
type PageEntry = (shell: Shell, selection: Selection, extra: Extra) => JSX.Element | null;

const PAGES: Partial<Record<Selection["kind"], PageEntry>> = {
  none: (shell, _selection, { homeScroll }) => (
    <div ref={homeScroll} className="relative min-h-0 flex-1 overflow-y-auto" data-testid="home-scroll">
      <ScreenBoundary key="home" screen="home" label="Home">
        <HomeScreen
          loading={!shell.loaded}
          sessions={shell.sessions}
          accounts={shell.accounts}
          projects={shell.projects}
          now={shell.now}
          onOpenSession={shell.selectSession}
          onNewSession={() => shell.newSheet.show()}
          // No account yet: "Find my accounts" comes back to first
          // run (every tool at once), as its Not now promises.
          onAddAccounts={() => shell.setFirstRun(true)}
          onFreshAccount={() => shell.startAddAccount(true)}
          onLimit={shell.setLimitFor}
          openLoginTerminal={shell.openLoginTerminal}
          onAccountsChanged={shell.accountsChanged}
          onChanged={() => void shell.refresh()}
          thisMac={shell.thisMacInfo}
          onOpenCleanup={() => shell.openOn("cleanup", CLEAN_UP_VIEW)}
          onOpenThisMac={() => shell.setGlobal("thismac")}
        />
      </ScreenBoundary>
    </div>
  ),
  conversation: (shell, selection) => {
    if (selection.kind !== "conversation") return null;
    return (
      <div className="min-h-0 flex-1" data-testid="conversation-scroll">
        <ConversationPage accountId={selection.accountId} id={selection.id} now={shell.now} projects={shell.projects} onSwitch={(sessionId, targetProfileId, targetLabel) => shell.setSwitchFlow({ sessionId, targetProfileId, targetLabel })} />
      </div>
    );
  },
  app: entityPage,
  subscription: entityPage,
  // The band above takes its height: the page shrinks to fit, never runs past the window (0.3.1).
  pool: (shell, selection) =>
    selection.kind !== "pool" ? null : (
      <div className="min-h-0 flex-1" data-testid="pool-scroll">
        <PoolPage poolId={selection.id} sessions={shell.railSessions} projects={shell.projects} accounts={shell.accounts} now={shell.now} />
      </div>
    ),
};

function entityPage(shell: Shell, selection: Selection): JSX.Element | null {
  if (selection.kind !== "app" && selection.kind !== "subscription") return null;
  return <EntityPage kind={selection.kind} id={selection.id} sessions={shell.railSessions} projects={shell.projects} accounts={shell.accounts} now={shell.now} onOpenSession={shell.selectSession} onOpenProject={shell.selectProject} onChanged={() => void shell.refresh()} />;
}

/** The work area: a sign-in's terminal, a paired Mac, or the band and
 * the selection's page. */
/** The session inspector's Stop: there while its process runs, whatever
 * it says (the rc20 walk: a session waiting at Claude's folder-trust
 * question, "Asks you", had no Stop). */
export function InspectorStop({ session, onStop }: { session: InstanceInfo; onStop: (session: InstanceInfo) => void }): JSX.Element | null {
  const held = useStopHeld(session.id);
  if (!processRunning(session)) return null; // one that ended at its limit has nothing to stop
  return (
    <Button size="sm" variant="secondary" disabled={held} onClick={() => onStop(session)} data-testid="inspector-stop">
      {held ? "Stopping…" : "Stop"}
    </Button>
  );
}

export function workArea(shell: Shell, extra: Extra): JSX.Element {
  const { selection } = shell;
  if (shell.selectedLogin) {
    return (
      <SessionPanel
        session={null}
        project={null}
        loginTerminal={shell.selectedLogin}
        profiles={shell.profiles}
        accounts={shell.accounts}
        onOpenProject={shell.selectProject}
        onStopped={() => void shell.refresh()}
        onHandover={async () => {}}
      />
    );
  }
  if (selection.kind === "remote") return remotePage(shell, selection);
  const page = PAGES[selection.kind];
  return (
    // The region takes its own space above the screen and pushes
    // it down — never over it, at any width (the 0.0.12 report).
    <div className="flex h-full min-h-0 flex-col">
      {/* AJ-E611: the alerts (polite, plain words), reset-queue rows
          with an explicit Resume click (nothing launches by itself). */}
      <div className="shrink-0" data-testid="notification-band">
        <NotificationRegion onOpenRemote={(machineId) => shell.inspectRemote(machineId, null)} />
      </div>
      {page ? page(shell, selection, extra) : inspectorPage(shell)}
    </div>
  );
}

/** AJ-E705: a paired Mac, or a session on it. */
function remotePage(shell: Shell, selection: Extract<Selection, { kind: "remote" }>): JSX.Element {
  const { remoteMachines } = shell;
  const machine = remoteMachines?.find((entry) => entry.machine_id === selection.machineId) ?? null;
  if (!machine) {
    return (
      <Card className="flex flex-col items-start gap-2 p-5 text-xs" data-testid="remote-missing">
        <p>{remoteMachines === null ? "Loading…" : "That machine is no longer paired with this Mac."}</p>
        <Button size="sm" variant="secondary" onClick={() => shell.setGlobal("fleet")}>
          Back to Fleet
        </Button>
      </Card>
    );
  }
  const instance = selection.instanceId
    ? (machine.instances.find((row) => row.id === selection.instanceId) ?? null)
    : null;
  const outcome = shell.remoteFocusOutcome;
  return (
    <ScreenBoundary key="inspector" screen="inspector" label="This view">
      <RightInspector
        folded={shell.inspectorFolded}
        onToggleFold={shell.toggleInspector}
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
              outcome &&
              outcome.machineId === machine.machine_id &&
              outcome.instanceId === instance?.id
                ? outcome.outcome
                : null,
          },
        }}
        sessions={shell.sessions}
        projects={shell.projects}
        accounts={shell.accounts}
        profiles={shell.profiles}
        terminalTitle={null}
        freshness={shell.freshness}
        now={shell.remoteNow}
        onOpenProject={shell.selectProject}
        onOpenSession={shell.selectSession}
        onLaunch={() => {}}
        onStopped={() => void shell.refresh()}
        onHandover={() => {}}
        onNew={() => shell.newSheet.show()}
        onRemoteBack={() => shell.setGlobal("fleet")}
        onRemoteSelect={(instanceId) => shell.inspectRemote(machine.machine_id, instanceId)}
        onRemoteActionDone={() => shell.setRemoteNow(Date.now())}
      />
    </ScreenBoundary>
  );
}

/** A session, project, tool or sign-in: the inspector's page. */
function inspectorPage(shell: Shell): JSX.Element {
  const { selection, selectedSession, selectedProvider } = shell;
  return (
    <div className="min-h-0 flex-1">
      <RightInspector
        folded={shell.inspectorFolded}
        onToggleFold={shell.toggleInspector}
        focusTabsRequest={shell.inspectorFocus}
        selection={inspectorSelection(selection)}
        data={{
          instance: selectedSession,
          project: shell.selectedProject,
          account: shell.selectedAccount,
          provider: selectedProvider ? { id: selectedProvider.id, name: selectedProvider.name } : null,
          machineId: "this-mac",
        }}
        sessions={shell.sessions}
        projects={shell.projects}
        accounts={shell.accounts}
        profiles={shell.profiles}
        terminalTitle={selectedSession ? `${selectedSession.account_label} · session` : null}
        freshness={shell.freshness}
        now={shell.now}
        primaryAction={selectedSession ? <InspectorStop session={selectedSession} onStop={shell.stopSession} /> : null}
        onOpenProject={shell.selectProject}
        onOpenSession={shell.selectSession}
        onLaunch={async (recipe, project) => {
          const doLaunch = async () => {
            const result = await launchSession(recipe.id, project.root);
            if (result) {
              void shell.refresh();
              shell.selectSession(result.session_id);
            }
          };
          // AJ-E505 Req.4: warn BEFORE spawn when the project's
          // learned dev port is held by a managed orphan; the
          // launch itself is never blocked (SPEC §5.2 advisor).
          try {
            const warning = await launchPortWarning(project.id ?? null);
            if (warning) {
              shell.setPortWarning({ warning, proceed: () => void doLaunch() });
              return;
            }
          } catch {
            // The pre-check is advisory: probe failures launch.
          }
          await doLaunch();
        }}
        onStopped={() => void shell.refresh()}
        switchBackSuggestions={shell.switchBackCards}
        onSwitchBack={(sessionId) => shell.setSwitchBack(sessionId)}
        onHandover={(sessionId, targetProfileId, targetLabel) => {
          void sessionHandover; // superseded by the E205 flow
          shell.setSwitchFlow({ sessionId, targetProfileId, targetLabel });
        }}
        onNew={() => shell.newSheet.show()}
      />
    </div>
  );
}

/** The inspector's own words for the selection (a session is an instance). */
function inspectorSelection(selection: Selection): InspectorSelection {
  switch (selection.kind) {
    case "session":
      return { kind: "instance", id: selection.id };
    case "project":
    case "login":
    case "provider":
      return selection;
    default:
      return { kind: "none" };
  }
}

/**
 * D-62 (AJ-E3204) — an App's page: one app in one of its profiles ("Claude
 * app · b-work", "Claude Code · Work Max"), with only its own sessions.
 * Its header says what it is and whether it runs, with the one thing to
 * do there: Go to it or Start (a desktop App), New session (a CLI App).
 * Its tabs: Overview (open now, last used, its account's room and who
 * shares it, where it works), Sessions (all of them, paged), Projects,
 * Launch settings (a CLI App's own sets, AJ-E3204 R2), Usage (where its
 * folder is known), Processes (while something's open in it) and Settings (its name, Start and Keep alive, its folder's
 * settings).
 *
 * Its sessions are the rail's under this App (lib/app-items.ts), so a
 * page and the rail never disagree.
 */
import { ProfileRoomChip } from "../../components/RoomChip";
import { useMemo, type JSX } from "react";
import { AppIcon } from "../../components/AppIcon";
import { AppKindBadge } from "../../components/AppKindBadge";
import { ProfileBadge } from "../../components/ProfileBadge";
import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { useActionHost } from "../../components/ItemMenu";
import { PageShell } from "../PageShell";
import { PageHeader } from "../PageHeader";
import { pageTabs } from "../registry";
import { RelatedList } from "../../components/RelatedList";
import type { Teach } from "../../components/related/RelatedEmpty";
import { AccountUsage } from "../AccountPage";
import { AppOverview, AppProjects, rowOf } from "./AppOverview";
import { AppSettings } from "./AppSettings";
import { AppLaunchSettings } from "./AppLaunchSettings";
import { ProcessesView } from "../../screens/processes/ProcessesView";
import { FolderMenu } from "../../components/InspectorHeaderMenu";
import { errorText, hasLiveBackend, signedIn } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { useRelationCounts } from "../../lib/relation-counts";
import { useLaunchSettings } from "./useLaunchSettings";
import { RelatedPools, usePagePools } from "../../components/RelatedPools";
import { AccountDoor, Dot, MacDoor, SessionToolChip, SharedRoom } from "../session/HeaderDoors";
import { useApps, type AppModel } from "../../lib/apps";
import { signInUnchecked, useSubscriptions } from "../../lib/subscriptions";
import { accountForApp } from "./AppTerms";
import { appItems, openHere, useWorkedProjects } from "../../lib/app-items";
import { placesOpen } from "../../lib/ipc/places";
import { placeWords } from "../../lib/places";
import { appByConversation, appByFolder } from "../../lib/rail-apps";
import { itemsOf } from "../../lib/rail-items";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";
import { askRestart, restartCounts } from "../../components/RestartApp";
import { showGoTo } from "../../lib/go-to";

export function AppPage({
  id,
  sessions,
  projects,
  accounts,
  now,
  onOpenSession,
  onOpenProject,
  onChanged,
}: {
  id: string;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
  /** Its folder changed (renamed, signed in, removed). */
  onChanged?: () => void;
}): JSX.Element {
  const apps = useApps(accounts);
  const app = apps?.find((one) => one.id === id) ?? null;
  if (apps === null) {
    return (
      <p className="p-6 text-meta text-ink-2" data-testid="app-page-loading">
        Reading its sessions…
      </p>
    );
  }
  if (!app) {
    return (
      <p className="p-6 text-meta text-ink-2" data-testid="app-page-gone">
        AI Juggler doesn't know this app now: its folder was removed, or it was forgotten. It comes back the next time it runs.
      </p>
    );
  }
  return <AppBody app={app} apps={apps} sessions={sessions} projects={projects} accounts={accounts} now={now} onOpenSession={onOpenSession} onOpenProject={onOpenProject} onChanged={onChanged ?? (() => {})} />;
}

function AppBody({
  app,
  apps,
  sessions,
  projects,
  accounts,
  now,
  onOpenSession,
  onOpenProject,
  onChanged,
}: {
  onChanged: () => void;
  app: AppModel;
  apps: AppModel[];
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
}): JSX.Element {
  const host = useActionHost();
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const places = useMemo(() => apps.flatMap((one) => (one.place ? [one.place] : [])), [apps]);
  const items = useMemo(() => {
    const all = itemsOf({ sessions, conversations, projects, seen });
    return appItems(all, app.railKey, appByConversation(places), appByFolder(places));
  }, [sessions, conversations, projects, seen, app.railKey, places]);
  const worked = useWorkedProjects("app", app.id, items, projects);
  const openIds = items.filter(openHere).flatMap((item) => item.session?.id ?? item.conversation?.id ?? []);
  const open = openIds.length;
  const restart = restartCounts(items.filter(openHere));
  // Its Account's other Apps, by the Account each resolves to (its stored
  // one, else its folder's), so a fresh App counts too (AJ-E3204 R1).
  const subs = useSubscriptions() ?? [];
  const accountOf = (one: AppModel) => accountForApp(subs, one.folder?.id ?? null, one.accountId)?.account_id ?? null;
  const mine = accountOf(app);
  const sharedWith = mine ? apps.filter((other) => other.id !== app.id && accountOf(other) === mine) : [];
  // AJ-E3102: Sessions counts what its list pages through (the backend's
  // relation counts); Projects, the projects its list shows.
  const related = useRelationCounts("app", app.id);
  const sessionsCount = hasLiveBackend() ? related?.sessions.all : items.length;
  // AJ-E3104: the pools its sessions are in.
  const own = useMemo(() => items.flatMap((item) => (item.session ? [item.session] : [])), [items]);
  const pools = usePagePools("app", app.id, own);
  // AJ-E3104: a CLI App's launch settings, counted from the tab's own read.
  const cli = app.kind === "cli" && app.folder !== null;
  const launch = useLaunchSettings(cli ? (app.folder?.id ?? null) : null);
  const tabs = useMemo(
    () => pageTabs("app", { hasUsage: app.folder !== null, live: open > 0, cli, counts: { sessions: sessionsCount, projects: worked.length, pools: pools?.length, launch: launch.recipes?.length } }),
    [app.folder, cli, sessionsCount, worked.length, open, pools?.length, launch.recipes?.length],
  );
  return (
    <div className="mx-auto flex h-full w-full max-w-[1440px] flex-col" data-testid="app-page" data-app-id={app.id}>
      <PageShell
        kind="app"
        subject={app.id}
        tabs={tabs}
        header={
          <PageHeader
            kind="app"
            mark={
              <span className="flex items-center gap-1.5">
                <AppIcon appId={app.appId} appPid={app.place?.app_pid ?? null} name={app.app} size={32} />
                {app.profile && <ProfileBadge app={app.slot} name={app.profile} />}
              </span>
            }
            name={app.name}
            // 0.4.1: its Account out of room or low, unmistakably.
            chips={
              <>
                {/* D-100: Desktop or CLI, by icon and word. */}
                <AppKindBadge kind={app.kind} variant="chip" />
                <ProfileRoomChip profileId={app.folder?.id} size="md" testId="app-room-chip" />
              </>
            }
            facts={<AppFacts app={app} now={now} sharedWith={sharedWith.map((other) => other.name)} unchecked={signInUnchecked(subs, app.folder?.id)} />}
            actions={
              <>
                <AppAction app={app} restart={restart} />
                {app.kind === "cli" && app.folder && <FolderMenu account={app.folder} sessions={sessions} onChanged={onChanged} />}
              </>
            }
          />
        }
        render={(tab, visible) => {
          switch (tab) {
            case "overview":
              return <AppOverview app={app} items={items} projects={worked} now={now} host={host} onOpenSession={onOpenSession} onOpenProject={onOpenProject} />;
            case "sessions":
              return (
                <RelatedList
                  listKey={`app-sessions:${app.id}`}
                  anchor={{ app_id: app.id }}
                  local={() => items.flatMap((item) => rowOf(item, Date.now(), onOpenSession, host) ?? [])}
                  folders={accounts}
                  projects={projects}
                  showProjects
                  inApp
                  teach={appTeach(app)}
                  onOpenSession={onOpenSession}
                  searchLabel={`Search ${app.name}'s sessions`}
                />
              );
            case "projects":
              return <AppProjects projects={worked} sessions={items.length} now={now} onOpenProject={onOpenProject} />;
            case "pools":
              return (
                <RelatedPools
                  pools={pools}
                  teach={{
                    title: `No pools hold ${app.name}'s sessions yet`,
                    body: "A session AI Juggler starts here joins its project's own pool and shows here; ones the app started itself aren't in pools.",
                  }}
                />
              );
            case "usage":
              return app.folder && visible ? <AccountUsage account={app.folder} accounts={accounts} /> : null;
            case "processes":
              // Its open sessions' processes, each whole (AJ-E2404's view).
              return visible ? <ProcessesView sessionIds={openIds} height="calc(100vh - 22rem)" testId="app-processes" /> : null;
            case "launch":
              return app.folder ? <AppLaunchSettings folder={app.folder} sessions={sessions} recipes={launch.recipes} reload={launch.reload} /> : null;
            case "settings":
              return <AppSettings app={app} now={now} resetsAtMs={soonestReset(sessions, app.folder?.id ?? null)} />;
            default:
              return null;
          }
        }}
      />
    </div>
  );
}

/** When the folder's newest limit resets: the reading of its session with
 * the least left (the folder page's rule). */
function soonestReset(sessions: InstanceInfo[], folder: string | null): number | null {
  const read = sessions.filter((session) => session.account_id === folder && session.quota !== null);
  read.sort((a, b) => a.quota!.used_percentage - b.quota!.used_percentage);
  return read.at(-1)?.quota?.resets_at_ms ?? null;
}

/** Whether it runs and where, then its Account (a door, AJ-E3104). A
 * desktop App starts at its state: its app is its title's first word, and
 * its sessions are counted on the Sessions tab and in Open now, not here
 * again (the 0.4 review, 47). A CLI App says it runs in a terminal. */
export function appFactParts(app: Pick<AppModel, "kind" | "tool" | "app" | "place">, now: number): string[] {
  return [app.kind === "cli" ? (app.tool ? "runs in a terminal" : `${app.app} in a terminal`) : null, app.place ? placeWords(app.place, now) : null].filter((part): part is string => part !== null);
}

function AppFacts({ app, now, sharedWith, unchecked }: { app: AppModel; now: number; sharedWith: readonly string[]; unchecked: string | null }): JSX.Element {
  const parts = appFactParts(app, now);
  return (
    <>
      {app.kind === "cli" && app.tool && <SessionToolChip tool={app.tool} />}
      {parts.map((part, index) => (
        <span key={part} className="flex items-center gap-2">
          {index > 0 && <span aria-hidden="true">·</span>}
          <span>{part}</span>
        </span>
      ))}
      <AccountDoor folderId={app.folder?.id ?? null} accountId={app.accountId} lead={<Dot />} />
      {/* 0.4.1: couldn't tell this time; it stays in its Account meanwhile. */}
      {unchecked && (
        <span className="flex items-center gap-2">
          <Dot />
          <span className="text-warning" title="It stays in its account until a check says otherwise. AI Juggler checks again on its own." data-testid="app-sign-in-unchecked">
            {unchecked}
          </span>
        </span>
      )}
      <SharedRoom names={sharedWith} />
      <MacDoor lead={<Dot />} />
    </>
  );
}

/** The one thing to do there: a desktop App's Go to it or Start, a CLI
 * App's New session. */
function AppAction({ app, restart }: { app: AppModel; restart: { open: number; working: number } }): JSX.Element | null {
  if (app.kind === "cli") {
    // Signed out, its Overview's notice is the one way on (sign in).
    if (!app.folder || !signedIn(app.folder.login_state)) return null;
    const folder = app.folder.id;
    return (
      <Button size="md" variant="primary" onClick={() => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } }))} data-testid="app-new-session">
        New session here
      </Button>
    );
  }
  const place = app.place;
  if (!place) return null;
  const running = place.app_pid !== null;
  const go = () => {
    showToast({ id: "go-to", message: running ? `Bringing ${app.name} forward…` : `Starting ${app.name}…` });
    // 0.3.8: one that lost its window offers Restart in its toast.
    placesOpen(place.id).then(showGoTo, (cause) => showToast({ id: "go-to", message: errorText(cause) }));
  };
  return (
    <>
      {/* D-71: a running one restarts after its confirm. */}
      {running && (
        <Button size="md" variant="secondary" onClick={() => askRestart({ placeId: place.id, name: app.name, ...restart })} data-testid="app-restart">
          Restart…
        </Button>
      )}
      <Button
        size="md"
        variant="primary"
        onClick={go}
        disabled={!running && place.cannot_start !== null}
        title={running ? undefined : (place.cannot_start ?? undefined)}
        data-testid="app-go"
      >
        {running ? "Go to it" : "Start"}
      </Button>
    </>
  );
}

/** E31 (AJ-E3103 Req.3): an App with no session yet says what would show
 * and how it comes: a CLI App's sessions start here or in a terminal on
 * its folder; a desktop App's are the ones its app lists (0.4.3), so an
 * empty one is empty, never "not seen yet" (0.4.3 persona D6). */
function appTeach(app: AppModel): Teach {
  if (app.kind === "cli" && app.folder) {
    const folder = app.folder.id;
    return {
      title: `No sessions in ${app.name} yet`,
      body: "Sessions you start in this App, here or in a terminal, show up here, what needs you first.",
      action: { label: "New session", onClick: () => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } })) },
    };
  }
  return { title: `No sessions in ${app.name} yet`, body: `${app.app} lists none for it. Sessions you start there show up here, what needs you first.` };
}

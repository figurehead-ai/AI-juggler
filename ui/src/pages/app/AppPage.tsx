/**
 * D-62 (AJ-E3204) — an App's page: one app in one of its profiles ("Claude
 * app · b-work", "Claude Code · Work Max"), with only its own sessions.
 * Its header says what it is and whether it runs, with the one thing to
 * do there: Go to it or Start (a desktop App), New session (a CLI App).
 * Its tabs: Overview (open now, last used, its account's room and who
 * shares it, where it works), Sessions (all of them, paged), Projects,
 * Usage (where its folder is known), Processes (while something's open
 * in it) and Settings (its name, Start and Keep alive, its folder's
 * settings).
 *
 * Its sessions are the rail's under this App (lib/app-items.ts), so a
 * page and the rail never disagree.
 */
import { useMemo, type JSX } from "react";
import { AppIcon } from "../../components/AppIcon";
import { ProfileBadge } from "../../components/ProfileBadge";
import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { useActionHost } from "../../components/ItemMenu";
import { PageHeader, PageShell } from "../PageShell";
import { pageTabs } from "../registry";
import { PagedSessions } from "../SessionsList";
import { AccountUsage } from "../AccountPage";
import { AppOverview, AppProjects, rowOf } from "./AppOverview";
import { AppSettings } from "./AppSettings";
import { ProcessesView } from "../../screens/processes/ProcessesView";
import { errorText, type AccountInfo, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { useApps, type AppModel } from "../../lib/apps";
import { appItems, appProjects, openHere } from "../../lib/app-items";
import { placesOpen } from "../../lib/ipc/places";
import { sessionsQuery } from "../../lib/ipc/sessions";
import { placeWords } from "../../lib/places";
import { appByConversation } from "../../lib/rail-apps";
import { itemsOf } from "../../lib/rail-items";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";

export function AppPage({
  id,
  sessions,
  projects,
  accounts,
  now,
  onOpenSession,
  onOpenProject,
}: {
  id: string;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
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
  return <AppBody app={app} apps={apps} sessions={sessions} projects={projects} accounts={accounts} now={now} onOpenSession={onOpenSession} onOpenProject={onOpenProject} />;
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
}: {
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
    return appItems(all, app.railKey, appByConversation(places));
  }, [sessions, conversations, projects, seen, app.railKey, places]);
  const worked = useMemo(() => appProjects(items), [items]);
  const openIds = items.filter(openHere).flatMap((item) => item.session?.id ?? item.conversation?.id ?? []);
  const open = openIds.length;
  const sharedWith = app.accountId ? apps.filter((other) => other.id !== app.id && other.accountId === app.accountId) : [];
  const tabs = useMemo(
    () => pageTabs("app", { hasUsage: app.folder !== null, live: open > 0, counts: { sessions: items.length, projects: worked.length } }),
    [app.folder, items.length, worked.length, open],
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
                {app.profile && <ProfileBadge name={app.profile} />}
              </span>
            }
            name={app.name}
            facts={<AppFacts app={app} open={open} total={items.length} now={now} />}
            actions={<AppAction app={app} />}
          />
        }
        render={(tab, visible) => {
          switch (tab) {
            case "overview":
              return <AppOverview app={app} items={items} projects={worked} sharedWith={sharedWith} now={now} host={host} onOpenSession={onOpenSession} onOpenProject={onOpenProject} />;
            case "sessions":
              return (
                <PagedSessions
                  listKey={`app-sessions:${app.id}`}
                  query={sessionsQuery({ app_id: app.id })}
                  local={() => items.flatMap((item) => rowOf(item, Date.now(), onOpenSession, host) ?? [])}
                  empty={`No sessions in ${app.name} yet.`}
                  projects={projects}
                  onOpenSession={onOpenSession}
                  inApp
                />
              );
            case "projects":
              return <AppProjects projects={worked} sessions={items.length} now={now} onOpenProject={onOpenProject} />;
            case "usage":
              return app.folder && visible ? <AccountUsage account={app.folder} accounts={accounts} /> : null;
            case "processes":
              // Its open sessions' processes, each whole (AJ-E2404's view).
              return visible ? <ProcessesView sessionIds={openIds} height="calc(100vh - 22rem)" testId="app-processes" /> : null;
            case "settings":
              return <AppSettings app={app} now={now} />;
            default:
              return null;
          }
        }}
      />
    </div>
  );
}

/** What it is and whether it runs: its app, its state, its sessions. */
function AppFacts({ app, open, total, now }: { app: AppModel; open: number; total: number; now: number }): JSX.Element {
  const parts = [
    app.kind === "cli" ? `${app.app} in a terminal` : app.app,
    app.place ? placeWords(app.place, now) : null,
    total === 0 ? "No sessions yet" : `${total === 1 ? "1 session" : `${total} sessions`}${open > 0 ? ` · ${open} open` : ""}`,
  ].filter((part): part is string => part !== null);
  return (
    <>
      {parts.map((part, index) => (
        <span key={part} className="flex items-center gap-2">
          {index > 0 && <span aria-hidden="true">·</span>}
          <span>{part}</span>
        </span>
      ))}
    </>
  );
}

/** The one thing to do there: a desktop App's Go to it or Start, a CLI
 * App's New session. */
function AppAction({ app }: { app: AppModel }): JSX.Element | null {
  if (app.kind === "cli") {
    if (!app.folder) return null;
    const folder = app.folder.id;
    return (
      <Button size="md" variant="primary" onClick={() => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } }))} data-testid="app-new-session">
        New session
      </Button>
    );
  }
  const place = app.place;
  if (!place) return null;
  const running = place.app_pid !== null;
  const go = () => {
    showToast({ id: "go-to", message: running ? `Bringing ${app.name} forward…` : `Starting ${app.name}…` });
    placesOpen(place.id).then(
      (outcome) => showToast({ id: "go-to", message: outcome.outcome }),
      (cause) => showToast({ id: "go-to", message: errorText(cause) }),
    );
  };
  return (
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
  );
}

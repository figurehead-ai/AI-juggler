/**
 * AJ-E3104 — This Mac's page: its state and what's using it (the
 * Overview, AJ-E1528), then everything on it, each one click away and
 * counted as the other pages count (relations-map §4): its sessions (the
 * one list), its Apps, the projects they worked in and the pools they're
 * in. Every App and every local session is this Mac's; the counts come
 * from the same relation cache as the lists, keyed by this Mac's id.
 */
import { useContext, useEffect, useMemo, useState, type JSX } from "react";
import { IconComputer } from "../../components/icons-actions";
import { useActionHost } from "../../components/ItemMenu";
import { RelatedList } from "../../components/RelatedList";
import { RelatedPools, usePagePools } from "../../components/RelatedPools";
import { macApps } from "../../lib/account-items";
import { appProjects, openHere } from "../../lib/app-items";
import { appByRailKey, useApps, type AppModel } from "../../lib/apps";
import { fleetSelf, hasLiveBackend, type AccountInfo, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { machineStatus, ThisMacContext } from "../../lib/machine";
import { THIS_MACHINE, THIS_MACHINE_IN_TEXT } from "../../lib/machine-words";
import { appByConversation, appKeyOf } from "../../lib/rail-apps";
import { itemsOf, type RailItem } from "../../lib/rail-items";
import { placeOf } from "../../lib/project-words";
import { useRelationCounts } from "../../lib/relation-counts";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";
import { THIS_MAC_SUBTITLE, ThisMacOverview } from "../../screens/ThisMacScreen";
import { AppProjects, rowOf } from "../app/AppOverview";
import { PageHeader, PageShell } from "../PageShell";
import { pageTabs } from "../registry";
import { AccountAppRows } from "../subscription/AccountApps";

type Props = {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
  onOpenCleanup?: () => void;
};

/** This Mac's id and name, as Fleet knows it (the id the sessions and
 * the relation counts carry); `null` until read. */
function useThisMacSelf(): { id: string; label: string } | null {
  const [self, setSelf] = useState<{ id: string; label: string } | null>(null);
  useEffect(() => {
    let alive = true;
    fleetSelf().then(
      (info) => alive && setSelf({ id: info.machine_id, label: info.label }),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, []);
  return self;
}

export function ThisMacPage({ sessions, projects, accounts, now, onOpenSession, onOpenProject, onOpenCleanup }: Props): JSX.Element {
  // A session's name on screen: its place (branch, else folder).
  const nameOf = (id: string) => {
    const session = sessions.find((s) => s.id === id);
    return session ? placeOf(session, projects).title : "A session";
  };
  const host = useActionHost();
  const self = useThisMacSelf();
  const status = machineStatus(useContext(ThisMacContext));
  const apps = useApps(accounts);
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const items = useMemo(() => itemsOf({ sessions, conversations, projects, seen }), [sessions, conversations, projects, seen]);
  const byConversation = useMemo(() => appByConversation((apps ?? []).flatMap((app) => (app.place ? [app.place] : []))), [apps]);
  const rows = useMemo(() => macApps(apps ?? [], items, byConversation), [apps, items, byConversation]);
  const worked = useMemo(() => appProjects(items), [items]);
  const own = useMemo(() => items.flatMap((item) => (item.session ? [item.session] : [])), [items]);
  const id = self?.id ?? null;
  const pools = usePagePools(id ? "machine" : null, id ?? "", own);
  // Sessions and projects count what their lists page through (the
  // backend's relation counts); the design preview counts what it shows.
  const related = useRelationCounts(id ? "machine" : null, id);
  const live = hasLiveBackend();
  const counts = {
    sessions: live ? related?.sessions.all : items.length,
    apps: apps ? rows.length : undefined,
    projects: live ? related?.projects : worked.length,
    pools: pools?.length,
  };
  const tabs = pageTabs("machine", { local: true, counts });
  const appOf = (item: RailItem): AppModel | null => (apps ? appByRailKey(apps, appKeyOf(item, byConversation)) : null);
  const open = items.filter(openHere).length;
  const facts = [
    status ? <span key="state" className={status.tone}>{status.words}</span> : null,
    apps ? <span key="apps">{rows.length === 1 ? "1 app" : `${rows.length} apps`}</span> : null,
    open > 0 ? <span key="open">{open} open</span> : null,
  ].filter((fact): fact is JSX.Element => fact !== null);
  return (
    <div className="mx-auto flex h-full min-h-0 w-full max-w-[1440px] flex-col" data-testid="this-mac-page">
      <PageShell
        kind="machine"
        subject="this-mac"
        tabs={tabs}
        header={
          <PageHeader
            kind="machine"
            mark={
              <span className="flex size-8 items-center justify-center rounded-lg border border-hairline bg-surface-2 text-muted">
                <IconComputer width={18} height={18} aria-hidden="true" />
              </span>
            }
            name={THIS_MACHINE}
            facts={facts.map((fact, index) => (
              <span key={fact.key} className="flex items-center gap-2">
                {index > 0 && <span aria-hidden="true">·</span>}
                {fact}
              </span>
            ))}
          />
        }
        render={(tab) => {
          switch (tab) {
            case "overview":
              return (
                <div className="flex flex-col gap-6" data-testid="this-mac-overview">
                  <p className="max-w-prose text-body text-ink-2">{THIS_MAC_SUBTITLE}</p>
                  <ThisMacOverview nameOf={nameOf} onOpenSession={onOpenSession} onOpenCleanup={onOpenCleanup} />
                </div>
              );
            case "sessions":
              return (
                <RelatedList
                  listKey="mac-sessions:this"
                  anchor={id ? { machine_id: id } : {}}
                  local={() => items.flatMap((item) => rowOf(item, Date.now(), onOpenSession, host, appOf(item)) ?? [])}
                  folders={accounts}
                  projects={projects}
                  showProjects
                  teach={{ title: `No sessions on ${THIS_MACHINE_IN_TEXT} yet`, body: "Every session here shows up in this list, what needs you first: AI Juggler's and the ones your apps started." }}
                  onOpenSession={onOpenSession}
                  searchLabel={`Search ${THIS_MACHINE_IN_TEXT}'s sessions`}
                />
              );
            case "apps":
              return (
                <div className="w-full max-w-content rounded-xl border border-hairline bg-surface px-2 py-2" data-testid="this-mac-apps-tab">
                  <AccountAppRows apps={rows} now={now} empty="No apps here yet. Add one with Add app…" />
                </div>
              );
            case "projects":
              return <AppProjects projects={worked} sessions={items.length} now={now} onOpenProject={onOpenProject} />;
            case "pools":
              return (
                <RelatedPools
                  pools={pools}
                  teach={{ title: `No pools hold ${THIS_MACHINE_IN_TEXT}'s sessions yet`, body: "A session AI Juggler starts joins its project's own pool and shows here; ones an app started itself aren't in pools." }}
                />
              );
            default:
              return null;
          }
        }}
      />
    </div>
  );
}

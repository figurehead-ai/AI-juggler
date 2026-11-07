/**
 * D-62 (AJ-E3205) — an Account's page: one subscription, however many
 * Apps use it. Its limits come first (the account is where limits live),
 * then its Apps, what's open on it and where it works. Tabs: Overview,
 * Apps, Sessions (paged, on any of its folders), Projects, Usage (all of
 * its folders together) and Settings (its name, renewal and fee, set
 * once). No email or id on the page beyond its line (D-33).
 */
import { ProfileRoomChip } from "../../components/RoomChip";
import { useEffect, useMemo, useState, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { IconFolder, IconPlay } from "../../components/usage/controls";
import { IconHistory } from "../../components/icons-places";
import { IconProfiles } from "../../components/icons";
import { Button } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { accountColor } from "../../lib/marks";
import { UsageDashboard } from "../../components/usage/Dashboard";
import { Segmented } from "../../components/usage/controls";
import { useRange } from "../../components/usage/useUsage";
import { PageHeader, PageShell, showTab } from "../PageShell";
import { pageTabs, rememberTab } from "../registry";
import { RelatedList } from "../../components/RelatedList";
import { AccountLimits } from "../AccountPage";
import { AccountQuality } from "../account/Quality";
import { AppProjects, MoreLink, ProjectRows, RowList, rowOf, type Shown } from "../app/AppOverview";
import { AccountAppRows } from "./AccountApps";
import { AccountSettings } from "./AccountSettings";
import { ChooseChip } from "./TermChoice";
import { hasLiveBackend } from "../../lib/bridge";
import { quotaDisplayAll, type QuotaDisplayData } from "../../lib/ipc/quota";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { RelatedPools, usePagePools } from "../../components/RelatedPools";
import { useRelationCounts } from "../../lib/relation-counts";
import { accountApps, accountItems } from "../../lib/account-items";
import { openHere, useWorkedProjects, type appProjects } from "../../lib/app-items";
import { appByRailKey, openApp, useApps, type AppModel } from "../../lib/apps";
import { cliAppId } from "../../lib/app-name";
import { appByConversation, appByFolder, appKeyOf } from "../../lib/rail-apps";
import { itemsOf, type RailItem } from "../../lib/rail-items";
import { renewalLabel, renewalRowVisible } from "../../lib/renewal";
import { RenewalChip } from "../../components/RenewalChip";
import { RANGES } from "../../lib/usage/range";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";
import { accountDisplay, accountLine, accountName, staleSubscriptions, useSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";

/** Apps the Overview lists before "All apps". */
const APPS_SHOWN = 5;

type PageProps = {
  id: string;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
};

export function SubscriptionPage(props: PageProps): JSX.Element {
  const [version, setVersion] = useState(0);
  const all = useSubscriptions(version);
  const sub = all?.find((row) => row.account_id === props.id) ?? null;
  if (all === null) {
    return (
      <p className="p-6 text-meta text-ink-2" data-testid="account-page-loading">
        Reading its sign-ins…
      </p>
    );
  }
  if (!sub) {
    return (
      <p className="p-6 text-meta text-ink-2" data-testid="account-page-gone">
        AI Juggler doesn't know this account now: no app is signed in to it here. It comes back when one is.
      </p>
    );
  }
  const changed = () => {
    staleSubscriptions();
    setVersion((was) => was + 1);
  };
  return <AccountBody {...props} sub={sub} onChanged={changed} />;
}

function AccountBody({ sub, onChanged, sessions, projects, accounts, now, onOpenSession, onOpenProject }: PageProps & { sub: SubscriptionInfo; onChanged: () => void }): JSX.Element {
  const host = useActionHost();
  const apps = useApps(accounts);
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const folderIds = useMemo(() => new Set(sub.profiles.map((profile) => profile.id)), [sub.profiles]);
  const allItems = useMemo(() => itemsOf({ sessions, conversations, projects, seen }), [sessions, conversations, projects, seen]);
  const items = useMemo(() => accountItems(allItems, folderIds), [allItems, folderIds]);
  const knownPlaces = useMemo(() => (apps ?? []).flatMap((app) => (app.place ? [app.place] : [])), [apps]);
  const byConversation = useMemo(() => appByConversation(knownPlaces), [knownPlaces]);
  const byFolder = useMemo(() => appByFolder(knownPlaces), [knownPlaces]);
  const onIt = useMemo(() => accountApps(apps ?? [], sub.account_id ?? "", folderIds, allItems, byConversation), [apps, sub.account_id, folderIds, allItems, byConversation]);
  const worked = useWorkedProjects("subscription", sub.account_id, items, projects);
  // Each session's App, by the rail's rule (a session that ended in the
  // Claude app says so, not its tool).
  const appOf = (item: RailItem): AppModel | null => (apps ? appByRailKey(apps, appKeyOf(item, byConversation, byFolder)) : null);
  const name = accountName(sub);
  const open = items.filter(openHere).length;
  // AJ-E3102: Sessions counts what its list pages through (the backend's
  // relation counts); Apps and Projects, what their lists show.
  const related = useRelationCounts("subscription", sub.account_id);
  const sessionsCount = hasLiveBackend() ? related?.sessions.all : items.length;
  // AJ-E3104: the pools its sessions are in.
  const own = useMemo(() => items.flatMap((item) => (item.session ? [item.session] : [])), [items]);
  const pools = usePagePools("subscription", sub.account_id ?? "", own);
  const tabs = useMemo(
    () => pageTabs("subscription", { counts: { apps: onIt.length, sessions: sessionsCount, projects: worked.length, pools: pools?.length } }),
    [onIt.length, sessionsCount, worked.length, pools?.length],
  );
  const renewal = renewalLabel(sub.renewal_date, now);
  const facts = [
    accountLine(sub),
    // AJ-E411: a renewal 7 days off or fewer is its chip, not words here.
    renewal && !renewalRowVisible(renewal) ? renewal.text : null,
    sub.fee_usd != null ? `$${sub.fee_usd}/month` : null,
    onIt.length === 1 ? "1 app" : `${onIt.length} apps`,
    open > 0 ? `${open} open` : null,
  ].filter((part): part is string => !!part);
  const folder = sub.profiles[0]?.id ?? null;
  return (
    <div className="mx-auto flex h-full w-full max-w-[1440px] flex-col" data-testid="account-page" data-account-id={sub.account_id}>
      <PageShell
        kind="subscription"
        subject={sub.account_id ?? sub.key}
        tabs={tabs}
        header={
          <PageHeader
            kind="subscription"
            mark={
              <span className="flex size-8 items-center justify-center rounded-lg border border-hairline bg-surface-2">
                <span aria-hidden="true" className="size-3 rounded-full" style={{ background: accountColor(sub) }} />
              </span>
            }
            name={name}
            chips={
              <>
                {/* 0.4.1: out of room or low, unmistakably (lib/room-mark). */}
                <ProfileRoomChip profileId={sub.profiles[0]?.id} size="md" testId="account-room-chip" />
                <RenewalChip renewal={renewal} />
                <ChooseChip sub={sub} onOpen={() => showTab("settings")} />
              </>
            }
            facts={facts.map((part, index) => (
              <span key={part} className="flex items-center gap-2">
                {index > 0 && <span aria-hidden="true">·</span>}
                <span>{part}</span>
              </span>
            ))}
            actions={
              folder && (
                <Button size="md" variant="primary" onClick={() => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } }))} data-testid="account-new-session">
                  New session
                </Button>
              )
            }
          />
        }
        render={(tab, visible) => {
          switch (tab) {
            case "overview":
              return <AccountOverview sub={sub} accounts={accounts} items={items} apps={onIt} projects={worked} now={now} rows={(list) => shown(list, now, onOpenSession, host, appOf)} onOpenProject={onOpenProject} />;
            case "apps":
              return (
                <div className="w-full max-w-content rounded-xl border border-hairline bg-surface px-2 py-2" data-testid="account-apps-tab">
                  <AccountAppRows apps={onIt} now={now} />
                </div>
              );
            case "sessions":
              return (
                <RelatedList
                  listKey={`account-sessions:${sub.account_id}`}
                  anchor={{ subscription_id: sub.account_id }}
                  local={() => items.flatMap((item) => rowOf(item, Date.now(), onOpenSession, host, appOf(item)) ?? [])}
                  folders={accounts}
                  projects={projects}
                  showProjects
                  teach={{
                    title: `No sessions on ${name} yet`,
                    body: "Sessions in every App signed in to this Account show up here, what needs you first.",
                    action: folder ? { label: "New session", onClick: () => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } })) } : undefined,
                  }}
                  onOpenSession={onOpenSession}
                  searchLabel={`Search ${name}'s sessions`}
                />
              );
            case "projects":
              return <AppProjects projects={worked} sessions={items.length} now={now} onOpenProject={onOpenProject} />;
            case "pools":
              return (
                <RelatedPools
                  pools={pools}
                  teach={{ title: `No pools hold ${name}'s sessions yet`, body: "A session AI Juggler starts in one of its Apps joins its project's own pool and shows here; ones an app started itself aren't in pools." }}
                />
              );
            case "usage":
              return visible ? <AccountUsageTab name={name} folders={[...folderIds]} accounts={accounts} /> : null;
            case "settings":
              return <AccountSettings sub={sub} now={now} onChanged={onChanged} />;
            default:
              return null;
          }
        }}
      />
    </div>
  );
}

function shown(list: RailItem[], now: number, onOpenSession: (id: string) => void, host: ReturnType<typeof useActionHost>, appOf: (item: RailItem) => AppModel | null): Shown[] {
  return list.flatMap((item) => {
    const row = rowOf(item, now, onOpenSession, host, appOf(item));
    return row ? [{ row, project: item.projectId ? { id: item.projectId, name: item.project } : null }] : [];
  });
}

/** Its limits first, then its Apps, what's open on it, what was last
 * used, and where it works. */
function AccountOverview({
  sub,
  accounts,
  items,
  apps,
  projects,
  now,
  rows,
  onOpenProject,
}: {
  sub: SubscriptionInfo;
  accounts: AccountInfo[];
  items: RailItem[];
  apps: ReturnType<typeof accountApps>;
  projects: ReturnType<typeof appProjects>;
  now: number;
  rows: (list: RailItem[]) => Shown[];
  onOpenProject: (id: string) => void;
}): JSX.Element {
  const limits = useLimitsFolder(sub, accounts);
  const open = items.filter(openHere);
  const recent = items.filter((item) => !openHere(item)).slice(0, 5);
  return (
    <div className="@container flex w-full max-w-content flex-col gap-grid" data-testid="account-overview">
      {/* AJ-E3205 R3: nothing read, a door to measuring: its App's Settings. */}
      {limits && <AccountLimits account={limits} now={now} tone="colour" onMeasure={() => (rememberTab("app", "settings"), openApp(cliAppId(limits.id)))} />}
      <AccountQuality folders={sub.profiles.map((profile) => profile.id)} />
      <Section
        title="Its apps"
        icon={<IconProfiles width={16} height={16} />}
        sub={apps.length > 1 ? "they share these limits" : undefined}
        actions={apps.length > APPS_SHOWN ? <MoreLink label="All apps" onOpen={() => showTab("apps")} testId="account-all-apps" /> : undefined}
        testId="account-apps-top"
      >
        <AccountAppRows apps={apps.slice(0, APPS_SHOWN)} now={now} compact />
      </Section>
      <div className="grid grid-cols-1 gap-grid @min-[900px]:grid-cols-2">
        <Section title="Open now" icon={<IconPlay />} sub={open.length > 0 ? `${open.length} open` : undefined} testId="account-open">
          <RowList rows={rows(open)} empty="Nothing open on this account now." within="account" />
        </Section>
        <Section title="Last used" icon={<IconHistory />} actions={items.length > open.length + recent.length ? <MoreLink label="All sessions" onOpen={() => showTab("sessions")} testId="account-all-sessions" /> : undefined} testId="account-recent">
          <RowList rows={rows(recent)} empty={items.length === 0 ? "No sessions on it yet." : "Everything on it is open now."} within="account" />
        </Section>
      </div>
      {items.length > 0 && (
        <Section title="Projects" icon={<IconFolder />} sub="where it works, most recent first" actions={projects.length > 5 ? <MoreLink label="All projects" onOpen={() => showTab("projects")} testId="account-all-projects" /> : undefined} testId="account-projects-top">
          <ProjectRows projects={projects.slice(0, 5)} now={now} onOpenProject={onOpenProject} />
        </Section>
      )}
    </div>
  );
}

/** The folder whose reading speaks for the account: the freshest
 * measured one, else its first (accountDisplay's rule). */
function useLimitsFolder(sub: SubscriptionInfo, accounts: AccountInfo[]): AccountInfo | null {
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  useEffect(() => {
    let alive = true;
    void quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
    return () => {
      alive = false;
    };
  }, [sub.key]);
  const id = accountDisplay(sub, displays)?.account ?? sub.profiles[0]?.id;
  return accounts.find((account) => account.id === id) ?? null;
}

/** All of its folders' usage, as one. */
function AccountUsageTab({ name, folders, accounts }: { name: string; folders: string[]; accounts: AccountInfo[] }): JSX.Element {
  const [range, setRange] = useRange("subscription");
  return (
    <section aria-label="Usage" className="flex flex-col gap-3" data-testid="account-usage">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="text-section text-ink">Usage on {name}</h3>
        <Segmented label="Range" options={RANGES.map(({ id, label }) => ({ id, label }))} value={range} onChange={setRange} />
      </div>
      <UsageDashboard kind="profile" scope={{ profiles: folders }} range={range} accounts={accounts.filter((account) => folders.includes(account.id))} testId="account-usage-dashboard" />
    </section>
  );
}

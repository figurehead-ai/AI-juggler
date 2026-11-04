/**
 * A tool's page (Claude Code, Codex): its Overview — what's installed,
 * how many Apps run it and each Account's runway, least left first — then
 * (AJ-E3104, relations-map §4) everything it holds, each one click away:
 * its Apps, its sessions (the one list), the projects they worked in and
 * the pools they're in. The counts beside the tabs come from the same
 * relation cache as the lists.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import { ProfileBadge } from "../../components/ProfileBadge";
import { cliAppKey, cliAppName } from "../../lib/app-name";
import { AccountRunway, runwayOf } from "../../components/AccountRunway";
import { ToolUpdateLine } from "../../components/ToolUpdateLine";
import { RelatedList } from "../../components/RelatedList";
import { RelatedPools, usePagePools } from "../../components/RelatedPools";
import { useActionHost } from "../../components/ItemMenu";
import { hasLiveBackend, quotaDisplayAll, signedIn, type AccountInfo, type InstanceInfo, type ProjectInfo, type QuotaDisplayData } from "../../lib/bridge";
import { toolApps } from "../../lib/account-items";
import { appProjects } from "../../lib/app-items";
import { useApps } from "../../lib/apps";
import { useConversations } from "../../lib/conversations";
import { installWords, providerDisplayName } from "../../lib/providers";
import { appByConversation } from "../../lib/rail-apps";
import { itemsOf, type RailItem } from "../../lib/rail-items";
import { useRelationCounts } from "../../lib/relation-counts";
import { useSeen } from "../../lib/seen";
import { useShownConversations } from "../../lib/shown-apps";
import { useToolInstall } from "../../lib/useToolInstall";
import { toolOpen } from "../ItemHeader";
import { openWords } from "../../lib/project-summary";
import { AppProjects, rowOf } from "../app/AppOverview";
import { AccountAppRows } from "../subscription/AccountApps";
import type { PageTabId } from "../registry";

export interface ToolProps {
  tool: { id: string; name: string };
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
}

/** A tool's sessions (AI Juggler's and the ones its apps started), its
 * Apps and its projects, as its tabs list them. */
function useToolModel({ tool, sessions, projects, accounts }: ToolProps) {
  const apps = useApps(accounts);
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const items = useMemo(() => itemsOf({ sessions, conversations, projects, seen }).filter((item) => item.tool === tool.id), [sessions, conversations, projects, seen, tool.id]);
  const byConversation = useMemo(() => appByConversation((apps ?? []).flatMap((app) => (app.place ? [app.place] : []))), [apps]);
  const rows = useMemo(() => toolApps(apps ?? [], tool.id, items, byConversation), [apps, tool.id, items, byConversation]);
  const worked = useMemo(() => appProjects(items), [items]);
  const own = useMemo(() => items.flatMap((item) => (item.session ? [item.session] : [])), [items]);
  const pools = usePagePools(tool.id ? "tool" : null, tool.id, own);
  return { items, rows, worked, pools };
}

/** The numbers beside its tabs: its sessions as its list holds them (the
 * relation cache; the design preview counts what it shows), its Apps, its
 * projects and its pools. */
export function useToolTabCounts(props: ToolProps | null): Partial<Record<PageTabId, number>> | undefined {
  const related = useRelationCounts(props ? "tool" : null, props?.tool.id);
  const model = useToolModel(props ?? NO_TOOL);
  if (!props) return undefined;
  return {
    apps: model.rows.length,
    sessions: hasLiveBackend() ? related?.sessions.all : model.items.length,
    projects: hasLiveBackend() ? related?.projects : model.worked.length,
    pools: model.pools?.length,
  };
}

const NO_TOOL: ToolProps = { tool: { id: "", name: "" }, sessions: [], projects: [], accounts: [], now: 0, onOpenSession: () => {}, onOpenProject: () => {} };

export function ToolTab(props: ToolProps & { tab: PageTabId }): JSX.Element | null {
  const { tool, tab, now } = props;
  const host = useActionHost();
  const { items, rows, worked, pools } = useToolModel(props);
  const name = providerDisplayName(tool.id);
  switch (tab) {
    case "overview":
      return <ToolOverview {...props} />;
    case "apps":
      return (
        <div className="w-full max-w-content rounded-xl border border-hairline bg-surface px-2 py-2" data-testid="tool-apps-tab">
          <AccountAppRows apps={rows} now={now} empty={`No app runs ${name} yet. Add one with Add app…`} />
        </div>
      );
    case "sessions":
      return (
        <RelatedList
          listKey={`tool-sessions:${tool.id}`}
          anchor={{ tool_id: tool.id }}
          local={() => items.flatMap((item: RailItem) => rowOf(item, Date.now(), props.onOpenSession, host) ?? [])}
          folders={props.accounts}
          projects={props.projects}
          showProjects
          teach={{ title: `No ${name} sessions yet`, body: `Sessions in every App that runs ${name} show up here, what needs you first.` }}
          onOpenSession={props.onOpenSession}
          searchLabel={`Search ${name}'s sessions`}
        />
      );
    case "projects":
      return <AppProjects projects={worked} sessions={items.length} now={now} onOpenProject={props.onOpenProject} />;
    case "pools":
      return (
        <RelatedPools
          pools={pools}
          teach={{ title: `No pools hold ${name} sessions yet`, body: `A ${name} session AI Juggler starts joins its project's own pool and shows here.` }}
        />
      );
    default:
      return null;
  }
}

/** Its Overview (§5.3): what's installed, its Apps and how many run, and
 * each Account's runway, least left first (recognition over recall). */
function ToolOverview({ tool, accounts, sessions }: ToolProps): JSX.Element {
  const install = useToolInstall(tool.id);
  const { all: conversations } = useConversations();
  // One source per fact: each account's line is Runway's own (runwayOf).
  const [quota, setQuota] = useState<QuotaDisplayData[]>([]);
  useEffect(() => {
    let alive = true;
    quotaDisplayAll().then(
      (rows) => alive && setQuota(rows),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [tool.id]);
  const toolAccounts = accounts.filter((a) => a.provider === tool.id);
  const toolSessions = sessions.filter((s) => s.provider === tool.id);
  // One count everywhere: its sessions running now, outside ones too (AJ-E1529).
  const now = toolOpen(tool.id, sessions, conversations);
  const perAccount = toolAccounts
    .map((account) => {
      const display = quota.find((q) => q.account === account.id);
      const mine = toolSessions.filter((s) => s.account_id === account.id);
      return { account, display, sessions: mine, left: runwayOf(display, mine).left };
    })
    .sort((a, b) => (a.left ?? Number.POSITIVE_INFINITY) - (b.left ?? Number.POSITIVE_INFINITY));
  return (
    <div className="flex flex-col gap-3" data-testid="provider-overview">
      <Card className="flex flex-col gap-2 p-4">
        {/* The header's words for it, a project's (one count, one word). */}
        <p className="text-meta">
          <span className="text-ink">
            {toolAccounts.length} app{toolAccounts.length === 1 ? "" : "s"}
          </span>
          <span className="text-muted"> · {openWords(now)}</span>
        </p>
        {/* Shown once known; nothing in its place until then (D-25). */}
        {install !== null && (
          <p className="text-label text-muted" data-testid="provider-install">
            {install === "failed" ? "Couldn't check what's installed" : installWords(install)}
          </p>
        )}
        {/* AJ-E1611: a newer version, and how to update it yourself. */}
        <ToolUpdateLine tool={tool.id} name={providerDisplayName(tool.id)} />
      </Card>
      <Card className="flex flex-col gap-1.5 p-4" data-testid="provider-aggregate">
        <p className="text-section font-semibold text-ink">Runway, least left first</p>
        {perAccount.length === 0 ? (
          <p className="text-meta text-ink-2">No apps for this tool yet.</p>
        ) : (
          perAccount.map(({ account, display, sessions: mine }) => (
            <div key={account.id} className="flex flex-col gap-1.5 py-1">
              <span className="flex items-center gap-1.5 text-meta">
                {/* Each App by its full name and its mark (the 0.4 review). */}
                <ProfileBadge app={cliAppKey(account.provider, account.id, account.label)} name={account.label} size={8} />
                <span className="text-ink">{cliAppName(account.provider, account.label)}</span>
                {!signedIn(account.login_state) ? <span className="text-muted"> · signed out</span> : null}
              </span>
              {/* D-25: a signed-out account with no reading says "signed
                  out" above; its runway would only repeat it. */}
              {display || signedIn(account.login_state) ? <AccountRunway display={display} sessions={mine} /> : null}
            </div>
          ))
        )}
      </Card>
    </div>
  );
}

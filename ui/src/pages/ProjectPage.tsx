/**
 * AJ-E1533 — a project's page (wireframe 03): its header facts, its body
 * (every session there, AI Juggler's and the ones it didn't start, live
 * first) and its Overview (accounts, sessions ever, checkouts, which
 * account starts work here, how the work travelled).
 */
import { AccountAvatar } from "../components/Avatar";
import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { PROJECT_SYNC_EVENT, ProjectSyncSheet } from "../components/ProjectSyncSheet";
import { projectSyncRules } from "../lib/ipc/projectSync";
import { useConversations, outside } from "../lib/conversations";
import { projectsGitStatus } from "../lib/ipc/projects";
import type { GitStatusInfo } from "../types/generated/GitStatusInfo";
import { useActionHost } from "../components/ItemMenu";
import { useProjectManage } from "../components/ManageWork";
import { ProjectDefaultAccount } from "../components/ProjectDefaultAccount";
import { Section } from "../components/charts/ChartCard";
import { IconGit, IconUser } from "../components/usage/controls";
import { ProjectOverviewView } from "./project/Overview";
import { homeRelative, projectStats } from "../lib/project-words";
import { isActiveStatus } from "../components/indicator";
import { IconTerminal } from "../components/icons";
import { Button, Card } from "../components/primitives";
import { signedIn, type AccountInfo, type InstanceInfo, type LaunchRecipeInfo, type ProjectInfo } from "../lib/bridge";
import { SessionsList, useSessionRows } from "./SessionsList";
import { launchSettingsName } from "../lib/account-words";

export type ProjectPanelProps = {
  project: ProjectInfo;
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onLaunch: (recipe: LaunchRecipeInfo, project: ProjectInfo) => void;
  /** Reads the app's lists again after a change made here. */
  onStopped: () => void;
};

/** A project's facts: where it is, its branch, how far from its upstream
 * and what isn't committed — read from its own root; until that answers
 * (or for a folder that isn't a git checkout), its sessions' branches. */
export function ProjectFacts({ project, sessions }: { project: ProjectInfo; sessions: InstanceInfo[] }) {
  const [git, setGit] = useState<GitStatusInfo | null>(null);
  useEffect(() => {
    let alive = true;
    setGit(null);
    projectsGitStatus(project.id).then(
      (status) => alive && setGit(status),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [project.id]);
  const branches = git?.branch
    ? [git.branch]
    : [...new Set(sessions.filter((s) => s.project_id === project.id && s.git_branch).map((s) => s.git_branch!))];
  const ahead = git?.ahead ?? 0;
  const behind = git?.behind ?? 0;
  const uncommitted = git ? git.changed + git.untracked : 0;
  return (
    <>
      <span className="font-mono text-label" title={project.root} data-testid="project-root">
        {homeRelative(project.root)}
      </span>
      {branches.length > 0 && <span data-testid="project-branch">· {branches.join(", ")}</span>}
      {ahead + behind > 0 && (
        <span data-testid="project-upstream">· {[ahead > 0 && `${ahead} ahead`, behind > 0 && `${behind} behind`].filter(Boolean).join(", ")}</span>
      )}
      {uncommitted > 0 && (
        <span className="text-warning" data-testid="project-uncommitted">
          · {uncommitted === 1 ? "1 file" : `${uncommitted} files`} not committed
        </span>
      )}
      <RulesLine project={project} />
    </>
  );
}

/** AJ-E1507: whether the project's rules files agree ("Rules in sync",
 * "1 rules file differs"), opening the sheet that compares them. Nothing
 * shows where there's nothing to compare. */
function RulesLine({ project }: { project: ProjectInfo }) {
  const [summary, setSummary] = useState<{ words: string; inSync: boolean } | null>(null);
  const [open, setOpen] = useState(false);
  const read = useCallback(() => {
    let alive = true;
    projectSyncRules(project.id).then(
      (rules) => alive && setSummary(rules.summary ? { words: rules.summary, inSync: rules.in_sync } : null),
      () => alive && setSummary(null),
    );
    return () => {
      alive = false;
    };
  }, [project.id]);
  useEffect(read, [read]);
  // The sheet can also be opened from the project's menu.
  useEffect(() => {
    const again = (event: Event) => {
      if ((event as CustomEvent<string>).detail === project.id) read();
    };
    window.addEventListener(PROJECT_SYNC_EVENT, again);
    return () => window.removeEventListener(PROJECT_SYNC_EVENT, again);
  }, [project.id, read]);
  if (!summary) return null;
  return (
    <>
      <span>
        ·{" "}
        <button
          type="button"
          onClick={() => setOpen(true)}
          className={`underline decoration-dotted underline-offset-2 hover:text-ink ${summary.inSync ? "" : "text-warning"}`}
          data-testid="project-rules"
        >
          {summary.words}
        </button>
      </span>
      {open &&
        createPortal(
          <ProjectSyncSheet
            project={project}
            onClose={() => setOpen(false)}
          />,
          document.body,
        )}
    </>
  );
}

/** A project's body: its sessions, AI Juggler's and the ones it didn't
 * start, live first. */
export function ProjectSessions(props: ProjectPanelProps) {
  const project = props.project;
  const host = useActionHost();
  const rows = useSessionRows(props.sessions, { projectId: project?.id ?? "" }, props.onOpenSession, props.now);
  const start = () => (host.newSessionIn ? host.newSessionIn(project) : window.dispatchEvent(new Event("aij:new-session")));
  return (
    <div className="flex flex-col gap-3">
      <MissingBanner project={project} onStopped={props.onStopped} />
      {rows.length > 0 ? <SessionsList rows={rows} empty="" /> : <EmptyProject project={project} now={props.now} onStart={start} />}
    </div>
  );
}

/** A project with no session yet: what was done there, and a start. */
function EmptyProject({ project, now, onStart }: { project: ProjectInfo; now: number; onStart: () => void }) {
  return (
    <Card className="flex flex-col items-center gap-3 p-6 text-center" data-testid="project-empty">
      {/* 0.0.11 walk, Q2: a project known from the setups' history says
          what was done there — the rail row's own line. */}
      {project.history_sessions > 0 ? (
        <div className="flex flex-col items-center gap-1">
          <p className="text-body font-medium text-ink">Nothing started from AI Juggler here yet</p>
          <p className="text-meta text-muted" data-testid="project-history-line">
            {projectStats(project, now).join(" · ")}
          </p>
        </div>
      ) : (
        <p className="text-body font-medium text-ink">No sessions in this project yet</p>
      )}
      <Button size="sm" variant="primary" onClick={onStart}>
        <IconTerminal width={13} height={13} />
        Start first session
      </Button>
    </Card>
  );
}

/** Project Overview (AJ-E1533; wireframe 03): what the project is beyond
 * its sessions (the body) — every account that touched it, how many
 * sessions it has had, its checkouts with a warning when two live
 * sessions share one, which account starts work here, and how the work
 * travelled. */
export function ProjectOverview(props: ProjectPanelProps) {
  const project = props.project;
  const { all: conversations } = useConversations();
  const rows = props.sessions.filter((s) => s.project_id === project.id);
  const sessionRows = useSessionRows(props.sessions, { projectId: project.id }, props.onOpenSession, props.now);
  const theirs = outside(conversations).filter((c) => c.project_id === project.id);
  const accountsUsed = [...new Set([...project.accounts_used, ...rows.map((r) => r.account_label), ...theirs.map((c) => c.account_label)])];
  // The checkouts its sessions worked in, and who's in each now.
  const checkouts = new Map<string, { branch: string | null; live: InstanceInfo[] }>();
  for (const row of rows) {
    const path = row.cwd;
    if (!path) continue;
    const entry = checkouts.get(path) ?? { branch: row.git_branch, live: [] };
    if (isActiveStatus(row.status)) entry.live.push(row);
    checkouts.set(path, entry);
  }
  const ranked = props.profiles
    .map((recipe) => {
      const worst = props.sessions
        .filter((s) => s.account_id === recipe.account_id && s.quota !== null)
        .reduce<number | null>((min, row) => {
          const left = 100 - row.quota!.used_percentage;
          return min === null ? left : Math.min(min, left);
        }, null);
      return { recipe, left: worst ?? Number.POSITIVE_INFINITY, usual: accountsUsed.includes(recipe.account_label) };
    })
    .sort((a, b) => b.left - a.left);
  const launch = (recipe: LaunchRecipeInfo) =>
    props.onLaunch(recipe, {
      id: project.id, root: project.root, name: project.name, last_activity_ms: null,
      active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: project.missing,
    });
  const sharedBy = Math.max(0, ...[...checkouts.values()].map((entry) => entry.live.length));
  const accountsCard = (
    <Section title="Accounts" icon={<IconUser />} sub="who starts work here" testId="project-accounts">
      <ProjectDefaultAccount project={project} accounts={props.accounts} onChanged={props.onStopped} />
      {/* Ranked only by what's known (P21): no headroom read yet, no claim. */}
      {ranked.length > 0 && (
        <p className="text-meta text-muted">
          {ranked.some((entry) => Number.isFinite(entry.left)) ? "Or start one on another account, most room first" : "Or start one on another account"}
        </p>
      )}
      <ul className="flex flex-col gap-1.5">
        {ranked.map(({ recipe, usual, left }) => {
          const ready = signedIn(props.accounts.find((a) => a.id === recipe.account_id)?.login_state);
          return (
            <li key={recipe.id}>
              <button
                type="button"
                disabled={!ready}
                title={ready ? launchSettingsName(recipe.name) : "Needs sign-in first"}
                onClick={() => launch(recipe)}
                className="flex w-full items-center gap-2.5 rounded-lg border border-hairline bg-surface-2 px-3 py-2 text-left text-meta text-ink outline-none hover:border-accent/40 focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60"
              >
                <AccountAvatar accountId={recipe.account_id} label={recipe.account_label} size={20} />
                <span className="min-w-0 flex-1 truncate font-medium">
                  {recipe.account_label}
                  {/* An account with several launch settings: say which. */}
                  {ranked.filter((other) => other.recipe.account_id === recipe.account_id).length > 1 && <span className="font-normal text-muted"> · {launchSettingsName(recipe.name)}</span>}
                </span>
                {usual && <span className="rounded-md bg-accent-soft px-1.5 py-0.5 text-label font-semibold text-accent-hi">Usual</span>}
                {Number.isFinite(left) && <span className={"text-label tabular-nums " + (left <= 25 ? "text-warning" : "text-muted")}>{Math.round(left)}% left</span>}
              </button>
            </li>
          );
        })}
      </ul>
      {ranked.length === 0 && <p className="text-meta text-muted">No launch settings yet: add an account first.</p>}
    </Section>
  );
  const checkoutsCard =
    checkouts.size > 1 ? (
      <Section title="Checkouts" icon={<IconGit />} sub={`${checkouts.size} folders this project's sessions worked in`} className="@min-[700px]:col-span-2" testId="project-checkouts">
        <ul className="flex flex-col divide-y divide-hairline">
          {[...checkouts].map(([path, entry]) => (
            <li key={path} className="flex flex-wrap items-center gap-x-3 gap-y-1 py-2 text-meta">
              <span className="font-mono text-ink" title={path}>
                {homeRelative(path)}
              </span>
              {entry.branch && <span className="text-muted">{entry.branch}</span>}
              <span className="ml-auto text-muted">{entry.live.length > 0 ? `${entry.live.length} running` : "nothing running"}</span>
            </li>
          ))}
        </ul>
      </Section>
    ) : null;
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="project-overview">
      <MissingBanner project={project} onStopped={props.onStopped} />
      <ProjectOverviewView
        project={project}
        rows={sessionRows}
        now={props.now}
        sharedBy={sharedBy}
        accounts={accountsCard}
        checkouts={checkoutsCard}
        onOpenSession={props.onOpenSession}
        onAllSessions={() => document.getElementById("page-tab-sessions")?.click()}
      />
    </div>
  );
}

/** A project whose folder moved, said first on whichever tab is open,
 * with Relocate… to point it at where it went (AJ-E1502). */
function MissingBanner({ project, onStopped }: { project: ProjectInfo; onStopped: () => void }) {
  const manage = useProjectManage(onStopped);
  if (!project.missing) return null;
  return (
    <>
      <div className="flex items-center gap-3 rounded-xl border border-danger/40 bg-danger-soft px-4 py-3" data-testid="project-missing-banner" role="alert">
        <span className="text-[12px] font-medium text-danger">Project directory is missing</span>
        <span className="min-w-0 flex-1 truncate font-mono text-label text-muted">{homeRelative(project.root)}</span>
        <Button size="sm" variant="secondary" onClick={() => void manage.actions.relocate(project)}>
          Relocate…
        </Button>
      </div>
      {manage.element}
    </>
  );
}

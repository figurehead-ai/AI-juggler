/**
 * AJ-E1533 — a project's page (wireframe 03): its header facts, its body
 * (every session there, AI Juggler's and the ones it didn't start, live
 * first) and its Overview (accounts, sessions ever, checkouts, which
 * account starts work here, how the work travelled).
 */
import { useEffect, useState } from "react";
import { useConversations, outside } from "../lib/conversations";
import { projectsGitStatus } from "../lib/ipc/projects";
import type { GitStatusInfo } from "../types/generated/GitStatusInfo";
import { useActionHost } from "../components/ItemMenu";
import { useProjectManage } from "../components/ManageWork";
import { ProjectDefaultAccount } from "../components/ProjectDefaultAccount";
import { SwitchGraph } from "../components/SwitchGraph";
import { projectStats } from "../components/WorkflowRail";
import { AccountDisc, isActiveStatus } from "../components/indicator";
import { IconTerminal } from "../components/icons";
import { Button, Card } from "../components/primitives";
import { signedIn, type AccountInfo, type InstanceInfo, type LaunchRecipeInfo, type ProjectInfo } from "../lib/bridge";
import { SessionsList, useSessionRows } from "./SessionsList";

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
      <span className="font-mono text-[11px]" title={project.root} data-testid="project-root">
        {project.root.replace(/^\/Users\/[^/]+/, "~")}
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
    </>
  );
}

/** A project's body: its sessions, AI Juggler's and the ones it didn't
 * start, live first. */
export function ProjectSessions(props: ProjectPanelProps) {
  const project = props.project;
  const host = useActionHost();
  const rows = useSessionRows(props.sessions, { projectId: project?.id ?? "" }, props.onOpenSession);
  if (rows.length > 0) return <SessionsList rows={rows} now={props.now} empty="" />;
  const start = () => (host.newSessionIn ? host.newSessionIn(project) : window.dispatchEvent(new Event("aij:new-session")));
  return (
    <Card className="flex flex-col items-center gap-3 p-6 text-center" data-testid="project-empty">
      {/* 0.0.11 walk, Q2: a project known from the setups' history says
          what was done there — the rail row's own line. */}
      {project.history_sessions > 0 ? (
        <div className="flex flex-col items-center gap-1">
          <p className="text-[12px] font-medium text-ink">Nothing started from AI Juggler here yet</p>
          <p className="text-[11.5px] text-muted" data-testid="project-history-line">
            {projectStats(project, props.now).join(" · ")}
          </p>
        </div>
      ) : (
        <p className="text-[12px] font-medium text-ink">No sessions in this project yet</p>
      )}
      <Button size="sm" variant="primary" onClick={start}>
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
  const manage = useProjectManage(props.onStopped);
  const { all: conversations } = useConversations();
  const rows = props.sessions.filter((s) => s.project_id === project.id);
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
  return (
    <div className="flex flex-col gap-3" data-testid="project-overview">
      {project.missing ? (
        <div className="flex items-center gap-3 rounded-xl border border-danger/40 bg-danger-soft px-4 py-3" data-testid="project-missing-banner" role="alert">
          <span className="text-[12px] font-medium text-danger">Project directory is missing</span>
          <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted">{project.root}</span>
          {/* AJ-E1502: point it at where the folder moved. */}
          <Button size="sm" variant="secondary" onClick={() => void manage.actions.relocate(project)}>
            Relocate…
          </Button>
        </div>
      ) : null}
      {manage.element}
      <Card className="flex flex-col gap-2 p-4" data-testid="project-summary">
        <p className="text-[11px] uppercase tracking-wider text-muted">At a glance</p>
        <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 text-[12px]">
          <dt className="text-muted">Sessions</dt>
          <dd className="text-ink" data-testid="project-sessions-ever">
            {rows.length + theirs.length}
            {theirs.length > 0 && <span className="text-muted"> ({theirs.length} outside AI Juggler)</span>}
            {project.history_sessions > 0 && <span className="text-muted"> · {projectStats(project, props.now).join(" · ")}</span>}
          </dd>
          <dt className="text-muted">Accounts</dt>
          <dd className="text-ink">{accountsUsed.join(", ") || "None yet"}</dd>
        </dl>
      </Card>
      {checkouts.size > 0 && (
        <Card className="flex flex-col gap-2 p-4" data-testid="project-checkouts">
          <p className="text-[11px] uppercase tracking-wider text-muted">Checkouts</p>
          {[...checkouts].map(([path, entry]) => (
            <div key={path} className="flex flex-col gap-0.5 text-[12px]">
              <span className="flex flex-wrap items-center gap-x-2">
                <span className="font-mono text-[11px] text-ink" title={path}>
                  {path.replace(/^\/Users\/[^/]+/, "~")}
                </span>
                {entry.branch && <span className="text-muted">{entry.branch}</span>}
                <span className="text-faint">{entry.live.length > 0 ? `${entry.live.length} running` : "nothing running"}</span>
              </span>
              {entry.live.length > 1 && (
                <span className="text-[11px] text-warning" data-testid="shared-checkout">
                  {entry.live.length} sessions share this checkout: their edits can collide. Give the next one its own worktree.
                </span>
              )}
            </div>
          ))}
        </Card>
      )}
      <Card className="flex flex-col gap-2 p-4" data-testid="project-accounts">
        <p className="text-[11px] uppercase tracking-wider text-muted">Accounts</p>
        <ProjectDefaultAccount project={project} accounts={props.accounts} onChanged={props.onStopped} />
        {ranked.length > 0 && <p className="text-[11px] text-muted">Start one on another account (most left first):</p>}
        <div className="flex flex-wrap gap-2">
          {ranked.map(({ recipe, usual }) => {
            const ready = signedIn(props.accounts.find((a) => a.id === recipe.account_id)?.login_state);
            return (
              <Button key={recipe.id} size="sm" variant="secondary" disabled={!ready} title={ready ? recipe.name : "Needs sign-in first"} onClick={() => launch(recipe)}>
                <AccountDisc accountId={recipe.account_id} label={recipe.account_label} size={14} />
                {recipe.account_label}
                {/* An account with several launch settings: say which. */}
                {ranked.filter((other) => other.recipe.account_id === recipe.account_id).length > 1 && (
                  <span className="text-muted">· {recipe.name}</span>
                )}
                {usual && <span className="text-[9px] text-accent-hi">Usual</span>}
              </Button>
            );
          })}
        </div>
        {ranked.length === 0 && <p className="text-[11.5px] text-muted">No launch settings yet: add an account first.</p>}
      </Card>
      {/* AJ-E209 Req.4/5: the per-Work Switch Graph — how the work
          travelled across accounts, with per-node Resume / Switch
          from here / Export pack and failed-edge recovery. */}
      <Card className="flex flex-col gap-2 p-4" data-testid="project-switch-graph-card">
        <p className="text-[11px] uppercase tracking-wider text-muted">How this work travelled</p>
        <SwitchGraph projectId={project.id} onOpenSession={props.onOpenSession} />
      </Card>
    </div>
  );
}

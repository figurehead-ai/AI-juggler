/**
 * 0.1.5 — a project's Overview, its plain-words cards: the checkout's
 * state as a callout (what isn't committed, how far from upstream, rules
 * files that differ, each with what to do), and the sessions open there
 * now, each opening its page.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { useActionHost } from "../../components/ItemMenu";
import { PROJECT_SYNC_EVENT, ProjectSyncSheet } from "../../components/ProjectSyncSheet";
import { Section } from "../../components/charts/ChartCard";
import { IconGit, IconPlay, IconRules } from "../../components/usage/controls";
import { projectSyncRules } from "../../lib/ipc/projectSync";
import { projectsGitStatus } from "../../lib/ipc/projects";
import { homeRelative } from "../../lib/project-words";
import type { ProjectInfo } from "../../lib/bridge";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";
import { PagedSessions, type Row } from "../SessionsList";
import { sessionsQuery } from "../../lib/ipc/sessions";

const BUTTON =
  "inline-flex h-8 shrink-0 items-center rounded-lg border border-hairline bg-surface-2 px-3 text-meta font-medium text-ink outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60";

function Item({ tone, icon, title, words, action, testId }: { tone: "warning" | "ok"; icon: JSX.Element; title: string; words: string; action?: JSX.Element; testId?: string }): JSX.Element {
  return (
    <div className="flex flex-wrap items-start gap-3 border-t border-hairline py-3 first:border-t-0 first:pt-0" data-testid={testId}>
      <span aria-hidden="true" className={"flex size-8 shrink-0 items-center justify-center rounded-lg " + (tone === "warning" ? "bg-warning-soft text-warning" : "bg-success-soft text-success")}>
        {icon}
      </span>
      <span className="flex min-w-[12rem] flex-1 flex-col gap-0.5">
        <span className="text-body font-semibold text-ink">{title}</span>
        <span className="text-meta text-muted">{words}</span>
      </span>
      {action}
    </div>
  );
}

/** The checkout: git's state and the rules files, said plainly with what to do. */
export function CheckoutCallout({ project, sharedBy, className = "" }: { project: ProjectInfo; sharedBy: number; className?: string }): JSX.Element | null {
  const host = useActionHost();
  const [git, setGit] = useState<GitStatusInfo | null | undefined>(undefined);
  const [rules, setRules] = useState<{ words: string; inSync: boolean } | null>(null);
  const [compare, setCompare] = useState(false);
  useEffect(() => {
    let alive = true;
    projectsGitStatus(project.id).then(
      (status) => alive && setGit(status),
      () => alive && setGit(null),
    );
    return () => {
      alive = false;
    };
  }, [project.id]);
  const readRules = useCallback(() => {
    let alive = true;
    projectSyncRules(project.id).then(
      (read) => alive && setRules(read.summary ? { words: read.summary, inSync: read.in_sync } : null),
      () => alive && setRules(null),
    );
    return () => {
      alive = false;
    };
  }, [project.id]);
  useEffect(readRules, [readRules]);
  useEffect(() => {
    const again = (event: Event) => {
      if ((event as CustomEvent<string>).detail === project.id) readRules();
    };
    window.addEventListener(PROJECT_SYNC_EVENT, again);
    return () => window.removeEventListener(PROJECT_SYNC_EVENT, again);
  }, [project.id, readRules]);

  const uncommitted = git ? git.changed + git.untracked : 0;
  const ahead = git?.ahead ?? 0;
  const behind = git?.behind ?? 0;
  const upstream = [ahead > 0 && `${ahead} ${ahead === 1 ? "commit" : "commits"} ahead of its upstream`, behind > 0 && `${behind} behind`].filter(Boolean).join(", ");
  const attention = uncommitted > 0 || (rules !== null && !rules.inSync) || sharedBy > 1;
  if (git === undefined && rules === null) return null;
  return (
    <Section title="The checkout" icon={<IconGit />} sub={git?.branch ? `on ${git.branch}` : homeRelative(project.root)} className={(attention ? "border-warning/30 " : "") + className} testId="project-checkout">
      <div className="flex flex-col">
        {git === null ? (
          <Item tone="ok" icon={<IconGit width={15} height={15} />} title="Not a git checkout" words="Nothing to commit here; AI Juggler reads git's state only where there is one." />
        ) : git ? (
          uncommitted > 0 ? (
            <Item
              tone="warning"
              icon={<IconGit width={15} height={15} />}
              title={`${uncommitted.toLocaleString()} ${uncommitted === 1 ? "file" : "files"} not committed`}
              words={[`${git.changed} changed, ${git.untracked} new`, upstream].filter(Boolean).join(" · ")}
              action={
                host.openProjectIn ? (
                  <button type="button" className={BUTTON} onClick={() => void host.openProjectIn!(project, "editor")}>
                    Open in editor
                  </button>
                ) : (
                  <button type="button" className={BUTTON} onClick={() => void (host.copyProjectPath ? host.copyProjectPath(project) : navigator.clipboard?.writeText(project.root))}>
                    Copy its path
                  </button>
                )
              }
              testId="project-uncommitted-callout"
            />
          ) : (
            <Item tone="ok" icon={<IconGit width={15} height={15} />} title="Everything is committed" words={upstream || "Up to date with its upstream."} />
          )
        ) : null}
        {sharedBy > 1 && (
          <Item tone="warning" icon={<IconPlay width={15} height={15} />} title={`${sharedBy} sessions share this checkout`} words="Their edits can collide. Give the next one its own worktree." testId="shared-checkout" />
        )}
        {rules && (
          <Item
            tone={rules.inSync ? "ok" : "warning"}
            icon={<IconRules width={15} height={15} />}
            title={rules.words}
            words={rules.inSync ? "Every tool works from the same rules." : "One tool works from rules the others don't have."}
            action={
              <button type="button" className={BUTTON} onClick={() => setCompare(true)} data-testid="project-rules-compare">
                {rules.inSync ? "Show rules" : "Compare…"}
              </button>
            }
          />
        )}
      </div>
      {compare && createPortal(<ProjectSyncSheet project={project} onClose={() => setCompare(false)} />, document.body)}
    </Section>
  );
}

/** The sessions open in the project now, each opening its page: the
 * backend's open list, a page at a time (0.1.5); `rows` are the design
 * preview's and say when the last one ran. */
export function OpenSessions({ projectId, rows, onAll, onOpenSession, className = "" }: { projectId: string; rows: Row[]; onAll: () => void; onOpenSession: (id: string) => void; className?: string }): JSX.Element {
  const local = rows.filter((row) => row.live);
  const [open, setOpen] = useState<number | null>(null);
  const count = open ?? local.length;
  return (
    <Section
      title="Open sessions"
      icon={<IconPlay />}
      sub={count === 0 ? "none open now" : `${count} open now`}
      className={className}
      testId="project-open-sessions"
      actions={
        <button type="button" onClick={onAll} className="text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60">
          All sessions ›
        </button>
      }
    >
      <PagedSessions
        listKey={`project-open:${projectId}`}
        query={sessionsQuery({ project_id: projectId, which: "open" })}
        local={() => local}
        empty={`Nothing is open here now. ${rows.length > 0 ? `The last one ran ${rows[0]!.when}.` : "Start a session from the project's header."}`}
        onOpenSession={onOpenSession}
        onTotal={setOpen}
        counted
      />
    </Section>
  );
}

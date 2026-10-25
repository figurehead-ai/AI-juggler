/**
 * 0.1.5 — a project's Overview, its plain-words cards: the checkout's
 * state as a callout (what isn't committed, how far from upstream, rules
 * files that differ, each with what to do), and the sessions open there
 * now, each opening its page.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { AccountAvatar } from "../../components/Avatar";
import { ToolMark } from "../../components/ToolMark";
import { useActionHost } from "../../components/ItemMenu";
import { PROJECT_SYNC_EVENT, ProjectSyncSheet } from "../../components/ProjectSyncSheet";
import { Section } from "../../components/charts/ChartCard";
import { IconGit, IconPlay, IconRules } from "../../components/usage/controls";
import { projectSyncRules } from "../../lib/ipc/projectSync";
import { projectsGitStatus } from "../../lib/ipc/projects";
import { homeRelative } from "../../lib/project-words";
import type { ProjectInfo } from "../../lib/bridge";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";
import type { Row } from "../SessionsList";

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
              <button type="button" className={BUTTON} onClick={() => setCompare(true)} data-testid="project-rules">
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

/** A state's chip: its tone's text on its tone's soft fill (the tokens keep both AA). */
const CHIP: Record<string, string> = {
  "text-warning": "text-warning bg-warning-soft",
  "text-live": "text-live bg-live-soft",
  "text-success": "text-success bg-success-soft",
  "text-danger": "text-danger bg-danger-soft",
};
const DOT: Record<string, string> = { "text-warning": "bg-warning", "text-live": "bg-live", "text-success": "bg-success", "text-danger": "bg-danger" };

/** The sessions open in the project now, each opening its page. */
export function OpenSessions({ rows, onAll, className = "" }: { rows: Row[]; onAll: () => void; className?: string }): JSX.Element {
  const live = rows.filter((row) => row.live);
  return (
    <Section
      title="Open sessions"
      icon={<IconPlay />}
      sub={live.length === 0 ? "none open now" : `${live.length} open now`}
      className={className}
      testId="project-open-sessions"
      actions={
        <button type="button" onClick={onAll} className="text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60">
          All sessions ›
        </button>
      }
    >
      {live.length === 0 ? (
        <p className="rounded-lg border border-dashed border-hairline px-4 py-5 text-center text-meta text-muted">
          Nothing is open here now. {rows.length > 0 ? `The last one ran ${rows[0]!.when}.` : "Start a session from the project's header."}
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {live.slice(0, 6).map((row) => {
            return (
              <li key={row.key}>
                <button
                  type="button"
                  onClick={row.open}
                  className="flex w-full items-center gap-3 rounded-xl border border-hairline bg-surface-2 px-3 py-2.5 text-left outline-none hover:border-accent/40 focus-visible:ring-2 focus-visible:ring-accent/60"
                  data-testid="project-open-session"
                >
                  <span className="flex shrink-0 items-center gap-1.5">
                    <span className="flex size-8 items-center justify-center rounded-lg bg-surface-3 text-ink">
                      <ToolMark tool={row.tool} size={18} />
                    </span>
                    <AccountAvatar accountId={row.accountId} label={row.accountLabel} size={18} />
                  </span>
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-body font-semibold text-ink">{row.title}</span>
                    <span className="truncate text-meta text-muted">{[row.accountLabel, row.detail, row.when].filter(Boolean).join(" · ")}</span>
                  </span>
                  <span className={"inline-flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-label font-semibold " + (CHIP[row.tone] ?? "bg-surface-3 text-muted")}>
                    <span aria-hidden="true" className={"size-1.5 rounded-full " + (DOT[row.tone] ?? "bg-faint")} />
                    {row.state}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Section>
  );
}

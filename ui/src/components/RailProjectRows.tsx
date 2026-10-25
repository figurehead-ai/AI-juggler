/**
 * The rail's project rows: a project's group heading, and a project with
 * no session here yet. Built like a session row (RailRow) — the same
 * size, spacing, hover and selection (AJ-E1534: a filled row, the accent
 * bar and `aria-current` when its page is open). Each is named by the
 * project's name; its folder, home-relative, is only its description
 * (P24: never the absolute path).
 */
import { useId, type JSX, type ReactNode } from "react";
import { useItemMenu } from "./ItemMenu";
import { AccountStack } from "./Avatar";
import { ProjectMark } from "./ProjectMark";
import { IconPin } from "./icons";
import { homeRelative, projectStats } from "../lib/project-words";
import { accountsWords, projectAccounts } from "../lib/project-accounts";
import { useConversations } from "../lib/conversations";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

/** The accent bar a selected row carries. */
function SelectedBar(): JSX.Element {
  return <span aria-hidden="true" className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />;
}

/** AJ-E1502: a pinned row's mark, beside its title. */
export function PinMark(): JSX.Element {
  return (
    <span title="Pinned" className="shrink-0 text-muted" data-testid="pin-mark">
      <IconPin width={10} height={10} aria-hidden="true" />
    </span>
  );
}

/** A project with no session here yet: its mark (AJ-E1532) where a
 * session shows its state, what was done there where a session shows its
 * state line, and the accounts that worked there where a session shows
 * its account. */
export function ProjectRow({
  project,
  density,
  now,
  selected,
  onSelect,
}: {
  project: ProjectInfo;
  density: "comfortable" | "compact";
  now: number;
  selected: boolean;
  onSelect: (id: string) => void;
}): JSX.Element {
  const stats = projectStats(project, now);
  const when = stats[0]?.endsWith("ago") || stats[0] === "just now" ? stats[0] : null;
  const menu = useItemMenu("project", project, project.name);
  const accounts = projectAccounts(project, [], useConversations().all);
  const accountsId = useId();
  return (
    <div className="group relative" {...menu.triggerProps}>
      <button
        type="button"
        onClick={() => onSelect(project.id)}
        title={homeRelative(project.root)}
        aria-describedby={accounts.length > 0 ? accountsId : undefined}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        className={
          "relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 pr-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (density === "compact" ? "py-1.5 " : "py-2 ") +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
        data-testid="rail-history-project"
      >
        {selected && <SelectedBar />}
        <ProjectMark projectId={project.id} size={16} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-baseline gap-1.5">
            <span className={"min-w-0 truncate text-meta font-medium " + (project.missing ? "text-muted" : "text-ink")}>{project.name}</span>
            {project.pinned && <PinMark />}
            {project.missing && <span className="shrink-0 text-label font-semibold text-muted">moved</span>}
            {density === "compact" && when && <span className="ml-auto shrink-0 font-mono text-label text-muted">{when === "just now" ? "now" : when.replace(" ago", "")}</span>}
          </span>
          {density === "comfortable" && stats.length > 0 && (
            <span className="min-w-0 truncate text-label text-muted" data-testid="rail-project-stats">
              {stats.join(" · ")}
            </span>
          )}
          <span className="sr-only">{project.pinned ? ", pinned" : ""}</span>
        </span>
        <AccountStack accounts={accounts} size={14} />
      </button>
      {/* The accounts are its description, outside it: its name stays
          the project's (AJ-E1532). */}
      {accounts.length > 0 && (
        <span id={accountsId} className="sr-only">
          {accountsWords(accounts)}
        </span>
      )}
      {menu.button({ label: `Project actions for ${project.name}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/** A project's group heading in the rail: opens the project, with its
 * menu on right-click, ⋯ or Shift+F10 (AJ-E1537). */
export function RailProjectHeader({
  project,
  rows,
  label,
  rollup,
  count,
  selected,
  onSelect,
  children,
}: {
  project: ProjectInfo;
  /** Its sessions shown in the rail. */
  rows: InstanceInfo[];
  label: string;
  rollup: string;
  count: number;
  selected: boolean;
  onSelect: () => void;
  children?: ReactNode;
}): JSX.Element {
  const menu = useItemMenu("project", project, label);
  const accounts = projectAccounts(project, rows, useConversations().all);
  const accountsId = useId();
  return (
    <div className="group relative flex min-w-0 flex-1 items-center" {...menu.triggerProps}>
      <button
        type="button"
        onClick={onSelect}
        title={rollup || undefined}
        aria-describedby={accounts.length > 0 ? accountsId : undefined}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        className={
          "relative flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md py-0.5 pl-1 pr-2.5 text-left text-meta font-semibold outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface text-ink" : "text-muted hover:text-ink")
        }
        data-testid="rail-project"
      >
        {selected && <SelectedBar />}
        <span className="flex min-w-0 items-center gap-1.5">
          <ProjectMark projectId={project.id} size={14} />
          <span className="min-w-0 truncate">{label}</span>
          {project.pinned && <PinMark />}
        </span>
        <span className="flex shrink-0 items-center gap-1.5">
          {children}
          <AccountStack accounts={accounts} size={14} />
          <span className="text-label font-normal text-muted">{count}</span>
        </span>
      </button>
      {/* The accounts are its description, outside it: its name stays
          the project's (AJ-E1532). */}
      {accounts.length > 0 && (
        <span id={accountsId} className="sr-only">
          {accountsWords(accounts)}
        </span>
      )}
      {menu.button({ label: `Project actions for ${label}`, className: "absolute right-6 top-1/2 -translate-y-1/2 bg-surface" })}
      {menu.element}
    </div>
  );
}

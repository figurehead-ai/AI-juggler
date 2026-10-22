/**
 * The rail's project rows: a project's group heading, and a project with
 * no session here yet. Built like a session row (RailRow) — the same
 * size, spacing, hover and selection (AJ-E1534: a filled row, the accent
 * bar and `aria-current` when its page is open). Each is named by the
 * project's name; its folder, home-relative, is only its description
 * (P24: never the absolute path).
 */
import type { JSX, ReactNode } from "react";
import { useItemMenu } from "./ItemMenu";
import { hueOf } from "./indicator";
import { IconPin, IconProjects } from "./icons";
import { homeRelative, projectStats } from "../lib/project-words";
import type { ProjectInfo } from "../lib/bridge";

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

/** A project with no session here yet: a folder mark tinted for the
 * project where a session shows its state, and what was done there where
 * a session shows its state line. */
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
  return (
    <div className="group relative" {...menu.triggerProps}>
      <button
        type="button"
        onClick={() => onSelect(project.id)}
        title={homeRelative(project.root)}
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
        <span aria-hidden="true" className="flex w-4 shrink-0 justify-center" style={{ color: `hsl(${hueOf(project.id)} 60% 66%)` }}>
          <IconProjects width={13} height={13} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-baseline gap-1.5">
            <span className={"min-w-0 truncate text-[12.5px] font-medium " + (project.missing ? "text-muted" : "text-ink")}>{project.name}</span>
            {project.pinned && <PinMark />}
            {project.missing && <span className="shrink-0 text-[9.5px] font-medium uppercase tracking-wide text-muted">moved</span>}
            {density === "compact" && when && <span className="ml-auto shrink-0 font-mono text-[10px] text-muted">{when === "just now" ? "now" : when.replace(" ago", "")}</span>}
          </span>
          {density === "comfortable" && stats.length > 0 && (
            <span className="min-w-0 truncate text-[11px] text-muted" data-testid="rail-project-stats">
              {stats.join(" · ")}
            </span>
          )}
          <span className="sr-only">{project.pinned ? ", pinned" : ""}</span>
        </span>
      </button>
      {menu.button({ label: `Project actions for ${project.name}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/** A project's group heading in the rail: opens the project, with its
 * menu on right-click, ⋯ or Shift+F10 (AJ-E1537). */
export function RailProjectHeader({
  project,
  label,
  rollup,
  count,
  selected,
  onSelect,
  children,
}: {
  project: ProjectInfo;
  label: string;
  rollup: string;
  count: number;
  selected: boolean;
  onSelect: () => void;
  children?: ReactNode;
}): JSX.Element {
  const menu = useItemMenu("project", project, label);
  return (
    <div className="group relative flex min-w-0 flex-1 items-center" {...menu.triggerProps}>
      <button
        type="button"
        onClick={onSelect}
        title={rollup || undefined}
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        className={
          "relative flex min-w-0 flex-1 items-baseline justify-between gap-2 rounded-md py-0.5 pl-1 pr-2.5 text-left text-[11.5px] font-semibold outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface text-ink" : "text-muted hover:text-ink")
        }
        data-testid="rail-project"
      >
        {selected && <SelectedBar />}
        <span className="flex min-w-0 items-baseline gap-1.5">
          <span className="min-w-0 truncate">{label}</span>
          {project.pinned && <PinMark />}
        </span>
        <span className="flex shrink-0 items-center gap-1.5">
          {children}
          <span className="text-[10px] font-normal text-faint">{count}</span>
        </span>
      </button>
      {menu.button({ label: `Project actions for ${label}`, className: "absolute right-6 top-1/2 -translate-y-1/2 bg-surface" })}
      {menu.element}
    </div>
  );
}

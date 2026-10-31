/**
 * 0.3 — one project on the Projects screen: its mark, name and folder
 * (and when it was last worked), then what needs you, its sessions, its
 * open tasks and this week's tokens, on one grid every row shares. Where
 * it's narrow the facts go under the name.
 */
import type { JSX, ReactNode } from "react";
import { Pill } from "../../components/Pill";
import { ProjectMark } from "../../components/ProjectMark";
import { exact, short } from "../../components/charts/scale";
import { homeRelative, lastWorked } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { needChips, openTasks, openWords, taskWords, type ProjectNeeds, type ProjectOpen } from "../../lib/project-summary";
import type { TaskCounts } from "../../lib/ipc/tasks";
import type { ProjectInfo } from "../../lib/bridge";

/** The one grid: wide, five columns; narrow, the name over its facts two
 * by two. */
const GRID = "grid grid-cols-2 gap-x-4 gap-y-1.5 @min-[760px]:grid-cols-[minmax(0,2.2fr)_minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,1fr)_6.5rem] @min-[760px]:items-center";

export function ProjectRowsHead(): JSX.Element {
  return (
    <div className={GRID + " hidden border-b border-hairline px-3 pb-2 text-meta text-muted @min-[760px]:grid"} aria-hidden="true">
      <span>Project</span>
      <span>Needs you</span>
      <span>Sessions</span>
      <span>Tasks</span>
      <span className="text-right">This week</span>
    </div>
  );
}

/** A fact's name where it's narrow ("Sessions"), its column head wide. */
function Fact({ name, children, className = "" }: { name: string; children: ReactNode; className?: string }): JSX.Element {
  return (
    <span className={"flex min-w-0 items-center gap-1.5 text-body " + className}>
      <span className="text-meta text-muted @min-[760px]:sr-only">{name}</span>
      {children}
    </span>
  );
}

export function ProjectRow({
  project,
  now,
  needs,
  open,
  tasks,
  tokens,
  onOpen,
}: {
  project: ProjectInfo;
  now: number;
  needs: ProjectNeeds | undefined;
  open: ProjectOpen | undefined;
  tasks: TaskCounts | null;
  /** This week's tokens in and out; `null` while they're read. */
  tokens: number | null;
  onOpen: () => void;
}): JSX.Element {
  const chips = needChips(needs);
  const last = lastWorked(project);
  const work = openTasks(tasks);
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        data-project-row
        data-testid="project-row"
        data-project-id={project.id}
        className={GRID + " w-full rounded-lg px-3 py-3 text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"}
      >
        <span className="col-span-2 flex min-w-0 items-center gap-3 @min-[760px]:col-span-1">
          <ProjectMark projectId={project.id} size={28} />
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-body font-semibold text-ink">{project.name}</span>
            <span className="truncate text-meta text-muted" title={project.root}>
              <span className="font-mono text-label">{homeRelative(project.root)}</span>
              {last > 0 && <span> · {agoWords(last, now)}</span>}
              {project.missing && <span className="text-danger"> · folder missing</span>}
            </span>
          </span>
        </span>
        <Fact name="Needs you">
          {chips.length > 0 ? (
            <span className="flex min-w-0 flex-wrap items-center gap-1" data-testid="project-row-needs">
              {chips.map((chip) => (
                <Pill key={chip.key} tone={chip.tone}>
                  {chip.words}
                </Pill>
              ))}
            </span>
          ) : (
            <span className="text-muted">Nothing waiting</span>
          )}
        </Fact>
        <Fact name="Sessions">
          <span className={"truncate " + (open && open.open > 0 ? "text-ink" : "text-muted")} data-testid="project-row-open">
            {open && open.working > 0 && <span aria-hidden="true" className="mr-1.5 inline-block size-2 rounded-full bg-live align-middle motion-safe:animate-pulse" />}
            {openWords(open)}
          </span>
        </Fact>
        <Fact name="Tasks">
          <span className={"truncate " + (work && work.open > 0 ? "text-ink" : "text-muted")} data-testid="project-row-tasks">
            {taskWords(tasks)}
          </span>
        </Fact>
        <Fact name="This week" className="@min-[760px]:justify-end">
          <span className="tabular-nums text-ink" title={tokens === null ? undefined : exact(tokens, "tokens") + " in and out this week"} data-testid="project-row-tokens">
            {tokens === null ? "…" : tokens > 0 ? short(tokens, "tokens") : <span className="text-muted">None</span>}
          </span>
        </Fact>
      </button>
    </li>
  );
}

/** A row's shape while the projects are read. */
export function ProjectRowSkeleton(): JSX.Element {
  return (
    <li className={GRID + " px-3 py-3"} aria-hidden="true">
      <span className="col-span-2 flex items-center gap-3 @min-[760px]:col-span-1">
        <span className="size-7 rounded-lg bg-skeleton motion-safe:animate-pulse" />
        <span className="flex flex-col gap-1.5">
          <span className="h-3.5 w-36 rounded bg-skeleton motion-safe:animate-pulse" />
          <span className="h-3 w-48 rounded bg-skeleton motion-safe:animate-pulse" />
        </span>
      </span>
      <span className="hidden h-3.5 w-24 rounded bg-skeleton motion-safe:animate-pulse @min-[760px]:block" />
      <span className="hidden h-3.5 w-20 rounded bg-skeleton motion-safe:animate-pulse @min-[760px]:block" />
      <span className="hidden h-3.5 w-20 rounded bg-skeleton motion-safe:animate-pulse @min-[760px]:block" />
      <span className="hidden h-3.5 w-12 justify-self-end rounded bg-skeleton motion-safe:animate-pulse @min-[760px]:block" />
    </li>
  );
}

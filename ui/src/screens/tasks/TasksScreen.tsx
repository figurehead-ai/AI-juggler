/**
 * 0.3 (the redesign, §4) — Tasks: every project's and pool's tasks in one
 * place. Board (Ready · In progress · In review · Blocked) or Plan (each
 * pool's epics → stories → tasks), filtered by project, pool and holder,
 * searched by title. A task opens beside the board with its own actions.
 *
 * D-44: agents write the tasks; you see and steer them here. Nothing here
 * makes or edits a task; the actions are the pool's own task view's
 * (Approve, Send back…, Free it, Cancel task, Start a session for it…).
 * A task in review stays in Needs you too.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { Page } from "../../components/Page";
import { Picker, Segmented, type PickerOption } from "../../components/controls";
import { IconPool } from "../../components/icons-actions";
import { IconProfiles, IconProjects } from "../../components/icons";
import { IconClock } from "../../components/icons-menu";
import { IconUser } from "../../components/usage/controls";
import { useActionHost } from "../../components/ItemMenu";
import { refreshPaged } from "../../lib/paged";
import { usePools } from "../../lib/pools";
import { onTasksChanged, type TaskCounts } from "../../lib/ipc/tasks";
import { tasksAcrossCounts, type TasksFilterInput } from "../../lib/ipc/tasks-across";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import { TaskView } from "../../pages/tasks/TaskView";
import { Board, Plan, type OpenTask } from "./TaskBoard";

const VIEW_KEY = "aij.tasks.view";

function storedView(): "board" | "plan" {
  try {
    return localStorage.getItem(VIEW_KEY) === "plan" ? "plan" : "board";
  } catch {
    return "board";
  }
}

/** "All" in a picker: no filter. */
const ALL = "all";
const HOLDERS: PickerOption[] = [
  { value: ALL, label: "Anyone", hint: "Whoever has it, or no one", icon: IconProfiles },
  { value: "nobody", label: "No one yet", hint: "Work no agent has taken", icon: IconClock },
  { value: "you", label: "You", hint: "Tasks an agent gave you", icon: IconUser },
];
const orAll = (value: string) => (value === ALL ? null : value);

export function TasksScreen({ projects, sessions, now }: { projects: ProjectInfo[]; sessions: InstanceInfo[]; now: number }): JSX.Element {
  const host = useActionHost();
  const pools = usePools()?.pools ?? [];
  const [view, setView] = useState(storedView);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [poolId, setPoolId] = useState<string | null>(null);
  const [holder, setHolder] = useState<string | null>(null);
  const [words, setWords] = useState("");
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<OpenTask | null>(null);
  const [counts, setCounts] = useState<TaskCounts | null>(null);
  // What's typed is looked for once typing pauses.
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(words.trim()), words ? 200 : 0);
    return () => window.clearTimeout(timer);
  }, [words]);
  // The plan's epics and stories have no holder: its filter is the board's.
  const holding = view === "board" ? holder : null;
  const filter = useMemo<TasksFilterInput>(
    () => ({ ...(projectId && { project_id: projectId }), ...(poolId && { pool_id: poolId }), ...(holding && { holder: holding }), ...(query && { words: query }) }),
    [projectId, poolId, holding, query],
  );
  const filterKey = JSON.stringify(filter);
  // Every list shown reads again when tasks change (their keys start
  // with `tasks`), and so do the counts.
  useEffect(() => onTasksChanged(() => refreshPaged("tasks")), []);
  useEffect(() => {
    let alive = true;
    const read = () => void tasksAcrossCounts(filter).then((next) => alive && setCounts(next), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterKey]);
  // Esc closes the open task (when nothing inside it took the key).
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[aria-modal="true"]')) return;
      setOpen(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const names = useMemo(() => new Map(projects.map((project) => [project.id, project.name])), [projects]);
  const projectName = (id: string | null) => (id ? (names.get(id) ?? null) : null);
  const poolOptions = pools
    .filter((pool) => !projectId || pool.project_id === projectId)
    .map((pool): PickerOption => ({ value: pool.id, label: pool.name, hint: pool.members === 1 ? "1 member" : `${pool.members} members`, icon: IconPool }));
  const projectOptions: PickerOption[] = [{ value: ALL, label: "All", hint: "Every project", icon: IconProjects }, ...projects.map((project): PickerOption => ({ value: project.id, label: project.name, icon: IconProjects }))];
  const chooseView = (next: "board" | "plan") => {
    setView(next);
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // This visit only.
    }
  };
  const shared = { filter, filterKey, open, projectName, onOpen: setOpen, onOpenProject: (id: string) => host.openProjectById?.(id) };
  const defaultAccount = projects.find((project) => project.id === open?.projectId)?.default_account_id ?? null;
  const total = counts ? counts.ready + counts.in_progress + counts.in_review + counts.blocked : null;
  return (
    <Page
      title="Tasks"
      subtitle={total === null ? "What the agents are working on, in every project." : `${total} open ${total === 1 ? "task" : "tasks"} across your projects. The agents write them; you follow and review.`}
      testId="tasks-screen"
    >
      <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Show">
        <Segmented
          label="View"
          segments={[
            { value: "board", label: "Board", title: "By state: ready, in progress, in review, blocked" },
            { value: "plan", label: "Plan", title: "Each pool's epics, stories and tasks" },
          ]}
          value={view}
          onChange={chooseView}
          testId="tasks-view"
          className="w-36 shrink-0"
        />
        <Picker size="sm" prefix="Project:" ariaLabel="Project" heading="Project" value={projectId ?? ALL} options={projectOptions} onChange={(next) => (setProjectId(orAll(next)), setPoolId(null))} testId="tasks-filter-project" />
        <Picker
          size="sm"
          prefix="Pool:"
          ariaLabel="Pool"
          heading="Pool"
          value={poolId ?? ALL}
          options={[{ value: ALL, label: "All", hint: projectId ? "Every pool of this project" : "Every pool", icon: IconPool }, ...poolOptions]}
          onChange={(next) => setPoolId(orAll(next))}
          testId="tasks-filter-pool"
        />
        {view === "board" && <Picker size="sm" prefix="Holder:" ariaLabel="Holder" heading="Holder" value={holder ?? ALL} options={HOLDERS} onChange={(next) => setHolder(orAll(next))} testId="tasks-filter-holder" />}
        <input
          type="search"
          value={words}
          onChange={(event) => setWords(event.target.value)}
          placeholder="Search tasks"
          aria-label="Search tasks"
          className="ml-auto h-8 w-56 max-w-full rounded-lg border border-hairline bg-surface px-3 text-body text-ink placeholder:text-muted outline-none focus:border-accent/60"
          data-testid="tasks-search"
        />
      </div>
      <div className={"grid min-w-0 items-start gap-5 " + (open ? "lg:grid-cols-[minmax(0,1fr)_minmax(340px,420px)]" : "")}>
        {view === "board" ? <Board counts={counts} narrow={open !== null} {...shared} /> : <Plan now={now} {...shared} />}
        {open && (
          <aside className="sticky top-0 min-w-0 rounded-xl border border-hairline bg-surface p-4 pr-10" aria-label="The open task" data-testid="tasks-open">
            <button type="button" onClick={() => setOpen(null)} aria-label="Close the task (Esc)" title="Close (Esc)" className="absolute right-2.5 top-2.5 rounded-lg p-1 text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="tasks-open-close">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
                  <path d="M6 6l12 12M18 6L6 18" />
                </svg>
            </button>
            <TaskView key={open.id} taskId={open.id} sessions={sessions} defaultAccountId={defaultAccount} now={now} onOpenTask={(id) => setOpen({ id, projectId: open.projectId })} />
          </aside>
        )}
      </div>
    </Page>
  );
}

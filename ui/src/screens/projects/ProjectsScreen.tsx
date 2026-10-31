/**
 * 0.3 (the redesign, §3) — Projects: every project the same way, the
 * most recently worked first — its name and folder, what needs you there,
 * its sessions working and open, its open tasks (ready, in review) and
 * this week's tokens. Searchable by name or folder; a page at a time.
 * Each row opens the project's page; New project (⌘⇧N) is on the title.
 *
 * Instant: the projects the window already holds draw at once; the needs
 * (Needs you's own list), the tasks (one call for the page shown) and the
 * tokens (this week's usage summary, read once) fill in as they come.
 */
import { useEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { Page } from "../../components/Page";
import { Button } from "../../components/primitives";
import { IconPlus } from "../../components/icons-actions";
import { useActionHost } from "../../components/ItemMenu";
import { useUsageHistory } from "../../components/usage/useUsage";
import { useBackendNeeds } from "../../lib/needs-you";
import { useConversations } from "../../lib/conversations";
import { lastWorked } from "../../lib/project-words";
import { needsByProject, openByProject } from "../../lib/project-summary";
import { onTasksChanged, type TaskCounts } from "../../lib/ipc/tasks";
import { tasksProjectCounts } from "../../lib/ipc/tasks-across";
import { keysOf } from "../../lib/palette-commands";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import { ProjectRow, ProjectRowsHead, ProjectRowSkeleton } from "./ProjectRow";

/** Rows a page shows (another page on Show more). */
export const PROJECTS_PAGE = 40;

/** The projects that match what's typed, the most recently worked first. */
export function projectsShown(projects: ProjectInfo[], words: string): ProjectInfo[] {
  const needle = words.trim().toLowerCase();
  return projects
    .filter((project) => !needle || project.name.toLowerCase().includes(needle) || project.root.toLowerCase().includes(needle))
    .sort((a, b) => lastWorked(b) - lastWorked(a) || a.name.localeCompare(b.name));
}

/** The page's tasks, read in one call, again when tasks change. */
function useProjectTasks(ids: string[]): Map<string, TaskCounts> {
  const [counts, setCounts] = useState<Map<string, TaskCounts>>(new Map());
  const key = ids.join("\n");
  useEffect(() => {
    if (!key) return;
    let alive = true;
    const read = () =>
      void tasksProjectCounts(key.split("\n")).then(
        (rows) => alive && setCounts((was) => new Map([...was, ...rows.map((row) => [row.project_id, row.counts] as const)])),
        () => undefined,
      );
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [key]);
  return counts;
}

export function ProjectsScreen({ projects, sessions, loading, now, onNewProject }: { projects: ProjectInfo[]; sessions: InstanceInfo[]; loading: boolean; now: number; onNewProject: () => void }): JSX.Element {
  const host = useActionHost();
  const [words, setWords] = useState("");
  const [pages, setPages] = useState(1);
  const listRef = useRef<HTMLUListElement | null>(null);
  const shown = useMemo(() => projectsShown(projects, words), [projects, words]);
  const page = shown.slice(0, pages * PROJECTS_PAGE);
  const tasks = useProjectTasks(page.map((project) => project.id));
  const needs = useBackendNeeds();
  const { all: conversations } = useConversations();
  const byNeeds = useMemo(() => needsByProject(needs.rows), [needs.rows]);
  const byOpen = useMemo(() => openByProject(sessions, conversations), [sessions, conversations]);
  const week = useUsageHistory("7d", "profile", {}, 0);
  const tokens = useMemo(() => new Map((week.data?.by_project ?? []).map((share) => [share.key, share.io])), [week.data]);
  useEffect(() => setPages(1), [words]);

  // ↑ and ↓ move between rows; Home and End to the first and last.
  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    const rows = [...(listRef.current?.querySelectorAll<HTMLElement>("[data-project-row]") ?? [])];
    const at = rows.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    const next = event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? rows.length - 1 : null;
    if (next === null) return;
    event.preventDefault();
    rows[Math.max(0, Math.min(rows.length - 1, next))]?.focus();
  };

  const count = projects.length;
  return (
    <Page
      title="Projects"
      subtitle={count === 0 ? "Each folder you work in with an AI tool is a project." : `${count} ${count === 1 ? "project" : "projects"}, the most recently worked first.`}
      testId="projects-screen"
      actions={
        <>
          <input
            type="search"
            value={words}
            onChange={(event) => setWords(event.target.value)}
            placeholder="Search projects by name or folder"
            aria-label="Search projects"
            className="h-9 w-72 max-w-full rounded-lg border border-hairline bg-surface px-3 text-body text-ink placeholder:text-muted outline-none focus:border-accent/60"
            data-testid="projects-search"
          />
          <Button size="sm" variant="primary" onClick={onNewProject} title={`New project (${keysOf("new-project")})`} data-testid="projects-new">
            <IconPlus width={14} height={14} aria-hidden="true" />
            New project
          </Button>
        </>
      }
    >
      <div className="@container flex flex-col">
        <ProjectRowsHead />
        {loading && projects.length === 0 ? (
          <ul className="flex flex-col" aria-label="Loading projects" aria-busy="true">
            {Array.from({ length: 5 }, (_, n) => (
              <ProjectRowSkeleton key={n} />
            ))}
          </ul>
        ) : shown.length === 0 ? (
          <p className="rounded-lg border border-dashed border-hairline px-4 py-6 text-center text-body text-ink-2" data-testid="projects-empty">
            {words.trim() ? `No project matches “${words.trim()}”.` : "No projects yet. Start a session in a folder, or make a project, and it shows here with what needs you, its sessions and its tasks."}
          </p>
        ) : (
          <ul ref={listRef} className="flex flex-col divide-y divide-hairline/70" aria-label="Projects" onKeyDown={onKeyDown} data-testid="projects-list">
            {page.map((project) => (
              <ProjectRow
                key={project.id}
                project={project}
                now={now}
                needs={byNeeds.get(project.id)}
                open={byOpen.get(project.id)}
                tasks={tasks.get(project.id) ?? null}
                tokens={week.data ? (tokens.get(project.id) ?? 0) : null}
                onOpen={() => host.openProjectById?.(project.id)}
              />
            ))}
          </ul>
        )}
        {shown.length > page.length && (
          <Button size="sm" variant="ghost" className="mt-3 self-center" onClick={() => setPages((n) => n + 1)} data-testid="projects-more">
            Show more projects ({shown.length - page.length} more)
          </Button>
        )}
      </div>
    </Page>
  );
}

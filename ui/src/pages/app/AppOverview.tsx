/**
 * D-62 (AJ-E3204) — an App's Overview and its Projects tab. The Overview
 * answers "this app, now": what's open in it, what was last open (once
 * something has ended), how
 * much room its account has (and who shares it), and where it works. Its
 * sessions are the rail's under this App, drawn with the rows every
 * session list uses.
 */
import type { JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { Button, Card } from "../../components/primitives";
import { signedIn } from "../../lib/bridge";
import { IconFolder, IconPlay } from "../../components/usage/controls";
import { IconHistory } from "../../components/icons-places";
import { ListRow } from "../SessionRow";
import { conversationRow, sessionRow, type Row } from "../SessionsList";
import { AccountLimits } from "../AccountPage";
import { FindProfiles } from "./FindProfiles";
import { showTab } from "../PageShell";
import type { ActionHost } from "../../lib/action-host";
import type { AppModel } from "../../lib/apps";
import { openHere, type AppProject } from "../../lib/app-items";
import type { RailItem } from "../../lib/rail-items";
import { agoWords } from "../../lib/time-words";

/** Inactive sessions the Overview shows before "All sessions". */
const RECENT_SHOWN = 5;
/** Projects it shows before "All projects". */
const PROJECTS_SHOWN = 5;

/** A rail item as the shared session row; `app`, the App it belongs to
 * when the caller knows it (its icon and "in …" then say that App). */
export function rowOf(item: RailItem, now: number, onOpenSession: (id: string) => void, host: ActionHost, app?: AppModel | null): Row | null {
  const row = item.session ? sessionRow(item.session, now, onOpenSession) : item.conversation ? conversationRow(item.conversation, now, host) : null;
  if (!row) return null;
  // The rail's own item: its App, its project and the backend's Needs you.
  row.rail = item;
  if (app) row.app = { appId: app.appId, appPid: app.place?.app_pid ?? null, name: app.name };
  return row;
}

export function AppOverview({
  app,
  items,
  projects,
  sharedWith,
  now,
  host,
  onOpenSession,
  onOpenProject,
}: {
  app: AppModel;
  items: RailItem[];
  projects: AppProject[];
  /** The other Apps on its account (its room is theirs too). */
  sharedWith: AppModel[];
  now: number;
  host: ActionHost;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
}): JSX.Element {
  const open = items.filter(openHere);
  const recent = items.filter((item) => !openHere(item)).slice(0, RECENT_SHOWN);
  const rows = (list: RailItem[]): Shown[] =>
    list.flatMap((item) => {
      const row = rowOf(item, now, onOpenSession, host);
      return row ? [{ row, project: item.projectId ? { id: item.projectId, name: item.project } : null }] : [];
    });
  const folder = app.folder;
  const folderOut = folder !== null && !signedIn(folder.login_state);
  // Walk A1: a desktop App (a Claude app profile) signs in inside its
  // app; its folder's terminal sign-in, never set up there, doesn't make
  // it "signed out". Its limits show once that folder reads them.
  const signedOut = folderOut && app.kind !== "desktop";
  if (items.length === 0) {
    // Nothing yet: one card that says what to do, not three empty ones.
    return (
      <div className="@container flex w-full max-w-content flex-col gap-grid" data-testid="app-overview">
        <FindProfiles key={app.id} app={app} />
        <Section title="No sessions yet" icon={<IconPlay />} testId="app-empty">
          <p className="text-body text-ink-2">
            {app.kind === "desktop"
              ? `Sessions you open in ${app.name} show here, with the projects it works in. Start it, then start a session in it.`
              : `Sessions you run with ${app.name} show here, with the projects it works in. New session starts one.`}
          </p>
        </Section>
        {signedOut ? <SignedOut name={app.name} /> : folder && !folderOut && <AccountLimits account={folder} now={now} />}
      </div>
    );
  }
  return (
    <div className="@container flex w-full max-w-content flex-col gap-grid" data-testid="app-overview">
      <FindProfiles key={app.id} app={app} />
      {signedOut && <SignedOut name={app.name} />}
      {/* The 0.4 review: Open now's count is its rows (not a sub-line
          again), and Last used shows only once something has ended;
          until then Open now takes the whole width (47, 78). */}
      <div className={"grid grid-cols-1 gap-grid" + (recent.length > 0 ? " @min-[900px]:grid-cols-2" : "")}>
        <Section title="Open now" icon={<IconPlay />} testId="app-open">
          <RowList rows={rows(open)} empty={app.kind === "desktop" ? `Nothing open in ${app.name} now.` : `Nothing running on ${app.name} now.`} />
        </Section>
        {recent.length > 0 && (
          <Section
            title="Last used"
            icon={<IconHistory />}
            actions={items.length > open.length + recent.length ? <MoreLink label="All sessions" onOpen={() => showTab("sessions")} testId="app-all-sessions" /> : undefined}
            testId="app-recent"
          >
            <RowList rows={rows(recent)} empty="" />
          </Section>
        )}
      </div>
      {folder && !folderOut && (
        <div className="flex flex-col gap-2" data-testid="app-room">
          <AccountLimits account={folder} now={now} />
          {sharedWith.length > 0 && (
            <p className="text-meta text-ink-2" data-testid="app-room-shared">
              Its account's room is shared with {sharedWith.map((other) => other.name).join(", ")}.
            </p>
          )}
        </div>
      )}
      <Section
        title="Projects"
        icon={<IconFolder />}
        sub="where it works, most recent first"
        actions={projects.length > PROJECTS_SHOWN ? <MoreLink label="All projects" onOpen={() => showTab("projects")} testId="app-all-projects" /> : undefined}
        testId="app-projects-top"
      >
        <ProjectRows projects={projects.slice(0, PROJECTS_SHOWN)} now={now} onOpenProject={onOpenProject} />
      </Section>
    </div>
  );
}

/** A folder that isn't signed in: nothing to measure, and where to sign
 * in (nothing is deleted). */
function SignedOut({ name }: { name: string }): JSX.Element {
  return (
    <Card className="flex flex-wrap items-center justify-between gap-3 p-4" data-testid="app-signed-out">
      <span className="flex flex-col gap-0.5">
        <span className="text-body font-medium text-ink">{name} is signed out</span>
        <span className="text-meta text-ink-2">Its limits and usage show again once it's signed in. Nothing is deleted.</span>
      </span>
      <Button size="sm" variant="primary" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))}>
        Sign in from Accounts
      </Button>
    </Card>
  );
}

/** The Projects tab: every project the App worked in. */
export function AppProjects({ projects, sessions, now, onOpenProject }: { projects: AppProject[]; sessions: number; now: number; onOpenProject: (id: string) => void }): JSX.Element {
  if (sessions === 0) return <p className="text-meta text-ink-2" data-testid="app-projects">No sessions yet, so no projects either.</p>;
  return (
    <div className="flex w-full max-w-content flex-col gap-2" data-testid="app-projects">
      <p className="text-meta text-ink-2">{projects.length === 1 ? "1 project" : `${projects.length} projects`}</p>
      <div className="overflow-hidden rounded-xl border border-hairline bg-surface px-1 py-1">
        <ProjectRows projects={projects} now={now} onOpenProject={onOpenProject} />
      </div>
    </div>
  );
}

export function ProjectRows({ projects, now, onOpenProject }: { projects: AppProject[]; now: number; onOpenProject: (id: string) => void }): JSX.Element {
  if (projects.length === 0) return <p className="px-2 py-1.5 text-meta text-ink-2">No project yet: its sessions so far ran outside one.</p>;
  return (
    <ul className="flex flex-col">
      {projects.map((project) => (
        <li key={project.id}>
          <button
            type="button"
            onClick={() => onOpenProject(project.id)}
            className="flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="app-project"
          >
            <IconFolder className="h-4 w-4 shrink-0 text-muted" aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{project.name}</span>
            <span className="shrink-0 text-meta text-ink-2">
              {project.sessions === 1 ? "1 session" : `${project.sessions} sessions`} · {agoWords(project.lastMs, now)}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

/** A row and the project it names. */
export type Shown = { row: Row; project: { id: string; name: string | null } | null };

export function RowList({ rows, empty, within = "app" }: { rows: Shown[]; empty: string; within?: "app" | "account" }): JSX.Element {
  if (rows.length === 0) return <p className="text-meta text-ink-2">{empty}</p>;
  return (
    <ul className="-mx-2 flex flex-col">
      {rows.map(({ row, project }) => (
        <li key={row.key} className="border-t border-hairline/60 first:border-t-0">
          <ListRow row={row} project={project} inApp={within === "app"} />
        </li>
      ))}
    </ul>
  );
}

export function MoreLink({ label, onOpen, testId }: { label: string; onOpen: () => void; testId: string }): JSX.Element {
  return (
    <button type="button" onClick={onOpen} className="rounded text-meta text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid={testId}>
      {label} <span aria-hidden="true">›</span>
    </button>
  );
}

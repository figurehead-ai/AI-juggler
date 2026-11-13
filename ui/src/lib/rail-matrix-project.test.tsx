/**
 * The rail matrix, Project lens. `buildLines` doesn't build this lens (it
 * reads "project" as the App grouping, rail-lines.ts:204-205); the rail
 * draws the project board (`InstanceBoard`), so the rows render it.
 *
 * Spec: ADR-0032 §2 ("each lens groups strictly by its own entity … Project
 * by Project"); E21 AJ-E2103 req. 1-2 (Needs you, Open now, then the Also
 * line; ended sessions from other apps are not added under every project);
 * D-64 (Inactive apps is the App grouping's); the same archived rule and
 * search rule as the App lens (ASSUMED here: the spec states them for Apps).
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { InstanceBoard } from "../components/WorkflowRail";
import { safeId } from "../components/rail-groups";
import { ConversationsProvider } from "./conversations";
import { needsFor } from "../test/needs-kit";
import { DEFAULT_VIEW, resetRailViewCache, setRailView, type RailView } from "./rail-view";
import { claudeApp, cliApp, H, NOW, open, past, PROJECTS } from "./rail-matrix.test-kit";
import type { ConversationInfo } from "./ipc/conversations";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const EAST = claudeApp("East", 41);
const CLI = cliApp("claude-code", "f-cli", "Side");

/** One of AI Juggler's own sessions. */
function own(id: string, projectId: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "r-1", adopted: false, provider: "claude-code", account_id: "f-cli", account_label: "Side", label: `Own ${id}`,
    model: "opus", launch_model: "opus", running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null,
    child_count: null, subtask_count: null, git_branch: "feature", git_dirty_count: null, needs_attention: null, attention: null,
    started_at_ms: NOW - H, last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/w/app", kind: "terminal", isolation_label: null, project_id: projectId, conversation_id: null, ...over,
  };
}

interface ProjectRow {
  name: string;
  query?: string;
  only?: RailView["only"];
  sessions: InstanceInfo[];
  talks: ConversationInfo[];
  /** Each project group drawn and the session ids under it (any order). */
  expect: Record<string, string[]>;
  mismatch?: string;
}

/** Open: own s-api and s-web; another app's o-api (East) and o-web (a terminal). Past: own s-old, another app's x-api. */
const SESSIONS = [own("s-api", "p-api", { label: "Fix api" }), own("s-web", "p-web", { label: "Docs pass" }), own("s-old", "p-api", { status: "finished", last_activity_ms: NOW - 30 * H })];
const TALKS = [
  open("o-api", EAST, { project_id: "p-api", title: "Notes" }),
  open("o-web", CLI, { project_id: "p-web", title: "Fix web" }),
  past("x-api", EAST, 2, { project_id: "p-api", title: "Fix old" }),
];

const ROWS: ProjectRow[] = [
  {
    name: "P1 no search: each project with something open, holding only its own open sessions, AI Juggler's and other apps'",
    sessions: SESSIONS, talks: TALKS,
    expect: { "p-api": ["s-api", "o-api"], "p-web": ["s-web", "o-web"] },
  },
  {
    // ASSUMED: a search shows every open match under its own project (the App lens's rule).
    // 0.4.4 (filed from the 0.4.3 audit): other apps' sessions were dropped by any typed text.
    name: "P2 search: every open match under its own project, other apps' too",
    query: "fix", sessions: SESSIONS, talks: TALKS,
    expect: { "p-api": ["s-api"], "p-web": ["o-web"] },
  },
  {
    // ASSUMED: Only › Project narrows to that project, its other apps' sessions too.
    // 0.4.4 (filed from the 0.4.3 audit): other apps' sessions were dropped by Only › Project.
    name: "P3 Only project api: api's open sessions, AI Juggler's and other apps'; nothing of web's",
    // (No ended own session here: under a filter the board lists those too, and the spec says nothing of it.)
    only: { project: "api" }, sessions: SESSIONS.slice(0, 2), talks: TALKS,
    expect: { "p-api": ["s-api", "o-api"] },
  },
  {
    // ASSUMED: Only › Account narrows other apps' sessions as it does AI Juggler's own (0.4.4, with P2 and P3).
    name: "P5 Only account Side: Side's open sessions, AI Juggler's and other apps'; none of East's",
    only: { account: "Side" }, sessions: SESSIONS, talks: TALKS,
    expect: { "p-api": ["s-api"], "p-web": ["s-web", "o-web"] },
  },
  {
    // RULED as App lens 7g (0.4.3): what runs shows, whatever its archive marks.
    name: "P4 an open session archived in its app or here still shows under its project: it runs",
    sessions: [own("s-api", "p-api")], talks: [open("o-api", EAST, { project_id: "p-api" }), open("o-arch", EAST, { project_id: "p-api", app_archived: true }), open("o-here", CLI, { project_id: "p-api", archived_here: true })],
    expect: { "p-api": ["s-api", "o-api", "o-arch", "o-here"] },
  },
];

afterEach(() => {
  cleanup();
  localStorage.clear();
  setRailView(DEFAULT_VIEW);
  resetRailViewCache();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
});

/** The project board for a row, and what each project group holds. */
async function board(row: ProjectRow): Promise<{ groups: Record<string, string[]>; text: string }> {
  resetRailViewCache();
  setRailView({ ...DEFAULT_VIEW, group: "project", only: row.only ?? {} });
  Object.assign(window, { __AIJ_FIXTURE__: { conversations: row.talks } });
  needsFor({ sessions: row.sessions, conversations: row.talks });
  const view = render(
    <ConversationsProvider>
      <InstanceBoard sessions={row.sessions} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={PROJECTS} snooze={{}} onSnooze={() => {}} onSelectProject={() => {}} />
    </ConversationsProvider>,
  );
  await view.findByTestId("instance-board");
  if (row.query) fireEvent.change(view.getByRole("searchbox"), { target: { value: row.query } });
  // Other apps' sessions arrive with the conversations' first read.
  const talks = new Set(row.talks.map((talk) => talk.id));
  if (Object.values(row.expect).flat().some((id) => talks.has(id))) await view.findAllByTestId("rail-outside");
  const groups: Record<string, string[]> = {};
  for (const { id } of PROJECTS) {
    const box = document.getElementById(safeId("rail-group", id));
    if (box) groups[id] = [...box.querySelectorAll<HTMLElement>("[data-instance-id], [data-conversation-id]")].map((el) => el.dataset.instanceId ?? el.dataset.conversationId ?? "?");
  }
  return { groups, text: view.getByTestId("instance-board").textContent ?? "" };
}

async function check(row: ProjectRow): Promise<void> {
  const { groups, text } = await board(row);
  const sorted = (record: Record<string, string[]>) => Object.fromEntries(Object.entries(record).map(([key, ids]) => [key, [...ids].sort()]));
  // Rows only under their own project; past ones never under a project (AJ-E2103 req. 2).
  const projectOf = new Map<string, string | null>([...row.sessions.map((s) => [s.id, s.project_id] as const), ...row.talks.map((t) => [t.id, t.project_id] as const)]);
  for (const [id, ids] of Object.entries(groups)) for (const one of ids) expect(projectOf.get(one), `${one} under ${id}`).toBe(id);
  // D-64's sections are the App grouping's.
  expect(text).not.toContain("Inactive apps");
  expect(text).not.toContain("Running, not added");
  expect(sorted(groups)).toEqual(sorted(row.expect));
}

describe("rail matrix: Project lens (the project board)", () => {
  it.each(ROWS.filter((row) => !row.mismatch))("$name", async (row) => check(row));
  it.fails.each(ROWS.filter((row) => row.mismatch))("$name (MISMATCH)", async (row) => check(row));
});

/**
 * The 0.4.6 RC walk K2 (F-K2-5): on a project's History tab, with App =
 * Codex CLI chosen, the Tool menu still offered Claude Code, which would
 * show nothing; the menus said no counts, and the App menu offered two
 * desktop Apps with no session. History's filters are now the related
 * lists' chips: each menu offers what the backend's `sessions_facets`
 * counts with the other filters chosen, each value with its count, and
 * the list is narrowed by App and tool themselves (never by the tool's
 * name as search words).
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Page, PageRequest } from "../lib/paged";
import type { SessionRowInfo, SessionsFacetsInfo, SessionsQuery } from "../lib/ipc/sessions";
import type { FacetFilters } from "../types/generated/FacetFilters";

vi.mock("../lib/bridge", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/bridge")>()),
  hasLiveBackend: () => true,
}));

const pages: SessionsQuery[] = [];
const facetsAsked: { query: SessionsQuery; filters: FacetFilters | null }[] = [];

/** The K2 lab's api project: Codex 167, Work 167, Personal 166; the
 * desktop Apps ran none there. Each kind counted with the others chosen. */
function facetsOf(filters: FacetFilters | null): SessionsFacetsInfo {
  const codex = filters?.app_id === "cli:codex";
  const claude = filters?.tool_id === "claude-code";
  return {
    projects: [],
    apps: claude ? [{ id: "cli:work", count: 167 }, { id: "cli:personal", count: 166 }] : [{ id: "cli:codex", count: 167 }, { id: "cli:work", count: 167 }, { id: "cli:personal", count: 166 }],
    subscriptions: [],
    machines: [{ id: "this-mac", count: codex ? 167 : 500 }],
    tools: codex ? [{ id: "codex", count: 167 }] : [{ id: "claude-code", count: 333 }, { id: "codex", count: 167 }],
  };
}

vi.mock("../lib/ipc/sessions", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/sessions")>()),
  sessionsPage: async (query: SessionsQuery, _page: PageRequest): Promise<Page<SessionRowInfo>> => {
    pages.push(query);
    return { items: [], next_cursor: null, total: 0 };
  },
  sessionsCounts: async () => ({ all: 500, open: 0, ended: 500, archived: 0 }),
  sessionsFacets: async (query: SessionsQuery, filters: FacetFilters | null = null) => {
    facetsAsked.push({ query, filters });
    return facetsOf(filters);
  },
}));

const { History } = await import("./History");
const { forgetPaged } = await import("../lib/paged");
const { forgetFacets } = await import("../components/related/useFacets");
const { genericTool, setToolsForTests } = await import("../lib/tools");
// The app loads the tools' catalog before its first render.
setToolsForTests([
  { ...genericTool("claude-code"), display_name: "Claude Code" },
  { ...genericTool("codex"), display_name: "Codex" },
]);

afterEach(() => {
  cleanup();
  forgetPaged();
  forgetFacets();
  pages.length = 0;
  facetsAsked.length = 0;
});

describe("History's filters (F-K2-5)", () => {
  it("each menu offers what the others' choices leave, with its count", async () => {
    render(<History sessions={[]} projects={[]} projectId="proj-api" onChanged={() => {}} />);
    // A project's own tab: no Project filter; App and Tool with counts.
    await waitFor(() => expect(screen.getByTestId("history-filter-app")).toBeTruthy());
    expect(screen.queryByTestId("history-filter-project")).toBeNull();
    expect(facetsAsked[0]!.query.project_id).toBe("proj-api");
    fireEvent.click(screen.getByTestId("history-filter-tool"));
    expect((await screen.findByTestId("history-filter-tool-claude-code")).textContent).toContain("Claude Code · 333");
    expect(screen.getByTestId("history-filter-tool-codex").textContent).toContain("Codex · 167");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    // App = Codex CLI: the Tool menu is counted with it chosen, so it
    // offers Codex alone, and can't narrow anything: it isn't offered.
    fireEvent.click(screen.getByTestId("history-filter-app"));
    expect((await screen.findByTestId("history-filter-app-cli:codex")).textContent).toContain("· 167");
    fireEvent.click(screen.getByTestId("history-filter-app-cli:codex"));
    await waitFor(() => expect(facetsAsked.some((asked) => asked.filters?.app_id === "cli:codex")).toBe(true));
    await waitFor(() => expect(screen.queryByTestId("history-filter-tool")).toBeNull());
    // The list narrowed by the App itself.
    await waitFor(() => expect(pages.at(-1)).toMatchObject({ project_id: "proj-api", app_id: "cli:codex", tool_id: null, text: null }));
  });

  it("a tool chosen narrows the list by the tool, never by its name as search words", async () => {
    render(<History sessions={[]} projects={[]} onChanged={() => {}} />);
    fireEvent.click(await screen.findByTestId("history-filter-tool"));
    fireEvent.click(await screen.findByTestId("history-filter-tool-claude-code"));
    await waitFor(() => expect(pages.at(-1)).toMatchObject({ tool_id: "claude-code", text: null }));
    // The App menu, counted with the tool chosen: Codex isn't offered.
    fireEvent.click(screen.getByTestId("history-filter-app"));
    await screen.findByTestId("history-filter-app-cli:work");
    expect(screen.queryByTestId("history-filter-app-cli:codex")).toBeNull();
    // Clear filters takes them all off.
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    fireEvent.click(screen.getByTestId("history-filter-clear"));
    await waitFor(() => expect(pages.at(-1)).toMatchObject({ tool_id: null, app_id: null }));
  });
});

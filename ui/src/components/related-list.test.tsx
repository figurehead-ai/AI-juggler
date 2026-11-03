/**
 * E31 (AJ-E3103) vitest — the one related list, on the project, App and
 * Account pages (the design preview's rows, narrowed by the same rules as
 * the backend's):
 *
 * - K2 by all pairs (0.4 validation plan §3, L3): page × size (0, 1, 8,
 *   9, 50, 1 000) × filter (none, state, App, Account, project) × search,
 *   each case against an expectation written from the fixture's own rules
 *   (not the list's code): the count equals the list, the order is needs
 *   you, working, open, recent, rest, search and filters appear at 9;
 * - an empty list teaches, and an emptied one says how to undo it;
 * - the one row: the same places for the App, state, project and when.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RelatedList, type Anchor } from "./RelatedList";
import { ActionHostProvider } from "./ItemMenu";
import { sessionRow } from "../pages/SessionsList";
import { allPairs } from "../test/pairwise";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import type { SubscriptionInfo } from "../lib/subscriptions";

// Two Accounts: folders a and c on one, b on the other.
const SUBS = [
  { key: "a", account_id: "acct:a", display_name: "Work", profiles: [{ id: "acc-a" }, { id: "acc-c" }] },
  { key: "b", account_id: "acct:b", display_name: "Home", profiles: [{ id: "acc-b" }] },
] as unknown as SubscriptionInfo[];
vi.mock("../lib/subscriptions", async (actual) => ({ ...(await actual<typeof import("../lib/subscriptions")>()), useSubscriptions: () => SUBS }));

afterEach(cleanup);

const NOW = Date.now();
const DAY = 86_400_000;
const LABELS: Record<string, string> = { "acc-a": "Work Max", "acc-b": "Home Pro", "acc-c": "Side" };
const folders = Object.entries(LABELS).map(([id, label]) => ({ id, label, provider: "claude-code", plan: null, login_state: "signed_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false }) as AccountInfo);
const projects = [
  { id: "p1", name: "storefront" },
  { id: "p2", name: "billing" },
] as ProjectInfo[];

type Page = "project" | "app" | "account";

/** Row `i` of a page's list, by rules the expectations below restate. */
function spec(page: Page, i: number) {
  const kind = i % 10 === 0 ? "needs" : i % 10 === 1 ? "working" : i % 10 === 2 ? "open" : "ended";
  const recent = kind === "ended" && i % 3 === 0;
  const last = kind === "ended" ? NOW - (recent ? 2 * DAY : 30 * DAY) - i * 1_000 : NOW - i * 1_000;
  const folder = page === "project" ? (i % 2 === 0 ? "acc-a" : "acc-b") : page === "app" ? "acc-a" : i % 2 === 0 ? "acc-a" : "acc-c";
  const project = page === "project" ? "p1" : i % 3 === 0 ? null : i % 3 === 1 ? "p1" : "p2";
  const title = i % 5 === 0 ? `Alpha task ${i}` : `Task ${i}`;
  return { key: `s/s${i}`, kind, recent, last, folder, project, title };
}

function session(page: Page, i: number): InstanceInfo {
  const s = spec(page, i);
  return {
    id: `s${i}`,
    status: s.kind === "needs" ? "waiting_user" : s.kind === "ended" ? "finished" : "running",
    runtime: s.kind === "open" ? { kind: "terminal", state: "idle", asks: [], can_steer: false } : null,
    provider: "claude-code",
    account_id: s.folder,
    account_label: LABELS[s.folder],
    title: s.title,
    git_branch: null,
    started_at_ms: s.last - 60_000,
    last_activity_ms: s.last,
    project_id: s.project,
    quota: null,
    attention: null,
    needs_attention: null,
    machine_id: "this-mac",
    kind: "terminal",
    cwd: "/w",
  } as unknown as InstanceInfo;
}

const ANCHOR: Record<Page, Anchor> = { project: { project_id: "p1" }, app: { app_id: "cli:acc-a" }, account: { subscription_id: "acct:a" } };

function renderList(page: Page, size: number) {
  const opened: string[] = [];
  const rows = Array.from({ length: size }, (_, i) => sessionRow(session(page, i), NOW, (id) => opened.push(id)));
  render(
    <ActionHostProvider host={{}}>
      <RelatedList
        listKey={`test:${page}:${size}:${Math.random()}`}
        anchor={ANCHOR[page]}
        local={() => rows}
        folders={folders}
        projects={projects}
        showProjects={page !== "project"}
        inApp={page === "app"}
        teach={{ title: "Nothing here yet", body: "Sessions you start here show up here.", action: { label: "New session", onClick: () => opened.push("new") } }}
        onOpenSession={(id) => opened.push(id)}
        searchLabel="Search these sessions"
      />
    </ActionHostProvider>,
  );
  return opened;
}

const RANK: Record<string, number> = { needs: 0, working: 1, open: 2, ended: 3 };

/** What the list should show, written from `spec`. */
function expected(page: Page, size: number, filter: string, search: string): string[] {
  const rows = Array.from({ length: size }, (_, i) => spec(page, i)).filter((s) => {
    if (search && !s.title.toLowerCase().includes(search)) return false;
    switch (filter) {
      case "open":
        return s.kind !== "ended";
      case "recent":
        return s.recent;
      case "ended":
        return s.kind === "ended";
      case "app":
        return s.folder === "acc-a";
      case "account":
        return s.folder === "acc-a" || s.folder === "acc-c";
      case "project":
        return s.project === "p1";
      default:
        return true;
    }
  });
  return rows.sort((a, b) => RANK[a.kind]! - RANK[b.kind]! || b.last - a.last || a.key.localeCompare(b.key)).map((s) => s.key);
}

const shownKeys = () => Array.from(document.querySelectorAll<HTMLElement>("[data-testid='page-session-item']")).map((row) => row.dataset.key);

async function pick(chip: string, value: string) {
  fireEvent.click(screen.getByTestId(`related-filter-${chip}`));
  fireEvent.click(await screen.findByTestId(`related-filter-${chip}-${value}`));
}

const CHIP: Record<string, [string, string]> = {
  open: ["state", "open"],
  recent: ["state", "recent"],
  ended: ["state", "ended"],
  app: ["app", "cli:acc-a"],
  account: ["account", "acct:a"],
  project: ["project", "p1"],
};

const PARAMS = {
  page: ["project", "app", "account"],
  size: ["0", "1", "8", "9", "50", "1000"],
  filter: ["none", "open", "recent", "ended", "app", "account", "project"],
  search: ["", "alpha"],
} as const;

// The page's own kind is no filter of its list; an App's folder is on one
// Account, so its list has no Account to choose.
const CASES = allPairs(PARAMS, (row) => !(row.page === "project" && row.filter === "project") && !(row.page === "app" && (row.filter === "app" || row.filter === "account")) && !(row.page === "account" && row.filter === "account"));

describe("AJ-E3103 K2 by all pairs: page × size × filter × search", () => {
  it("covers every pair in a few dozen cases", () => {
    expect(CASES.length).toBeGreaterThan(30);
    expect(CASES.length).toBeLessThan(80);
  });

  it.each(CASES.map((row) => [`${row.page} · ${row.size} · ${row.filter} · ${row.search || "no search"}`, row] as const))("%s", async (_, row) => {
    const page = row.page as Page;
    const size = Number(row.size);
    renderList(page, size);
    const findable = size > 8;
    // Search and filters appear at 9.
    expect(screen.queryByTestId("related-search") !== null).toBe(findable);
    const filter = findable ? row.filter : "none";
    const search = findable ? row.search : "";
    if (filter !== "none") await pick(...CHIP[filter]!);
    if (search) fireEvent.change(screen.getByTestId("related-search"), { target: { value: search } });
    const want = expected(page, size, filter, search);
    if (want.length === 0) {
      await waitFor(() => expect(screen.getByTestId(filter !== "none" || search ? "related-no-match" : "related-empty")).toBeTruthy());
      expect(shownKeys()).toEqual([]);
      return;
    }
    // The count equals the list (and the reference).
    await waitFor(() => expect(screen.getByTestId("page-sessions-count").textContent).toMatch(new RegExp(`^${want.length} sessions?\\b`)));
    const shown = shownKeys();
    expect(shown.length).toBeGreaterThanOrEqual(Math.min(want.length, 10));
    // The order: needs you, working, open, recent, rest; newest first.
    expect(shown).toEqual(want.slice(0, shown.length));
  });
});

describe("AJ-E3103 the one list", () => {
  it("heads its sections once there's more than one, in order", async () => {
    renderList("app", 12);
    const heads = await screen.findAllByTestId("page-sessions-group");
    expect(heads.map((head) => head.textContent)).toEqual(["Needs you", "Working", "Open", "Last 7 days", "Earlier"]);
  });

  it("an empty list teaches: what would appear and the one step", () => {
    const opened = renderList("app", 0);
    expect(screen.getByTestId("related-empty").textContent).toContain("Sessions you start here show up here.");
    fireEvent.click(screen.getByRole("button", { name: "New session" }));
    expect(opened).toEqual(["new"]);
  });

  it("a filter that empties it says so, and Clear brings everything back", async () => {
    renderList("app", 9);
    await pick("state", "open");
    fireEvent.change(screen.getByTestId("related-search"), { target: { value: "nothing like this" } });
    await waitFor(() => expect(screen.getByTestId("related-no-match").textContent).toContain("“nothing like this” with these filters"));
    fireEvent.click(screen.getByTestId("related-clear"));
    await waitFor(() => expect(shownKeys().length).toBe(9));
  });

  it("the project filter offers “No project”, and it narrows to those", async () => {
    renderList("app", 9);
    await pick("project", "(none)");
    await waitFor(() => expect(shownKeys().sort()).toEqual(["s/s0", "s/s3", "s/s6"]));
    expect(screen.getByTestId("related-filter-project").textContent).toContain("No project");
  });

  it("each filter offers only its values, with how many each shows", async () => {
    renderList("account", 20);
    fireEvent.click(screen.getByTestId("related-filter-app"));
    const work = await screen.findByTestId("related-filter-app-cli:acc-a");
    expect(work.textContent).toContain("Claude Code · Work Max · 10");
    expect(screen.queryByTestId("related-filter-account")).toBeNull();
    expect(screen.queryByTestId("related-filter-machine")).toBeNull();
  });

  it("a row says its App, state, project and when in the same places, and opens its page", async () => {
    const opened = renderList("account", 3);
    const row = document.querySelector<HTMLElement>("[data-key='s/s1']")!;
    expect(row.querySelector("[data-testid='page-session-title']")!.textContent).toBe("Task 1");
    expect(row.querySelector("[data-testid='page-session-project']")!.textContent).toBe("storefront");
    expect(row.querySelector("[data-testid='page-session-app']")!.textContent).toContain("Claude Code · Side");
    expect(row.querySelector("[data-testid='page-session-state']")!.textContent).toBe("Working");
    await act(async () => {
      fireEvent.click(row.querySelector("[data-testid='page-session-row']")!);
    });
    expect(opened).toEqual(["s1"]);
  });
});

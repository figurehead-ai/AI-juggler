/**
 * AJ-E1702 vitest — pools in the rail, the pool sheet and Home's filter.
 *
 * - The rail: Group: Pool and the pool filter appear once the user has
 *   made a pool; pool groups read like project groups (fold, name,
 *   count), empty pools keep a place, the header opens the pool.
 * - The row menu: Move to each other pool, and New pool….
 * - The sheet: make a pool (the session goes in), rename (a taken name
 *   refused in words), roles, add and take out, what roles may do (the
 *   fixed rows locked), delete with its consequence said first. "All
 *   sessions" has no rename or delete.
 * - Home: one pool's sessions only, remembered.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import { DEFAULT_POOL_ID, type GrantInfo, type InstanceInfo, type PoolsInfo, type ProjectInfo } from "../lib/bridge";
import { filterByPool, groupSessions } from "../lib/railview";
import { HomeScreen } from "../screens/Home";
import { PoolSheet } from "./PoolSheet";
import { InstanceBoard } from "./WorkflowRail";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
});

const NOW = 1_728_800_000_000;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: id,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000,
    machine_id: "this-mac",
    cwd: `/w/shop/${id}`,
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-shop",
    conversation_id: null,
    ...overrides,
  } as InstanceInfo;
}

const PROJECTS: ProjectInfo[] = [
  { id: "proj-shop", root: "/w/shop", name: "shop", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false } as ProjectInfo,
];

const GRANTS: GrantInfo[] = [
  { capability: "roster", fixed: true, lead: true, member: true, observer: true },
  { capability: "message", fixed: false, lead: true, member: true, observer: false },
  { capability: "broadcast", fixed: false, lead: true, member: false, observer: false },
];

const SESSIONS = [mk("planner"), mk("builder-1"), mk("builder-2"), mk("other")];

function poolsWith(user: { id: string; name: string; members: [string, string][] }[], scopes = 1): PoolsInfo {
  const members = SESSIONS.map((session) => {
    for (const pool of user) {
      const hit = pool.members.find(([id]) => id === session.id);
      if (hit) return { session_id: session.id, pool_id: pool.id, role: hit[1] };
    }
    return { session_id: session.id, pool_id: DEFAULT_POOL_ID, role: "member" };
  });
  return {
    pools: [
      { id: DEFAULT_POOL_ID, name: "All sessions", kind: "default", project_id: null, grants: GRANTS, scopes: 1 },
      ...user.map((pool) => ({ id: pool.id, name: pool.name, kind: "user", project_id: null, grants: GRANTS, scopes })),
    ],
    members,
  };
}

const DEFAULT_ONLY = poolsWith([]);
const CHECKOUT = poolsWith([
  { id: "pool-checkout", name: "Checkout redesign", members: [["planner", "lead"], ["builder-1", "member"], ["builder-2", "member"]] },
  { id: "pool-empty", name: "Docs", members: [] },
]);

function board(pools: PoolsInfo | null, extra: Partial<Parameters<typeof InstanceBoard>[0]> = {}) {
  return render(
    <InstanceBoard
      sessions={SESSIONS}
      loginTerminals={[]}
      selectedId={null}
      onSelect={() => {}}
      density="compact"
      freshness={{}}
      now={NOW}
      projects={PROJECTS}
      snooze={{}}
      onSnooze={() => {}}
      pools={pools}
      onOpenPool={() => {}}
      onNewPool={() => {}}
      onPoolsChanged={() => {}}
      {...extra}
    />,
  );
}

const openFilters = (container: HTMLElement) => {
  if (container.querySelector('[data-testid="rail-more-filters"]')) return;
  const toggle = [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((button) =>
    (button.getAttribute("aria-label") ?? "").startsWith("Filters"),
  );
  fireEvent.click(toggle!);
};

const ids = (container: HTMLElement) =>
  [...container.querySelectorAll("[data-instance-id]")].filter((el) => el.closest("[hidden]") === null).map((el) => (el as HTMLElement).dataset.instanceId);

describe("AJ-E1702 pools in the rail", () => {
  it("stay out of sight until the user makes one", () => {
    const view = board(DEFAULT_ONLY);
    openFilters(view.container);
    const groupBy = view.container.querySelector<HTMLSelectElement>('select[aria-label="Group by"]')!;
    expect([...groupBy.options].map((option) => option.textContent)).not.toContain("Group: Pool");
    expect(view.container.querySelector('select[aria-label="Filter by pool"]')).toBeNull();
  });

  it("group like projects: fold, name, count; an empty pool keeps its place; the header opens the pool", () => {
    const onOpenPool = vi.fn();
    const view = board(CHECKOUT, { onOpenPool });
    openFilters(view.container);
    fireEvent.change(view.container.querySelector('select[aria-label="Group by"]')!, { target: { value: "pool" } });
    const headers = [...view.container.querySelectorAll<HTMLButtonElement>('[data-testid="rail-pool"]')];
    expect(headers.map((header) => header.textContent)).toEqual(["Checkout redesign3", "All sessions1", "Docs0"]);
    const folds = view.getAllByRole("button", { name: /^Fold / }).map((button) => button.getAttribute("aria-label"));
    expect(folds).toEqual(["Fold Checkout redesign", "Fold All sessions", "Fold Docs"]);
    fireEvent.click(view.getByRole("button", { name: "Fold Checkout redesign" }));
    expect(ids(view.container)).toEqual(["other"]);
    fireEvent.click(headers[2]!);
    expect(onOpenPool).toHaveBeenCalledWith("pool-empty");
    expect(view.getByTestId("rail-new-pool").textContent).toBe("New pool");
  });

  it("filter to one pool behind Filters, and Reset clears it", () => {
    const view = board(CHECKOUT);
    openFilters(view.container);
    fireEvent.change(view.container.querySelector('select[aria-label="Filter by pool"]')!, { target: { value: "pool-checkout" } });
    expect(ids(view.container).sort()).toEqual(["builder-1", "builder-2", "planner"]);
    expect(view.getByRole("button", { name: "Filters (some on)" })).toBeTruthy();
    fireEvent.click(view.getByTestId("rail-filters-reset"));
    expect(ids(view.container)).toHaveLength(4);
  });

  it("the first pool made while the list is open groups the list by pool", () => {
    const view = board(DEFAULT_ONLY);
    expect(view.container.querySelector('[data-testid="rail-pool"]')).toBeNull();
    view.rerender(
      <InstanceBoard sessions={SESSIONS} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="compact" freshness={{}} now={NOW} projects={PROJECTS} snooze={{}} onSnooze={() => {}} pools={CHECKOUT} onOpenPool={() => {}} onNewPool={() => {}} />,
    );
    expect(view.container.querySelectorAll('[data-testid="rail-pool"]').length).toBe(3);
  });

  it("at start-up the list keeps its grouping", () => {
    const view = board(CHECKOUT);
    expect(view.container.querySelector('[data-testid="rail-pool"]')).toBeNull();
  });

  it("a session's menu moves it to another pool, or starts a new one with it", async () => {
    const moved = poolsWith([{ id: "pool-checkout", name: "Checkout redesign", members: [["other", "member"]] }]);
    const move = vi.spyOn(bridge, "poolMove").mockResolvedValue(moved);
    const onPoolsChanged = vi.fn();
    const onNewPool = vi.fn();
    const view = board(CHECKOUT, { onPoolsChanged, onNewPool });
    const row = view.container.querySelector('[data-instance-id="other"]')!;
    fireEvent.click(within(row.parentElement as HTMLElement).getByRole("button", { name: /Row actions/ }));
    const group = view.getByRole("group", { name: "Move to pool" });
    expect([...group.querySelectorAll("[role='menuitem']")].map((item) => item.textContent)).toEqual(["Checkout redesign", "Docs"]);
    const items = [...view.getByRole("menu").querySelectorAll("[role='menuitem']")].map((item) => item.textContent);
    expect(items.at(-1)).toBe("New pool…");
    fireEvent.click(view.getByRole("menuitem", { name: "Checkout redesign" }));
    await waitFor(() => expect(onPoolsChanged).toHaveBeenCalledWith(moved));
    expect(move).toHaveBeenCalledWith("other", "pool-checkout");

    fireEvent.click(within(view.container.querySelector('[data-instance-id="planner"]')!.parentElement as HTMLElement).getByRole("button", { name: /Row actions/ }));
    expect(view.getByRole("menuitem", { name: "All sessions" })).toBeTruthy();
    fireEvent.click(view.getByRole("menuitem", { name: "New pool…" }));
    expect(onNewPool).toHaveBeenCalledWith("planner");
  });

  it("a refused move says why, in the rail", async () => {
    vi.spyOn(bridge, "poolMove").mockRejectedValue({ code: "refused", retryable: false, user_message: "«Shop team» is for shop only — this session works somewhere else" });
    const view = board(CHECKOUT);
    fireEvent.click(within(view.container.querySelector('[data-instance-id="other"]')!.parentElement as HTMLElement).getByRole("button", { name: /Row actions/ }));
    fireEvent.click(view.getByRole("menuitem", { name: "Docs" }));
    await waitFor(() => expect(view.getByTestId("rail-pool-error").textContent).toContain("is for shop only"));
  });

  it("rail helpers: pool groups and the pool filter", () => {
    expect(filterByPool(SESSIONS, null, CHECKOUT)).toHaveLength(4);
    expect(filterByPool(SESSIONS, DEFAULT_POOL_ID, CHECKOUT).map((row) => row.id)).toEqual(["other"]);
    // No list yet: every session is in All sessions.
    expect(groupSessions(SESSIONS, "pool", PROJECTS, null).map((group) => [group.label, group.rows.length])).toEqual([["All sessions", 4]]);
  });
});

describe("AJ-E1702 the pool sheet", () => {
  const sheet = (target: Parameters<typeof PoolSheet>[0]["target"], pools = CHECKOUT, onChanged = vi.fn()) => {
    const view = render(<PoolSheet target={target} pools={pools} sessions={SESSIONS} projects={PROJECTS} onClose={() => {}} onChanged={onChanged} />);
    return { ...view, onChanged };
  };

  it("makes a pool and puts the session in it", async () => {
    const made = poolsWith([{ id: "pool-new", name: "Checkout redesign", members: [] }]);
    const placed = poolsWith([{ id: "pool-new", name: "Checkout redesign", members: [["planner", "member"]] }]);
    const create = vi.spyOn(bridge, "poolCreate").mockResolvedValue(made);
    const move = vi.spyOn(bridge, "poolMove").mockResolvedValue(placed);
    const view = sheet({ mode: "create", sessionId: "planner" }, DEFAULT_ONLY);
    expect(view.getByTestId("pool-create-session").textContent).toBe("shop · planner · Work Max goes in it.");
    expect((view.getByTestId("pool-create-save") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(view.getByTestId("pool-create-name"), { target: { value: "Checkout redesign" } });
    fireEvent.change(view.getByTestId("pool-create-project"), { target: { value: "proj-shop" } });
    fireEvent.click(view.getByTestId("pool-create-save"));
    await waitFor(() => expect(view.onChanged).toHaveBeenCalledWith(placed, "pool-new"));
    expect(create).toHaveBeenCalledWith("Checkout redesign", "proj-shop");
    expect(move).toHaveBeenCalledWith("planner", "pool-new");
  });

  it("a taken name is refused in words, and nothing else happens", async () => {
    vi.spyOn(bridge, "poolCreate").mockRejectedValue({ code: "refused", retryable: false, user_message: "There's already a pool called «Docs» — pick another name" });
    const move = vi.spyOn(bridge, "poolMove");
    const view = sheet({ mode: "create", sessionId: "planner" });
    fireEvent.change(view.getByTestId("pool-create-name"), { target: { value: "docs" } });
    fireEvent.click(view.getByTestId("pool-create-save"));
    await waitFor(() => expect(view.getByRole("alert").textContent).toBe("There's already a pool called «Docs» — pick another name"));
    expect(move).not.toHaveBeenCalled();
    expect(view.onChanged).not.toHaveBeenCalled();
  });

  it("opens a pool: its sessions with their roles, rename, add, take out", async () => {
    const after = poolsWith([{ id: "pool-checkout", name: "Checkout v2", members: [["planner", "lead"]] }]);
    const rename = vi.spyOn(bridge, "poolRename").mockResolvedValue(after);
    const role = vi.spyOn(bridge, "poolSetRole").mockResolvedValue(after);
    const move = vi.spyOn(bridge, "poolMove").mockResolvedValue(after);
    const view = sheet({ mode: "pool", poolId: "pool-checkout" });
    expect(view.getByRole("heading", { name: "Checkout redesign" })).toBeTruthy();
    const members = view.getAllByTestId("pool-member");
    expect(members).toHaveLength(3);
    expect(view.getByRole("combobox", { name: "Role for shop · planner · Work Max" })).toBeTruthy();
    const roles = view.getAllByTestId("pool-member-role").map((select) => (select as HTMLSelectElement).value);
    expect(roles).toEqual(["lead", "member", "member"]);

    // Rename shows only when the name changed.
    expect(view.queryByTestId("pool-rename")).toBeNull();
    fireEvent.change(view.getByTestId("pool-name"), { target: { value: "Checkout v2" } });
    fireEvent.click(view.getByTestId("pool-rename"));
    await waitFor(() => expect(rename).toHaveBeenCalledWith("pool-checkout", "Checkout v2"));

    fireEvent.change(view.getAllByTestId("pool-member-role")[1]!, { target: { value: "observer" } });
    await waitFor(() => expect(role).toHaveBeenCalledWith("builder-1", "observer"));

    // Add: only sessions not in it.
    const add = view.getByTestId("pool-add") as HTMLSelectElement;
    expect([...add.options].map((option) => option.value)).toEqual(["", "other"]);
    fireEvent.change(add, { target: { value: "other" } });
    await waitFor(() => expect(move).toHaveBeenCalledWith("other", "pool-checkout"));
    fireEvent.click(view.getAllByTestId("pool-member-out")[0]!);
    await waitFor(() => expect(move).toHaveBeenCalledWith("planner", DEFAULT_POOL_ID));
  });

  it("what each role may do: behind a disclosure, the fixed rows locked", async () => {
    const grant = vi.spyOn(bridge, "poolSetGrant").mockResolvedValue(CHECKOUT);
    const view = sheet({ mode: "pool", poolId: "pool-checkout" });
    const details = view.getByTestId("pool-grants") as HTMLDetailsElement;
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("Agents can't reach pools yet");
    const fixed = view.getByRole("checkbox", { name: "Observer: See who's here and what they're doing" }) as HTMLInputElement;
    expect(fixed.checked).toBe(true);
    expect(fixed.disabled).toBe(true);
    const broadcast = view.getByRole("checkbox", { name: "Member: Message the whole pool" }) as HTMLInputElement;
    expect(broadcast.checked).toBe(false);
    fireEvent.click(broadcast);
    await waitFor(() => expect(grant).toHaveBeenCalledWith("pool-checkout", "broadcast", "member", true));
  });

  it("delete says what happens first, then goes back to the list", async () => {
    const del = vi.spyOn(bridge, "poolDelete").mockResolvedValue(DEFAULT_ONLY);
    const view = sheet({ mode: "pool", poolId: "pool-checkout" });
    fireEvent.click(view.getByTestId("pool-delete"));
    expect(view.getByTestId("pool-delete-confirm").textContent).toContain("Its 3 sessions go back to All sessions; nothing else changes.");
    expect(del).not.toHaveBeenCalled();
    fireEvent.click(view.getByTestId("pool-delete-yes"));
    await waitFor(() => expect(view.onChanged).toHaveBeenCalledWith(DEFAULT_ONLY, null));
  });

  it("All sessions: no rename, no delete, no take out; says what agents will see (D-33)", () => {
    const view = sheet({ mode: "pool", poolId: DEFAULT_POOL_ID });
    expect(view.queryByTestId("pool-name")).toBeNull();
    expect(view.queryByTestId("pool-delete")).toBeNull();
    expect(view.queryByTestId("pool-member-out")).toBeNull();
    expect(view.queryByTestId("pool-add")).toBeNull();
    expect(view.getByTestId("pool-scope").textContent).toBe("Agents here will see only sessions on their own project and account.");
    expect(view.getAllByTestId("pool-member")).toHaveLength(1);
  });

  it("a pool mixing projects or accounts says so", () => {
    const mixed = poolsWith([{ id: "pool-mixed", name: "Mixed", members: [["planner", "member"]] }], 2);
    const view = sheet({ mode: "pool", poolId: "pool-mixed" }, mixed);
    expect(view.getByTestId("pool-scope").textContent).toBe("This pool mixes projects or accounts: agents in it would see each other's work.");
  });
});

describe("AJ-E1702 Home's pool filter", () => {
  beforeEach(() => {
    vi.spyOn(bridge, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(bridge, "dashboardStats").mockResolvedValue({ ledger_degraded: false } as never);
  });

  const home = (pools: PoolsInfo | null) =>
    render(
      <HomeScreen
        sessions={SESSIONS}
        pools={pools}
        accounts={[{ id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false } as bridge.AccountInfo]}
        projects={PROJECTS}
        now={NOW}
        onOpenSession={() => {}}
        onNewSession={() => {}}
        onAddAccounts={() => {}}
      />,
    );

  it("isn't there without a pool", () => {
    const view = home(DEFAULT_ONLY);
    expect(view.queryByTestId("home-pool-filter")).toBeNull();
  });

  it("shows one pool's sessions only, and remembers it", () => {
    const view = home(CHECKOUT);
    const running = () => [...view.getByTestId("home-running").querySelectorAll("li")].length;
    expect(running()).toBe(4);
    const filter = view.getByTestId("home-pool-filter") as HTMLSelectElement;
    expect([...filter.options].map((option) => option.textContent)).toEqual(["Pool: all", "All sessions", "Checkout redesign", "Docs"]);
    fireEvent.change(filter, { target: { value: "pool-checkout" } });
    expect(running()).toBe(3);
    expect(localStorage.getItem("aij.home.pool")).toBe("pool-checkout");
    cleanup();
    const again = home(CHECKOUT);
    expect((again.getByTestId("home-pool-filter") as HTMLSelectElement).value).toBe("pool-checkout");
  });
});

/**
 * AJ-E2807: the Pools screen's order, a card's menu (Open its canvas,
 * Rename…, Delete… saying what happens to its members) and its empty text.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { PoolInfo } from "../../types/generated/PoolInfo";
import { resetPreviewPools, scopeOf } from "../../lib/ipc/pools";
import { forgetPaged } from "../../lib/paged";
import { refreshPools, resetPools, setPools } from "../../lib/pools";
import { cardOrder, PoolsScreen } from "./PoolsScreen";

const first: InstanceInfo = MOCK_SESSIONS[0]!;
const second: InstanceInfo = MOCK_SESSIONS[1]!;
const seed = (pools: unknown) => Object.assign(window, { __AIJ_POOLS__: pools });

const pool = (id: string, kind: string, members: number): PoolInfo => ({
  id,
  name: id,
  kind,
  project_id: kind === "default" ? id : null,
  access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false },
  rules: [],
  spans_macs: false,
  scopes: 1,
  members,
  ...scopeOf(kind, kind === "default" ? id : null),
});

beforeEach(() => {
  resetPreviewPools();
  resetPools();
  forgetPaged();
  Reflect.deleteProperty(window, "__AIJ_POOLS__");
});
afterEach(cleanup);

describe("the Pools screen (AJ-E2807)", () => {
  it("puts the pools that ask you first, then the latest active (R2)", () => {
    const pools = [pool("mine", "user", 2), pool("quiet", "default", 3), pool("asking", "default", 1), pool("busy", "user", 1)];
    const order = cardOrder(pools, new Map(), new Map([["asking", 1]]), new Map([["busy", 900], ["mine", 100], ["quiet", 500]]));
    expect(order.map((row) => row.id)).toEqual(["asking", "busy", "quiet", "mine"]);
  });

  it("gives a card its menu: open, rename, and delete saying what happens to its members (R4)", async () => {
    seed({
      pools: [{ id: "pool-team", name: "Team", rules: [] }],
      members: [
        { session_id: first.id, pool_id: "pool-team", role: "lead" },
        { session_id: second.id, pool_id: "pool-team", role: "member" },
      ],
    });
    await act(async () => {
      await refreshPools();
    });
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => void opened.push(id) }}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const names = () => view.queryAllByTestId("pool-card-tile").map((tile) => tile.textContent ?? "");
    const card = (await view.findAllByTestId("pool-card-tile")).find((tile) => tile.dataset.pool === "pool-team")!;
    const box = card.parentElement!;
    // A few cards keep a card's width (the E28 walk: one pool was a
    // full-width banner).
    expect(view.getByTestId("pools-cards").style.gridTemplateColumns).toContain("auto-fill");
    fireEvent.contextMenu(box, { clientX: 30, clientY: 30, button: 2 });
    const menu = view.getByRole("menu");
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Open its canvas", "Rename…", "Delete…"]);
    for (const item of within(menu).getAllByRole("menuitem")) expect(item.querySelector("svg"), item.textContent ?? "").toBeTruthy();
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Open its canvas" }));
    expect(opened).toEqual(["pool-team"]);
    // Rename… from its ⋯ button
    fireEvent.click(within(box).getByTestId("pool-card-menu"));
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: "Rename…" }));
    fireEvent.change(view.getByTestId("pool-rename-input"), { target: { value: "Checkout team" } });
    fireEvent.click(view.getByTestId("pool-rename-save"));
    await waitFor(() => expect(names().some((words) => words.includes("Checkout team"))).toBe(true));
    // Delete… asks first, saying where its sessions go
    fireEvent.keyDown(box, { key: "F10", shiftKey: true });
    fireEvent.click(within(view.getByRole("menu")).getByRole("menuitem", { name: "Delete…" }));
    expect(view.getByTestId("pool-delete-words").textContent).toContain("Its 2 sessions go back to their projects' own pools");
    fireEvent.click(view.getByTestId("pool-delete-confirm"));
    await waitFor(() => expect(names().some((words) => words.includes("Checkout team"))).toBe(false));
  });

  it("a project's own pool can't be renamed or deleted, and its menu says why", async () => {
    await act(async () => {
      await refreshPools();
    });
    const view = render(
      <ActionHostProvider host={{ openPool: () => undefined }}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const tiles = await view.findAllByTestId("pool-card-tile");
    const project = tiles.find((tile) => within(tile).getByTestId("pool-card-scope").textContent === "(project pool)")!;
    fireEvent.contextMenu(project.parentElement!, { clientX: 30, clientY: 30, button: 2 });
    const rename = within(view.getByRole("menu")).getByRole("menuitem", { name: /Rename…/ });
    expect(rename.hasAttribute("disabled") || rename.getAttribute("aria-disabled") === "true").toBe(true);
    expect(view.getByRole("menu").textContent).toContain("A project's own pool goes with its project");
  });

  it("with no project yet, says pools come with it (R5)", async () => {
    act(() => setPools({ pools: [], placements: [] }));
    const view = render(<PoolsScreen sessions={[]} projects={[]} accounts={[]} />);
    expect((await view.findByTestId("pools-empty")).textContent).toBe("Pools appear with your first project.");
  });
});

/**
 * R4 (main's ruling, from the persona walks: three testers expected one
 * confirmation and got New pool's nine steps): "Keep it apart…" on a
 * project opens one sheet that says what it does in plain words. Keep
 * apart makes the pool the wizard's sealed start would, by the same
 * calls; Customise… opens the wizard on that start; a project already
 * kept apart opens its pool.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState, type JSX } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AppActions } from "../../components/AppActions";
import { ActionHostProvider, useActionHost } from "../../components/ItemMenu";
import { ToastHost, resetToasts } from "../../components/Toast";
import { poolsList, resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools, takeNewPoolRequest, doneNewPoolRequest } from "../../lib/pools";
import { sealLine } from "../../lib/pool-sentences";
import { startName } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { KEEP_APART_START } from "../../lib/keep-apart";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { clearSeams, seamCalls, setSeam } from "../../test/fixtures";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { NewPool } from "./NewPool";
import { PoolsScreen } from "./PoolsScreen";

const calls = () => seamCalls("__AIJ_POOL_CALLS__");

const project: ProjectInfo = MOCK_PROJECTS[0]!;
/** Main's ruling on R4: a pool kept apart is named after its project. */
const KEPT = `${project.name} · kept apart`;
/** Long enough for the sheet's code to load on its first open. */
const LOADS = { timeout: 5_000 };

beforeEach(async () => {
  clearSeams("__AIJ_POOL_CALLS__");
  localStorage.clear();
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await act(async () => {
    await refreshPools();
  });
});
afterEach(() => {
  cleanup();
  resetToasts();
  doneNewPoolRequest();
  clearSeams("__AIJ_POOLS__");
  resetPreviewPools();
  vi.restoreAllMocks();
});

/** What the project's menu and its Agents card do: the shared action. */
function KeepItApart(): JSX.Element {
  const host = useActionHost();
  return (
    <button type="button" onClick={() => host.keepProjectApart?.(project)}>
      Keep it apart…
    </button>
  );
}

function renderIt(given: { selectPool?: (id: string) => void; openPools?: () => void } = {}) {
  const none = () => {};
  const selectPool = vi.fn(given.selectPool ?? none);
  const openPools = vi.fn(given.openPools ?? none);
  render(
    <AppActions selectConversation={none} selectSession={none} selectProject={none} selectAccount={none} selectTool={none} openToolSettings={none} selectPool={selectPool} openAccounts={none} openPools={openPools} openHome={none} stopSession={none} newSessionIn={none} refresh={none}>
      <KeepItApart />
      <ToastHost />
    </AppActions>,
  );
  return { selectPool, openPools };
}

const open = async () => {
  fireEvent.click(screen.getByRole("button", { name: "Keep it apart…" }));
  return screen.findByRole("dialog", { name: `Keep ${project.name} apart` }, LOADS);
};

/** Pool ids read alike, so two runs' calls compare. */
const sameIds = (list: string[]) => list.map((call) => call.replace(/pool-preview-\d+/g, "<pool>"));

describe("Keep it apart… (R4)", () => {
  it("opens one sheet that says what it does and doesn't, in plain words, with the guide", async () => {
    const { openPools } = renderIt();
    const sheet = await open();
    const text = sheet.textContent ?? "";
    expect(screen.getByTestId("keep-apart-lead").textContent).toBe(`${project.name}'s sessions get a team of their own. Nothing from their work reaches your other pools through AI Juggler.`);
    expect(screen.getAllByTestId("keep-apart-fact").map((fact) => fact.textContent)).toEqual([
      `In this pool onlyEvery session in ${project.name}, now and later, is in this pool and in no other.`,
      "Nothing in or outNo messages, notes or tasks pass between it and your other pools through AI Juggler.",
      "Still a team insideIts agents can still message each other and share notes and tasks.",
    ]);
    expect(screen.getByTestId("keep-apart-example").textContent).toMatch(/^Example: /);
    // What it doesn't do: no account tied to it (D-134 is later).
    expect(screen.getByTestId("keep-apart-not").textContent).toContain(`It doesn't tie ${project.name} to one account`);
    // What it makes, before it does: the wizard's name for it, and how its sessions join.
    expect(screen.getByTestId("keep-apart-makes").textContent).toBe(
      `It makes the pool «${KEPT}». New sessions in ${project.name} join as Member and may Work together. Change any of it first with Customise…, or later on its diagram.`,
    );
    // "What does this mean?": the word its header will show first, then the guide's section.
    fireEvent.click(within(sheet).getByRole("button", { name: "What does this mean?" }));
    expect(screen.getByTestId("keep-apart-explain").textContent).toContain(sealLine({ sealed: true }));
    expect(within(sheet).getByRole("button", { name: /Learn more in the guide/ })).toBeTruthy();
    // No internal words.
    expect(text).not.toMatch(/template|scope|sealed-client|\bpack\b/i);
    // Its buttons; nothing made, and the wizard not opened, by opening it.
    expect(within(sheet).getByRole("button", { name: "Keep apart" }).hasAttribute("data-primary")).toBe(true);
    expect(within(sheet).getByRole("button", { name: "Customise…" })).toBeTruthy();
    expect(openPools).not.toHaveBeenCalled();
    expect(takeNewPoolRequest()).toBeNull();
    expect(calls()).toEqual([]);
    // Cancel does nothing.
    fireEvent.click(within(sheet).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls()).toEqual([]);
  });

  it("Keep apart makes one sealed pool on the project, as the wizard's sealed start would, and a toast names it with Open it", async () => {
    // The wizard's own: its sealed start on the project, made as it comes.
    let wizardMade: string | null = null;
    const wizard = render(
      <ActionHostProvider host={{ openPool: (id: string) => (wizardMade = id) }}>
        <NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={project.id} startWith="sealed-client" onClose={() => undefined} />
      </ActionHostProvider>,
    );
    for (let n = 0; n < 4 && !screen.queryByTestId("new-pool-make"); n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
    fireEvent.click(screen.getByTestId("new-pool-make"));
    await waitFor(() => expect(wizardMade).not.toBeNull());
    const byWizard = sameIds(calls());
    wizard.unmount();
    clearSeams("__AIJ_POOL_CALLS__");
    resetPreviewPools();
    resetPreviewPoolModel();
    resetPools();
    await act(async () => {
      await refreshPools();
    });

    const { selectPool } = renderIt();
    const sheet = await open();
    fireEvent.click(within(sheet).getByRole("button", { name: "Keep apart" }));
    const toast = await screen.findByTestId("keep-apart-toast");
    expect(screen.queryByRole("dialog")).toBeNull();
    const made = (await poolsList()).pools.filter((pool) => pool.kind === "user");
    expect(made).toHaveLength(1);
    const pool = made[0]!;
    expect(pool).toMatchObject({ name: KEPT, project_id: project.id, sealed: true });
    expect(calls()).toEqual([
      `create:${KEPT}`,
      `scope:${pool.id}:${project.id}`,
      `join:${pool.id}:all:member:`,
      `talk:${pool.id}:role:observer:role:lead:blocked`,
      `talk:${pool.id}:role:observer:role:member:blocked`,
      `talk:${pool.id}:role:observer:role:reviewer:blocked`,
      `talk:${pool.id}:role:observer:role:observer:blocked`,
      `sealed:${pool.id}:true`,
    ]);
    // The same calls as the wizard's Make pool, one for one.
    expect(sameIds(calls())).toEqual(byWizard);
    expect(toast.textContent).toContain(`${project.name} is kept apart`);
    expect(toast.textContent).toContain(`«${KEPT}»`);
    fireEvent.click(within(toast).getByRole("button", { name: "Open it" }));
    expect(selectPool).toHaveBeenCalledWith(pool.id);
  });

  it("Customise… opens New pool on the sealed start, as the action did before the sheet; nothing is made", async () => {
    function App(): JSX.Element {
      const [pools, setPools] = useState(false);
      const none = () => {};
      return (
        <AppActions selectConversation={none} selectSession={none} selectProject={none} selectAccount={none} selectTool={none} openToolSettings={none} selectPool={none} openAccounts={none} openPools={() => setPools(true)} openHome={none} stopSession={none} newSessionIn={none} refresh={none}>
          <KeepItApart />
          {pools && <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />}
        </AppActions>
      );
    }
    render(<App />);
    const sheet = await open();
    fireEvent.click(within(sheet).getByRole("button", { name: "Customise…" }));
    expect(screen.queryByRole("dialog", { name: `Keep ${project.name} apart` })).toBeNull();
    const wizard = await screen.findByTestId("new-pool");
    const sealed = within(wizard)
      .getAllByTestId("new-pool-template")
      .find((card) => card.dataset.template === "sealed-client")!;
    expect(within(sealed).getByRole("radio")).toHaveProperty("checked", true);
    fireEvent.click(within(wizard).getByTestId("new-pool-next"));
    expect(within(wizard).getByTestId("new-pool-name")).toHaveProperty("value", KEPT);
    expect(calls()).toEqual([]);
  });

  it("is named after its project, numbered when taken and clipped to fit; Pools › New pool's sealed start with no project keeps its own name", () => {
    const sealed = templateById(KEEP_APART_START);
    expect(startName(sealed, "web", [])).toBe("web · kept apart");
    expect(startName(sealed, "web", ["Web · kept apart"])).toBe("web · kept apart 2");
    expect(startName(sealed, null, [])).toBe("Sealed client project");
    const long = startName(sealed, "a-very-long-client-project-name-for-acme-and-co", []);
    expect(Array.from(long).length).toBeLessThanOrEqual(45);
    expect(long).toMatch(/^a-very-long-client-project-name…? · kept apart$/);
    // A template not named after a project keeps its own name.
    expect(startName(templateById("feature-team"), "web", [])).toBe("Feature team");
    expect(startName(templateById("blank"), "web", [])).toBe("");
  });

  it("New pool from the Pools screen: choosing its sealed start, with no project picked, keeps «Sealed client project»", () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} />);
    const sealed = screen.getAllByTestId("new-pool-template").find((card) => card.dataset.template === "sealed-client")!;
    fireEvent.click(within(sealed).getByRole("radio"));
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(screen.getByTestId("new-pool-name")).toHaveProperty("value", "Sealed client project");
  });

  it("a project already kept apart opens its pool instead", async () => {
    setSeam("__AIJ_POOLS__", { pools: [{ id: "pool-client-a", name: "Client A only", project_id: project.id, sealed: true }] });
    resetPreviewPools();
    await act(async () => {
      await refreshPools();
    });
    const { selectPool, openPools } = renderIt();
    fireEvent.click(screen.getByRole("button", { name: "Keep it apart…" }));
    expect(selectPool).toHaveBeenCalledWith("pool-client-a");
    expect(openPools).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls()).toEqual([]);
  });

  it("with the pools not read yet, it reads them first and still opens the pool keeping it apart", async () => {
    setSeam("__AIJ_POOLS__", { pools: [{ id: "pool-client-a", name: "Client A only", project_id: project.id, sealed: true }] });
    resetPreviewPools();
    resetPools();
    const { selectPool } = renderIt();
    fireEvent.click(screen.getByRole("button", { name: "Keep it apart…" }));
    await waitFor(() => expect(selectPool).toHaveBeenCalledWith("pool-client-a"));
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

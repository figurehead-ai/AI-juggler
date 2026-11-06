/**
 * AJ-E3007: the New pool wizard. A template sets roles and talk; step 2
 * picks from what exists, greyed rows saying why; step 3's grid cycles a
 * pair's rule; the review says it in sentences; Make sends the user's own
 * calls in one go (create, scope, members, talk) and opens the canvas;
 * Escape leaves nothing made.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { draftOf, sessionCandidates, talkChanges } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { NewPool, freeName } from "./NewPool";
import { talkSentence } from "./NewPoolReview";

type Seams = { __AIJ_POOL_CALLS__?: string[] };
const seams = () => globalThis as unknown as Seams;

beforeEach(async () => {
  delete seams().__AIJ_POOL_CALLS__;
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await refreshPools();
});
afterEach(() => cleanup());

describe("AJ-E3007 the templates and the draft", () => {
  it("a template's talk and roles become the draft's", () => {
    const team = draftOf(templateById("feature-team"), "p1");
    expect(team.projects).toEqual(["p1"]);
    expect(talkChanges(team.talk)).toContainEqual({ from: "role:member", to: "role:member", rule: "ask" });
    const everywhere = draftOf(templateById("reviewer-for-everything"), "p1");
    expect(everywhere.every).toBe(true);
    expect(talkSentence(everywhere)).toContain("Member to Member: blocked");
    expect(draftOf(templateById("sealed-client"), "p1").sealed).toBe(true);
  });

  it("a session out of scope says why and offers the scope", () => {
    const draft = { ...draftOf(templateById("writer-reviewer"), "nowhere"), projects: ["nowhere"] };
    const rows = sessionCandidates({ draft, sessions: MOCK_SESSIONS, conversations: [], pools: null, tools: [], projectName: (id) => id ?? "No project", nameOf: (s) => s.id, now: Date.now() });
    const out = rows.find((row) => row.blocked?.includes("not in this pool's projects"));
    expect(out?.fix?.action).toBe("scope");
  });

  it("a name already taken gets the next number", () => {
    expect(freeName("Scouts", ["scouts", "Scouts 2"])).toBe("Scouts 3");
  });
});

describe("AJ-E3007 the wizard, start to end", () => {
  it("picks, sets talk, reviews and makes the pool with the user's own calls", async () => {
    let opened: string | null = null;
    render(
      <ActionHostProvider host={{ openPool: (id: string) => (opened = id) }}>
        <NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => undefined} onByHand={() => undefined} />
      </ActionHostProvider>,
    );
    const wizard = screen.getByTestId("new-pool");
    fireEvent.click(within(wizard).getAllByTestId("new-pool-template").find((card) => card.dataset.template === "feature-team")!);
    fireEvent.click(screen.getByTestId("new-pool-next"));
    // Every project, so every open session can join.
    fireEvent.click(within(screen.getByTestId("new-pool-where")).getByText("Every project"));
    const first = screen.getAllByTestId("new-pool-candidate").find((row) => !row.querySelector("[role=checkbox]")!.hasAttribute("disabled"))!;
    fireEvent.click(within(first).getByRole("checkbox"));
    expect(within(screen.getByTestId("new-pool-side")).getByText(/1 full|0 full/)).toBeTruthy();
    fireEvent.click(screen.getByTestId("new-pool-next"));
    // The template's member↔member ask first, then blocked on a click.
    const cell = screen.getAllByTestId("new-pool-talk-cell").find((one) => one.dataset.pair === "member>member")!;
    expect(cell.textContent).toBe("Ask first");
    fireEvent.click(cell);
    expect(cell.textContent).toBe("Blocked");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(screen.getByTestId("new-pool-review").textContent).toContain("Member to Member: blocked");
    fireEvent.change(screen.getByTestId("new-pool-name"), { target: { value: "Checkout team" } });
    fireEvent.click(screen.getByTestId("new-pool-next"));
    await waitFor(() => expect(opened).not.toBeNull());
    const calls = seams().__AIJ_POOL_CALLS__ ?? [];
    expect(calls).toContain("create:Checkout team");
    expect(calls.some((call) => call.startsWith(`scope:${opened}:every`))).toBe(true);
    expect(calls.some((call) => call.startsWith(`join:${opened}:session:`) && call.endsWith(":lead:"))).toBe(true);
    expect(calls).toContain(`talk:${opened}:role:member:role:member:blocked`);
  });

  it("Escape leaves nothing made", () => {
    let closed = false;
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project={null} onClose={() => (closed = true)} onByHand={() => undefined} />);
    fireEvent.keyDown(screen.getByTestId("new-pool"), { key: "Escape" });
    expect(closed).toBe(true);
    expect(seams().__AIJ_POOL_CALLS__ ?? []).toEqual([]);
  });
});

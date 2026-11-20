/**
 * SK-13 (the soak's LA and LA2): a named session's role was set only on
 * its canvas card, and who may message whom between two members only by
 * pressing their line on the canvas. The List view, the canvas's twin for
 * the keyboard and screen readers, sets both now; the member's inspector
 * sets its role too; and New pool names the Reviewer a template promises.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { pick } from "../../test/pick";
import { draftOf, type Candidate, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { NewPoolEveryone } from "../../screens/pools/NewPoolEveryone";
import { first, openPage, resetCanvasTests, second, seams, seedTeam } from "./canvas.test-kit";

beforeEach(resetCanvasTests);
afterEach(() => {
  cleanup();
  localStorage.clear();
});

const toList = async (view: Awaited<ReturnType<typeof openPage>>) => {
  fireEvent.click(within(view.getByTestId("pool-mode")).getByRole("radio", { name: "List" }));
  return view.findByTestId("pool-list");
};

describe("the List view sets what the canvas sets", () => {
  it("each member's row has its role, set there as on its card", async () => {
    seedTeam();
    const view = await openPage();
    const list = await toList(view);
    await waitFor(() => expect(within(list).getAllByTestId("pool-list-role")).toHaveLength(2));
    const rows = within(list).getAllByTestId("pool-card");
    const row = rows.find((one) => one.querySelector("[data-testid='pool-card-role']")?.textContent?.includes("Member"))!;
    pick(within(row).getByTestId("pool-list-role"), "Reviewer");
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`role:${second.id}:reviewer`));
  });

  it("any two members' line opens in the inspector, its rule set there; the role pairs' one click away", async () => {
    seedTeam();
    const view = await openPage();
    const list = await toList(view);
    const pair = within(list).getByTestId("pool-list-pair");
    expect(within(pair).getByTestId("pool-list-pair-open").hasAttribute("disabled")).toBe(true);
    pick(within(pair).getByTestId("pool-list-pair-from"), view.getAllByTestId("pool-card-name")[1]?.textContent ?? "");
    pick(within(pair).getByTestId("pool-list-pair-to"), view.getAllByTestId("pool-card-name")[0]?.textContent ?? "");
    fireEvent.click(within(pair).getByTestId("pool-list-pair-open"));
    expect(await view.findByTestId("inspect-edge")).toBeTruthy();
    expect(await view.findByTestId("edge-talk-rule")).toBeTruthy();
    fireEvent.click(within(pair).getByTestId("pool-list-pair-roles"));
    expect(await view.findByTestId("pool-talk")).toBeTruthy();
  });

  it("a member's inspector sets its role too, Lead included", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.container.querySelector<HTMLButtonElement>(`[data-node-name="${second.id}"]`)!, { detail: 0 });
    pick(await view.findByTestId("inspect-node-role"), "Lead");
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`role:${second.id}:lead`));
    expect(seams().__AIJ_POOL_CALLS__).toContain(`role:${first.id}:member`);
  });
});

describe("New pool names the Reviewer its template promises", () => {
  it("Writer and reviewer: a session open now is picked as the Reviewer", () => {
    const writer: Draft["members"][number] = { ref: "session:w", kind: "session", label: "p1-writer", role: "lead", level: null, reach: "full", shell: null };
    let draft: Draft = { ...draftOf(templateById("writer-reviewer"), "p-web"), members: [writer] };
    const open: Candidate = { ref: "session:r", kind: "session", label: "p1-reviewer", detail: "web · open", group: "open", reach: "full", blocked: null, lastMs: 2 };
    const view = render(<NewPoolEveryone draft={draft} onDraft={(next) => (draft = next)} where="web" nameOf={(_, label) => label} candidates={[open]} apps={[]} />);
    pick(screen.getByTestId("new-pool-reviewer-pick"), "p1-reviewer");
    expect(draft.members.map((member) => [member.ref, member.role])).toEqual([
      ["session:w", "lead"],
      ["session:r", "reviewer"],
    ]);
    view.rerender(<NewPoolEveryone draft={draft} onDraft={(next) => (draft = next)} where="web" nameOf={(_, label) => label} candidates={[open]} apps={[]} />);
    // Named: the row goes; its member row says Reviewer.
    expect(screen.queryByTestId("new-pool-name-reviewer")).toBeNull();
  });
});

/**
 * The rc2 walk P5 (AJ-E3304 L4): a member's own choice in its pool —
 * what it may do on its own there — is reachable from its node's
 * inspector and from its row in the List view. The walker found it on
 * neither: the node took no accessibility press, the row had none.
 */
import { cleanup, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { forgetSharedReads } from "../../lib/shared-read";
import { first, openPage, resetCanvasTests, seedTeam } from "./canvas.test-kit";

beforeEach(() => {
  resetCanvasTests();
  forgetSharedReads();
});
afterEach(cleanup);

describe("a member's own choice here, from both places (the rc2 walk P5)", () => {
  it("from its node: press the node (a button), and its inspector offers it near the top", async () => {
    seedTeam();
    const view = await openPage();
    const node = view.container.querySelector<HTMLButtonElement>(`[data-node-name="${first.id}"]`)!;
    // The node itself, found by its role: the press an accessibility client makes.
    expect(view.getAllByRole("button", { name: node.getAttribute("aria-label")! })).toContain(node);
    fireEvent.click(node, { detail: 0 });
    const own = await view.findByTestId("inspect-node-own");
    expect(within(own).getByRole("heading", { name: "What it may do on its own" })).toBeTruthy();
    expect(within(own).getByRole("button", { name: /^What it may do on its own: / })).toBeTruthy();
    // Above what it's doing, its tasks and its usage, not at the foot.
    const body = view.getByTestId("inspect-node");
    const sections = [...body.querySelectorAll("section h4")].map((heading) => heading.textContent);
    expect(sections.indexOf("What it may do on its own")).toBeLessThan(sections.indexOf("Usage"));
  });

  it("from its row in the List view, behind the row's disclosure", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.getByTestId("pool-mode-list"));
    const row = (await view.findAllByTestId("pool-card"))[0]!;
    fireEvent.click(within(row).getByRole("button", { name: /^Show what .+ is doing$/ }));
    const own = await within(row).findByTestId("pool-card-own");
    await waitFor(() => expect(within(own).getByRole("button", { name: /may do on its own: / })).toBeTruthy());
    expect(row.textContent).toContain("On its own");
  });
});

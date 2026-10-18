/**
 * AJ-E1002 vitest — the worktree offer and the removal guard: a second
 * launch into a busy project defaults to its own worktree at a shown,
 * editable path; sharing is the explicit opt-out with its risk named; a
 * non-git project says isolation is unavailable; removing a worktree
 * with uncommitted work needs its folder name typed back.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { IsolationOfferDialog } from "./IsolationOfferDialog";
import { WorktreeRemoveDialog } from "./WorktreeGit";
import { launchSession, type IsolationOfferInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_LAUNCH_CALLS__ as string[] | undefined) ?? [];

const offer = (over: Partial<IsolationOfferInfo> = {}): IsolationOfferInfo => ({
  project_name: "storefront",
  in_use_by: ["Work Max"],
  available: true,
  unavailable_reason: null,
  proposed_path: "/Users/dev/storefront.juggler/a1b2c3",
  proposed_branch: "juggler/a1b2c3",
  base_branch: "main",
  ...over,
});

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_ISOLATION_OFFER__;
  delete w().__AIJ_LAUNCH_CALLS__;
  delete w().__AIJ_WORKTREE_REMOVAL__;
});

describe("AJ-E1002 worktree offer", () => {
  it("defaults to its own worktree at the shown path and creates it only on the click", async () => {
    w().__AIJ_ISOLATION_OFFER__ = offer();
    const { container } = render(<IsolationOfferDialog />);
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    let launched: Promise<unknown> = Promise.resolve();
    await act(async () => {
      launched = launchSession("p-2", "/Users/dev/storefront");
    });
    await waitFor(() => expect(q("isolation-offer")).not.toBeNull());
    expect(container.textContent).toContain("Work Max is already working in storefront");
    expect((q("isolation-worktree") as HTMLInputElement).checked).toBe(true);
    expect((q("isolation-path") as HTMLInputElement).value).toBe("/Users/dev/storefront.juggler/a1b2c3");
    expect(container.textContent).toContain("one can silently overwrite the other's work");
    expect(calls()).toEqual([]);
    fireEvent.change(q("isolation-path")!, { target: { value: "/Users/dev/wt/b" } });
    await act(async () => {
      fireEvent.click(q("isolation-go")!);
      await launched;
    });
    expect(calls()).toEqual(["worktree:juggler/a1b2c3:/Users/dev/wt/b", "p-2"]);
  });

  it("sharing is an explicit choice; a non-git project offers no fake isolation", async () => {
    w().__AIJ_ISOLATION_OFFER__ = offer({
      available: false,
      unavailable_reason: "Unavailable — not a git project",
      proposed_path: null,
      proposed_branch: null,
    });
    const { container } = render(<IsolationOfferDialog />);
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    let launched: Promise<unknown> = Promise.resolve();
    await act(async () => {
      launched = launchSession("p-2", "/Users/dev/notes");
    });
    await waitFor(() => expect(q("isolation-unavailable")).not.toBeNull());
    expect(q("isolation-unavailable")!.textContent).toContain("Unavailable — not a git project");
    expect((q("isolation-worktree") as HTMLInputElement).disabled).toBe(true);
    expect(q("isolation-path")).toBeNull();
    expect(q("isolation-go")!.textContent).toBe("Launch in the shared checkout");
    await act(async () => {
      fireEvent.click(q("isolation-go")!);
      await launched;
    });
    expect(calls()).toEqual(["p-2"]);
  });

  it("cancel launches nothing", async () => {
    w().__AIJ_ISOLATION_OFFER__ = offer();
    const { container } = render(<IsolationOfferDialog />);
    let result: Promise<unknown> = Promise.resolve();
    await act(async () => {
      result = launchSession("p-2", "/Users/dev/storefront").catch((error: Error) => error.message);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='isolation-cancel']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='isolation-cancel']")!);
    });
    expect(await result).toMatch(/Not launched/);
    expect(calls()).toEqual([]);
  });
});

describe("AJ-E1002 removal guard", () => {
  it("needs the folder name typed back before discarding uncommitted work", async () => {
    w().__AIJ_WORKTREE_REMOVAL__ = {
      path: "/Users/dev/storefront.juggler/a1b2c3",
      folder: "a1b2c3",
      branch: "juggler/a1b2c3",
      changed: 2,
      untracked: 1,
      needs_confirmation: true,
      summary: "Discards 2 changed and 1 untracked files — they are not committed anywhere. The branch juggler/a1b2c3 stays with its commits.",
      in_use: false,
    };
    const { container } = render(
      <WorktreeRemoveDialog path="/Users/dev/storefront.juggler/a1b2c3" onClose={() => {}} />,
    );
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    await waitFor(() => expect(q("worktree-remove-summary")).not.toBeNull());
    expect(q("worktree-remove-summary")!.textContent).toContain("Discards 2 changed and 1 untracked files");
    const go = q("worktree-remove-go") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.change(q("worktree-remove-confirm")!, { target: { value: "a1b2c" } });
    expect((q("worktree-remove-go") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(q("worktree-remove-confirm")!, { target: { value: "a1b2c3" } });
    await act(async () => {
      fireEvent.click(q("worktree-remove-go")!);
    });
    expect(calls()).toEqual(["worktree-remove:/Users/dev/storefront.juggler/a1b2c3:a1b2c3"]);
    expect(container.textContent).toContain("The branch juggler/a1b2c3 stays");
  });
});

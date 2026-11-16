/**
 * AJ-E3008 R3 (D-62): the add palette lists every candidate by its kind,
 * each with what it brings or why it can't join and the step that lets it
 * in; an add is a button (Return on it adds, as a click does).
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as poolModel from "../../lib/ipc/pool-model";
import { DESKTOP_NOTE, type Candidate } from "../../lib/new-pool";
import { PaletteMore, type MoreKind } from "./PaletteMore";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const app = (ref: string, blocked: string | null = null): Candidate => ({ ref, kind: "app", label: `App ${ref}`, detail: "payments-api · 2 sessions", group: "open", reach: "later", blocked, fix: blocked ? { label: "Add its project", action: "scope", arg: "proj-2" } : undefined, lastMs: 0 });
const account = (ref: string): Candidate => ({ ref, kind: "account", label: `Account ${ref}`, detail: "Claude Max", group: "all", reach: "later", blocked: null, lastMs: 0 });
const lists = (): Record<MoreKind, Candidate[]> => ({ watched: [], app: [app("a1"), app("a2", "Its project isn't in this pool")], account: [account("c1")], tool: [], mac: [], everyone: [], blocked: [] });

describe("the add palette's other candidates (AJ-E3008 R3)", () => {
  it("lists them by kind with what each brings, a blocked one with why and its step", () => {
    const onFix = vi.fn();
    render(<PaletteMore poolId="pool-1" lists={lists()} onFix={onFix} onAdded={() => undefined} />);
    const more = screen.getByTestId("canvas-palette-more");
    expect(screen.getByTestId("canvas-palette-kind").textContent).toContain("Apps 2");
    const rows = within(more).getAllByTestId("canvas-palette-more-item");
    expect(rows).toHaveLength(2);
    expect(rows[0]!.textContent).toContain("payments-api · 2 sessions");
    expect(rows[1]!.textContent).toContain("Its project isn't in this pool");
    expect(within(rows[1]!).queryByTestId("canvas-palette-more-add")).toBeNull();
    fireEvent.click(within(rows[1]!).getByText("Add its project"));
    expect(onFix).toHaveBeenCalledWith(expect.objectContaining({ ref: "a2" }));
  });

  // The rc26 P1 walk: a desktop App's Add read as if its sessions would act.
  // Round 7 #9 (aij-test run 3): said once for the list, never on each row.
  it("desktop Apps keep their Add; how their sessions take part is said once for the list (AJ-E3013)", () => {
    const desktop = (id: string, label: string): Candidate => ({ ...app(id), label, detail: "none open now", note: DESKTOP_NOTE });
    render(<PaletteMore poolId="pool-1" lists={{ ...lists(), app: [desktop("d1", "Claude app · Work Max"), desktop("d2", "Claude app · Side")] }} onFix={() => undefined} onAdded={() => undefined} />);
    expect(screen.getAllByTestId("canvas-palette-more-item").map((row) => within(row).getByTestId("canvas-palette-more-add"))).toHaveLength(2);
    expect(screen.getAllByTestId("canvas-palette-more-note").map((note) => note.textContent)).toEqual([DESKTOP_NOTE]);
    for (const row of screen.getAllByTestId("canvas-palette-more-item")) expect(row.textContent).not.toContain("a small helper");
  });

  it("adds one with its button, as a member, and says what it brings", async () => {
    const join = vi.spyOn(poolModel, "poolJoin").mockResolvedValue({ pools: [], placements: [] });
    const added = vi.fn();
    render(<PaletteMore poolId="pool-1" lists={lists()} onFix={() => undefined} onAdded={added} />);
    const add = within(screen.getAllByTestId("canvas-palette-more-item")[0]!).getByTestId("canvas-palette-more-add");
    expect(add.tagName).toBe("BUTTON");
    fireEvent.click(add);
    await waitFor(() => expect(join).toHaveBeenCalledWith("pool-1", "a1", "member"));
    await waitFor(() => expect(added).toHaveBeenCalledTimes(1));
  });
});

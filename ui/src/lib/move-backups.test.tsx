/**
 * 0.5.1 F93 vitest (PL1 J5, F-PL1-25): the header's "N to clean up" and
 * Settings › Moves read the backups once, together, and both read again
 * after a move (or a repair) with no click on a Moves row. Before: the
 * header kept its first read (1 backup, or none) while Moves said 2.
 */
import type { JSX } from "react";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCleanupCount } from "./cleanup-count";
import { useMoveBackups } from "./move-backups";
import { switchMoveExecute, type MoveBackupsInfo, type MoveDoneInfo } from "./ipc/switch-move";
import { ownershipRepair } from "./ipc/ownership";

// Two processes left to clean up, told once.
vi.mock("./ipc/processes", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/processes")>()),
  processesCounts: async () => ({ to_clean: 2 }),
  onProcessCounts: () => () => undefined,
}));

const kept = (count: number): MoveBackupsInfo => ({ count, bytes: count * 4096, exact: true, old_count: 0, old_bytes: 0, low_room: false });

const DONE: MoveDoneInfo = {
  move_id: "m2",
  payload: { state: "verified", detail: "" },
  catalog: { state: "verified", detail: "" },
  opened: { state: "verified", detail: "" },
  open_place_id: null,
  conversation: "c1",
  model: { state: "pending", detail: "" },
  notes: [],
};

/** The header's count (2 processes and the backups) and Moves' own. */
function Both(): JSX.Element {
  const header = useCleanupCount();
  const moves = useMoveBackups();
  return (
    <>
      <span data-testid="header">{header}</span>
      <span data-testid="moves">{moves?.count ?? "…"}</span>
    </>
  );
}

afterEach(() => {
  cleanup();
  delete window.__AIJ_MOVE_BACKUPS__;
  delete window.__AIJ_MOVE_DONE__;
});

describe("the backups the header counts and Moves lists (F93)", () => {
  it("one read for both, read again after a move and after a repair", async () => {
    window.__AIJ_MOVE_BACKUPS__ = kept(1);
    render(<Both />);
    await waitFor(() => expect(screen.getByTestId("header").textContent).toBe("3"));
    expect(screen.getByTestId("moves").textContent).toBe("1");
    // A move commits a second backup: no Moves row was clicked.
    window.__AIJ_MOVE_BACKUPS__ = kept(2);
    window.__AIJ_MOVE_DONE__ = DONE;
    await act(async () => {
      await switchMoveExecute("s1", "p2", false);
    });
    await waitFor(() => expect(screen.getByTestId("moves").textContent).toBe("2"));
    expect(screen.getByTestId("header").textContent).toBe("4");
    // A repair keeps one for its Undo.
    window.__AIJ_MOVE_BACKUPS__ = kept(3);
    await act(async () => {
      await ownershipRepair("wt-7");
    });
    await waitFor(() => expect(screen.getByTestId("moves").textContent).toBe("3"));
    expect(screen.getByTestId("header").textContent).toBe("5");
  });
});

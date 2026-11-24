/**
 * AJ-E2325 vitest — the backups kept for Undo, together: Settings' Moves
 * in rich mode (a sticky header and footer with the total), the low-disk
 * offer that names what it frees and clears only on its sheet's confirm,
 * and the clean-up view's line that opens the Moves section.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BackupsCleanupRow, MovesPanel } from "./MovesBackups";
import type { MoveBackupsInfo, MoveRecordInfo } from "../../lib/ipc/switch-move";

const row: MoveRecordInfo = {
  move_id: "m1",
  conversation: "c1",
  title: "Fix the parser",
  from: "A",
  to: "B",
  state: "committed",
  at_ms: 1,
  detail: null,
  can_undo: true,
  undo_why: null,
  can_finish: false,
  has_backup: true,
  kept_until_ms: null,
  backup_bytes: 8_178_892,
  backup_exact: false,
  timeline: [],
  kept: [],
};

const backups = (over: Partial<MoveBackupsInfo>): MoveBackupsInfo => ({ count: 2, bytes: 3 * 1024 ** 3, exact: false, old_count: 1, old_bytes: 2 * 1024 ** 3, low_room: false, ...over });

afterEach(() => {
  cleanup();
  delete window.__AIJ_MOVE_BACKUPS__;
  delete window.__AIJ_MOVE_OLD_CLEARED__;
});

describe("move backups together", () => {
  it("Settings' Moves: a sticky header and footer, the footer with the total", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({});
    render(<MovesPanel moves={[row]} onChanged={() => {}} />);
    const head = await screen.findByTestId("moves-head");
    expect(head.className).toContain("sticky");
    expect(head.className).toContain("top-0");
    expect(head.textContent).toBe("1 move2 move backups");
    const foot = screen.getByTestId("moves-foot");
    expect(foot.className).toContain("sticky");
    expect(foot.className).toContain("bottom-0");
    expect(screen.getByTestId("moves-total").textContent).toBe("2 move backups kept for Undo: frees up to 3.0 GB in all");
    expect(screen.queryByTestId("moves-offer")).toBeNull();
  });

  it("a nearly full disk offers to clear backups older than a day, naming what it frees; Keep clears nothing", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({ low_room: true });
    const changed = vi.fn();
    render(<MovesPanel moves={[row]} onChanged={changed} />);
    expect((await screen.findByTestId("moves-offer")).textContent).toContain("The disk is nearly full. Backups older than a day: frees up to 2.0 GB.");
    fireEvent.click(screen.getByTestId("moves-clear"));
    const sheet = await screen.findByTestId("moves-clear-sheet");
    expect(sheet.textContent).toContain("Clear 1 move backup older than a day?");
    expect(sheet.textContent).toContain("That move can no longer be undone.");
    fireEvent.click(screen.getByRole("button", { name: "Keep" }));
    await waitFor(() => expect(screen.queryByTestId("moves-clear-sheet")).toBeNull());
    expect(changed).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("moves-clear"));
    fireEvent.click(await screen.findByTestId("moves-clear-confirm"));
    expect((await screen.findByTestId("moves-cleared")).textContent).toContain("can no longer be undone");
    expect(changed).toHaveBeenCalled();
  });

  it("D-146: exact sizes (APFS measured) say \"frees <size>\" in the total, the offer and the clean-up line", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({ exact: true, low_room: true });
    const { unmount } = render(<MovesPanel moves={[row]} onChanged={() => {}} />);
    expect((await screen.findByTestId("moves-total")).textContent).toBe("2 move backups kept for Undo: frees 3.0 GB in all");
    expect(screen.getByTestId("moves-offer").textContent).toContain("Backups older than a day: frees 2.0 GB.");
    unmount();
    render(<BackupsCleanupRow />);
    expect((await screen.findByTestId("cleanup-backups")).textContent).toContain("2 move backups kept for Undo, frees 3.0 GB.");
  });

  it("no offer without backups a day old, even on a nearly full disk", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({ low_room: true, old_count: 0, old_bytes: 0 });
    render(<MovesPanel moves={[row]} onChanged={() => {}} />);
    await screen.findByTestId("moves-total");
    expect(screen.queryByTestId("moves-offer")).toBeNull();
  });

  it("the clean-up view says the backups with their size and opens Moves", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({});
    const sections: unknown[] = [];
    const opened = (event: Event) => sections.push(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:open-settings", opened);
    render(<BackupsCleanupRow />);
    expect((await screen.findByTestId("cleanup-backups")).textContent).toContain("Also to clean up: 2 move backups kept for Undo, frees up to 3.0 GB.");
    fireEvent.click(screen.getByTestId("cleanup-backups-open"));
    expect(sections).toEqual([{ section: "moves" }]);
    window.removeEventListener("aij:open-settings", opened);
  });

  it("no backups: the clean-up view says nothing", async () => {
    window.__AIJ_MOVE_BACKUPS__ = backups({ count: 0, bytes: 0, old_count: 0, old_bytes: 0 });
    const { container } = render(<BackupsCleanupRow />);
    await waitFor(() => expect(container.innerHTML).toBe(""));
  });
});

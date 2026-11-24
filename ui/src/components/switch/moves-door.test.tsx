/**
 * AJ-E2321 vitest — the Moves door: each move by its id, from → to, its
 * title, date and state in plain words; Undo… (whose sheet says what it
 * restores, and runs only on its confirm) for a committed or interrupted
 * move, Finish only for one that passed its checks; Copy details carries
 * its timeline; a conversation's page shows only its own moves; no moves:
 * nothing shown.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { JSX } from "react";
import { MoveRows, detailsOf, freesWords, keptWords, stateWords, useMoves } from "./MovesDoor";
import type { MoveRecordInfo } from "../../lib/ipc/switch-move";

const move = (over: Partial<MoveRecordInfo>): MoveRecordInfo => ({
  move_id: "1700000000000-a1",
  conversation: "conv-1",
  title: "Fix the parser",
  from: "Claude app · A",
  to: "Claude app · B",
  state: "committed",
  at_ms: 1_700_000_000_000,
  detail: null,
  can_undo: true,
  undo_why: null,
  can_finish: false,
  has_backup: false,
  kept_until_ms: null,
  backup_bytes: null,
  backup_exact: false,
  recovered: null,
  prompts: [],
  repair: false,
  timeline: ["phase: phase planned", "intent: step 1"],
  kept: [],
  ...over,
});

/** The door as each surface draws it: its rows only when there are any. */
function MovesDoor({ conversation = null }: { conversation?: string | null }): JSX.Element | null {
  const [moves, reload] = useMoves(conversation);
  if (!moves || moves.length === 0) return null;
  return <MoveRows moves={moves} onChanged={reload} />;
}

afterEach(() => {
  cleanup();
  delete window.__AIJ_MOVES__;
  delete window.__AIJ_MOVE_UNDONE__;
  delete window.__AIJ_MOVE_FINISHED__;
  delete window.__AIJ_MOVE_BACKUP_DELETED__;
});

describe("Moves door", () => {
  it("shows nothing when there are no moves", async () => {
    window.__AIJ_MOVES__ = [];
    const { container } = render(<MovesDoor />);
    await waitFor(() => expect(container.innerHTML).toBe(""));
  });

  it("says each move's title, from → to, date and state, with Undo for a committed one", async () => {
    window.__AIJ_MOVES__ = [move({})];
    render(<MovesDoor />);
    const row = await screen.findByTestId("move-row");
    expect(row.textContent).toContain("“Fix the parser” · Claude app · A → Claude app · B");
    expect(screen.getByTestId("move-row-state").textContent).toContain("Moved");
    expect(screen.getByTestId("move-undo")).toBeTruthy();
    expect(screen.queryByTestId("move-finish")).toBeNull();
    // No ids or internal words on screen.
    expect(row.textContent).not.toContain("1700000000000-a1");
  });

  it("offers Finish and Undo for an interrupted move that passed its checks, Undo only otherwise", async () => {
    window.__AIJ_MOVES__ = [
      move({ move_id: "m-v", state: "interrupted", can_finish: true, detail: "it stopped after its checks passed" }),
      move({ move_id: "m-a", state: "interrupted", can_finish: false, detail: "it stopped at step 3 of 5" }),
    ];
    render(<MovesDoor />);
    const rows = await screen.findAllByTestId("move-row");
    expect(rows[0]!.querySelector("[data-testid=move-finish]")).toBeTruthy();
    expect(rows[1]!.querySelector("[data-testid=move-finish]")).toBeNull();
    expect(rows[1]!.querySelector("[data-testid=move-undo]")).toBeTruthy();
    expect(stateWords(move({ state: "interrupted", can_finish: true }))).toBe("Interrupted: finish or undo");
  });

  it("Undo's sheet says what it restores and runs only on its confirm, by the move's id", async () => {
    window.__AIJ_MOVES__ = [move({})];
    window.__AIJ_MOVE_UNDONE__ = { words: "Undone: the conversation is back where it was.", kept: "", notes: [] };
    render(<MovesDoor />);
    fireEvent.click(await screen.findByTestId("move-undo"));
    const sheet = await screen.findByTestId("move-undo-sheet");
    expect(sheet.textContent).toContain("It goes back to Claude app · A");
    expect(sheet.textContent).toContain("Kept aside, never deleted");
    expect(screen.queryByTestId("move-said")).toBeNull();
    fireEvent.click(screen.getByTestId("move-undo-confirm"));
    expect((await screen.findByTestId("move-said")).textContent).toContain("Undone");
  });

  it("Finish says it finished", async () => {
    window.__AIJ_MOVES__ = [move({ state: "interrupted", can_finish: true })];
    render(<MovesDoor />);
    fireEvent.click(await screen.findByTestId("move-finish"));
    expect((await screen.findByTestId("move-said")).textContent).toContain("Finished");
  });

  it("says why a committed move can't be undone, and offers no Undo", async () => {
    window.__AIJ_MOVES__ = [move({ can_undo: false, undo_why: "its backup was let go" })];
    render(<MovesDoor />);
    const row = await screen.findByTestId("move-row");
    expect(row.textContent).toContain("Can't be undone: its backup was let go.");
    expect(screen.queryByTestId("move-undo")).toBeNull();
  });

  it("on a conversation's page shows only its own moves", async () => {
    window.__AIJ_MOVES__ = [move({}), move({ move_id: "other", conversation: "conv-2", title: "Other" })];
    render(<MovesDoor conversation="conv-2" />);
    await waitFor(() => expect(screen.getAllByTestId("move-row")).toHaveLength(1));
    expect(screen.getByTestId("move-row").textContent).toContain("Other");
  });

  it("Copy details carries the move's id and its timeline", async () => {
    const text = detailsOf(move({}));
    expect(text).toContain("Move 1700000000000-a1");
    expect(text).toContain("2. intent: step 1");
    const write = vi.fn(() => Promise.resolve());
    Object.assign(navigator, { clipboard: { writeText: write } });
    window.__AIJ_MOVES__ = [move({})];
    render(<MovesDoor />);
    fireEvent.click(await screen.findByTestId("move-copy"));
    expect(write).toHaveBeenCalledWith(text);
  });

  it("AJ-E2325: a finished move's backup says until when it is kept", () => {
    expect(keptWords(move({}))).toBeNull();
    expect(keptWords(move({ has_backup: true, kept_until_ms: null }))).toBe("Backup kept until you delete it");
    expect(keptWords(move({ has_backup: true, kept_until_ms: new Date(2026, 10, 30).getTime() }))).toBe("Backup kept until Nov 30");
  });

  it("AJ-E2325: a kept backup says what deleting it frees, a ceiling where clones share blocks", async () => {
    expect(freesWords(8_178_892, false)).toBe("frees up to 8 MB");
    expect(freesWords(3 * 1024 ** 3, true)).toBe("frees 3.0 GB");
    window.__AIJ_MOVES__ = [move({ has_backup: true, kept_until_ms: new Date(2026, 10, 30).getTime(), backup_bytes: 8_178_892, backup_exact: false })];
    render(<MovesDoor />);
    expect((await screen.findByTestId("move-kept-until")).textContent).toBe("Backup kept until Nov 30 · frees up to 8 MB");
  });

  it("D-146: a backup measured exactly (APFS) says what deleting it frees, no \"up to\"", async () => {
    window.__AIJ_MOVES__ = [move({ has_backup: true, kept_until_ms: new Date(2026, 10, 30).getTime(), backup_bytes: 8192, backup_exact: true })];
    render(<MovesDoor />);
    expect((await screen.findByTestId("move-kept-until")).textContent).toBe(`Backup kept until Nov 30 · ${freesWords(8192, true)}`);
    expect(screen.getByTestId("move-kept-until").textContent).not.toContain("up to");
  });

  it("AJ-E2325: Delete backup says Undo goes with it, Keep cancels, and it deletes only on its confirm", async () => {
    window.__AIJ_MOVES__ = [move({ has_backup: true, kept_until_ms: new Date(2026, 10, 30).getTime() })];
    render(<MovesDoor />);
    expect((await screen.findByTestId("move-kept-until")).textContent).toBe("Backup kept until Nov 30");
    fireEvent.click(screen.getByTestId("move-delete"));
    expect((await screen.findByTestId("move-delete-sheet")).textContent).toContain("You can no longer undo this move.");
    fireEvent.click(screen.getByText("Keep"));
    await waitFor(() => expect(screen.queryByTestId("move-delete-sheet")).toBeNull());
    expect(screen.queryByTestId("move-said")).toBeNull();
    fireEvent.click(screen.getByTestId("move-delete"));
    fireEvent.click(await screen.findByTestId("move-delete-confirm"));
    expect((await screen.findByTestId("move-said")).textContent).toContain("can no longer be undone");
  });

  it("0.5.1 F91: a repair's row names the conversation and says Repaired ownership; its Undo sheet is the repair's", async () => {
    window.__AIJ_MOVES__ = [move({ repair: true, title: "Tidy the checkout page", from: "Claude app · work", to: "Claude app · personal", has_backup: true, kept_until_ms: new Date(2026, 10, 30).getTime() })];
    render(<MovesDoor />);
    const row = await screen.findByTestId("move-row");
    expect(row.textContent).toContain("“Tidy the checkout page” · Claude app · work → Claude app · personal");
    expect(screen.getByTestId("move-row-state").textContent).toContain("Repaired ownership");
    expect(row.textContent).not.toContain("A conversation");
    expect(row.textContent).not.toContain("Moved");
    fireEvent.click(screen.getByTestId("move-undo"));
    const sheet = await screen.findByTestId("move-undo-sheet");
    expect(sheet.textContent).toContain("Undo the repair of “Tidy the checkout page”?");
    expect(sheet.textContent).toContain("Its worktree belongs to Claude app · work again");
    expect(sheet.textContent).toContain("The worktree's files");
    expect(sheet.textContent).not.toContain("since the move");
    fireEvent.click(screen.getByText("Cancel"));
    fireEvent.click(screen.getByTestId("move-delete"));
    expect((await screen.findByTestId("move-delete-sheet")).textContent).toContain("You can no longer undo this repair.");
    expect(detailsOf(move({ repair: true })).startsWith("Repair ")).toBe(true);
  });

  it("0.5.1 F92: a move AI Juggler stopped during says so, and which Apps started again", async () => {
    const recovered = "AI Juggler stopped during this move, before it changed anything, so nothing was moved. Claude app · A and Claude app · B were started again";
    window.__AIJ_MOVES__ = [move({ state: "undone", can_undo: false, recovered })];
    render(<MovesDoor />);
    expect((await screen.findByTestId("move-row-state")).textContent).toContain("Interrupted: nothing moved");
    expect(screen.getByTestId("move-recovered").textContent).toBe(`${recovered}.`);
    expect(screen.queryByTestId("move-undo")).toBeNull();
    expect(stateWords(move({ state: "committed", recovered }))).toBe("Interrupted, then finished");
  });
});

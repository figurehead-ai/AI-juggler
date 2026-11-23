/**
 * AJ-E2328 vitest — Settings › Desktop apps › Moving conversations: each
 * app's version and whether moves were checked on it (req 9 c); a split
 * conversation's row with Repair…, whose sheet names the Apps it quits and
 * says the worktree's files are never touched, and repairs only on the
 * confirm (req 7); the info line for a worktree another App tidies; no
 * internal words on screen; nothing found and no app: no section.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MovingSection, checkedWords } from "./MovingSection";
import type { OwnershipInfo } from "../../lib/ipc/ownership";

const finding = {
  key: "wt-7@lab-a",
  title: "Fix the parser",
  words: "Its worktree wt-7 still belongs to Claude app · A, so Claude app · B can't write there and may make a fresh one",
  worktree: "wt-7",
  holder: "Claude app · B",
  quits: ["Claude app · A", "Claude app · B"],
  notes: ["This conversation has two worktrees; both are kept"],
};

const seeded = (info: OwnershipInfo) => {
  window.__AIJ_OWNERSHIP__ = info;
};

afterEach(() => {
  cleanup();
  delete window.__AIJ_OWNERSHIP__;
  delete window.__AIJ_OWNERSHIP_REPAIRED__;
});

describe("Moving conversations", () => {
  it("says the Claude app's version and whether moves were checked on it", async () => {
    expect(checkedWords("Claude app", "5.11492.5", true)).toBe("Claude app 5.11492.5 · moves checked");
    expect(checkedWords("Claude app", "5.11610.2", false)).toBe("Claude app 5.11610.2 · not yet checked");
    seeded({ apps: [{ app_id: "claude-app", app: "Claude app", version: "5.11610.2", checked: false }], findings: [], tidied: [], unreadable: null });
    render(<MovingSection />);
    const row = await screen.findByTestId("moving-app-claude-app");
    expect(row.textContent).toContain("Claude app 5.11610.2 · not yet checked");
    expect(row.textContent).toContain("Copy still works");
    expect(screen.queryByTestId("moving-finding")).toBeNull();
  });

  it("nothing to show: no section", async () => {
    seeded({ apps: [], findings: [], tidied: [], unreadable: null });
    const { container } = render(<MovingSection />);
    await act(async () => {});
    expect(container.textContent).toBe("");
  });

  it("a split conversation's Repair names the Apps it quits, never touches the files, and acts only on the confirm", async () => {
    seeded({
      apps: [{ app_id: "claude-app", app: "Claude app", version: "5.11492.5", checked: true }],
      findings: [finding],
      tidied: [{ worktree: "wt-9", words: "Claude app · A also tidies this folder; keep Claude app · B running in it or commit your work" }],
      unreadable: null,
    });
    window.__AIJ_OWNERSHIP_REPAIRED__ = { words: "Repaired: the worktree wt-7 belongs to Claude app · B now. Its files weren't touched.", notes: [] };
    render(<MovingSection />);
    const row = await screen.findByTestId("moving-finding");
    expect(row.textContent).toContain("Fix the parser");
    expect(row.textContent).toContain("This conversation has two worktrees; both are kept.");
    expect(screen.getByTestId("moving-tidied").textContent).toContain("also tidies this folder; keep Claude app · B running in it or commit your work.");
    fireEvent.click(screen.getByTestId("moving-finding-repair"));
    const sheet = await screen.findByTestId("ownership-repair-sheet");
    expect(screen.getByTestId("ownership-repair-quits").textContent).toContain("Claude app · A and Claude app · B");
    expect(screen.getByTestId("ownership-repair-files").textContent).toContain("The worktree's files");
    for (const word of ["lease", "sighting", "untracked"]) expect(document.body.textContent?.toLowerCase()).not.toContain(word);
    expect(sheet.textContent).toContain("Cancel");
    // The repair is found done: the next check has nothing.
    seeded({ apps: [{ app_id: "claude-app", app: "Claude app", version: "5.11492.5", checked: true }], findings: [], tidied: [], unreadable: null });
    await act(async () => {
      fireEvent.click(screen.getByTestId("ownership-repair-confirm"));
    });
    await waitFor(() => expect(screen.queryByTestId("ownership-repair-sheet")).toBeNull());
    await waitFor(() => expect(screen.getByTestId("moving-repaired").textContent).toContain("Its files weren't touched."));
    expect(screen.queryByTestId("moving-finding")).toBeNull();
  });

  it("Cancel repairs nothing", async () => {
    seeded({ apps: [], findings: [finding], tidied: [], unreadable: null });
    render(<MovingSection />);
    fireEvent.click(await screen.findByTestId("moving-finding-repair"));
    await screen.findByTestId("ownership-repair-sheet");
    fireEvent.click(screen.getByText("Cancel"));
    await waitFor(() => expect(screen.queryByTestId("ownership-repair-sheet")).toBeNull());
    expect(screen.getByTestId("moving-finding")).toBeTruthy();
    expect(screen.queryByTestId("moving-repaired")).toBeNull();
  });
});

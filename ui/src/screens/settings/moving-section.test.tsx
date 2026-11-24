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
import { MovingSection, checkedWords, quitsWords } from "./MovingSection";
import type { OwnershipInfo } from "../../lib/ipc/ownership";
import { PLACES_CHANGED, type KnownPlaceInfo } from "../../lib/ipc/places";

const A = "app:claude-app:/Users/someone/Profiles/a";
const B = "app:claude-app:/Users/someone/Profiles/b";

/** A remembered App, running when `pid` is given. */
function place(id: string, label: string, pid: number | null): KnownPlaceInfo {
  return { recent: [], id, app_id: "claude-app", app: "Claude", instance: label, profile: label, label, app_pid: pid, lost: false, keep_alive: false, paused: null, account_id: null, seen_ms: 1, cannot_start: null, found: false };
}

/** The Apps as the rail knows them now (both running unless said), the
 * preview's places seam. */
const placesNow = (a: number | null = 11, b: number | null = 12) => {
  Object.assign(window, { __AIJ_PLACES__: { places: [place(A, "Claude app · A", a), place(B, "Claude app · B", b)] } });
};

const finding = {
  key: "wt-7@lab-a",
  title: "Fix the parser",
  words: "Its worktree wt-7 still belongs to Claude app · A, so Claude app · B can't write there and may make a fresh one",
  worktree: "wt-7",
  holder: "Claude app · B",
  quits: ["Claude app · A", "Claude app · B"],
  apps: [
    { place_id: A, name: "Claude app · A" },
    { place_id: B, name: "Claude app · B" },
  ],
  notes: ["This conversation has two worktrees; both are kept"],
};

const seeded = (info: OwnershipInfo) => {
  window.__AIJ_OWNERSHIP__ = info;
};

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
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
    placesNow();
    render(<MovingSection />);
    const row = await screen.findByTestId("moving-finding");
    expect(row.textContent).toContain("Fix the parser");
    expect(row.textContent).toContain("This conversation has two worktrees; both are kept.");
    expect(screen.getByTestId("moving-tidied").textContent).toContain("also tidies this folder; keep Claude app · B running in it or commit your work.");
    fireEvent.click(screen.getByTestId("moving-finding-repair"));
    const sheet = await screen.findByTestId("ownership-repair-sheet");
    await waitFor(() => expect(screen.getByTestId("ownership-repair-quits").textContent).toContain("Claude app · A and Claude app · B"));
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

  // 0.5.1 F90 (PL1 I13, F-PL1-22): the check ran while both Apps were
  // closed; they were started with Settings open. The sheet reads which
  // run when it opens, and keeps it current while it is open.
  it("the sheet names the Apps running when it opens, not when Settings did, and keeps it current", async () => {
    seeded({ apps: [], findings: [{ ...finding, quits: [] }], tidied: [], unreadable: null });
    placesNow(null, null);
    render(<MovingSection />);
    const repair = await screen.findByTestId("moving-finding-repair");
    // Both started after the check (Dev's own Start), Settings still open.
    placesNow(46009, 46598);
    fireEvent.click(repair);
    await screen.findByTestId("ownership-repair-sheet");
    await waitFor(() => expect(screen.getByTestId("ownership-repair-quits").textContent).toBe(quitsWords(["Claude app · A", "Claude app · B"])));
    expect(screen.getByTestId("ownership-repair-confirm").hasAttribute("disabled")).toBe(false);
    // One quits while the sheet is open: heard at once.
    placesNow(null, 46598);
    act(() => {
      window.dispatchEvent(new CustomEvent(PLACES_CHANGED));
    });
    await waitFor(() => expect(screen.getByTestId("ownership-repair-quits").textContent).toBe(quitsWords(["Claude app · B"])));
  });
});

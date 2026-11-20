/**
 * AJ-E2311 item 2 (the 0.4.6 walks: every New session opened on another
 * of the maintainer's projects and App, Recent reshuffled, and the sheet's
 * layout shifted when its memory line came): New session opens in the
 * project of the page you're on, else where you last started one here;
 * Recent holds its order while the sheet is open; this Mac's line is there
 * in its first frame once read, and the sheet grows downward from the top.
 */
import { act, cleanup, render, renderHook, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import { hereProject, recentFolders, steadyRecent } from "../lib/new-session-folder";
import { useNewSessionSheet, standOffer, withdrawOffer } from "../lib/new-session-sheet";
import { machineAdvices } from "../lib/kept-reads";
import { clearSeams, instance, project, recipe, setSeam } from "../test/fixtures";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { MachineAdviceInfo } from "../lib/machine";

const NOW = Date.now();
const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-lab", "aij-test"), folder("claude-work", "Work Max")];
const RECIPES = [recipe({ id: "p-lab", account_id: "claude-lab", account_label: "aij-test" }), recipe({ id: "p-work", account_id: "claude-work", account_label: "Work Max" })];
// The maintainer's own repo: its tools' history wrote a second ago.
const REAL = project({ id: "proj-real", root: "/Users/you/Dev/real", name: "real", history_last_used_ms: NOW - 1_000, last_activity_ms: null, history_sessions: 40, accounts_used: ["Work Max"] });
// The lab folder: a session started here ten minutes ago.
const LAB = project({ id: "proj-web", root: "/Users/you/lab/web", name: "web", last_activity_ms: NOW - 600_000, history_last_used_ms: NOW - 590_000, accounts_used: ["aij-test"] });
const STARTED = instance({ id: "s-lab", project_id: "proj-web", account_id: "claude-lab", account_label: "aij-test", started_at_ms: NOW - 600_000, cwd: "/Users/you/lab/web", status: "finished" });

beforeEach(() => {
  setSeam("__AIJ_RECIPES__", RECIPES);
  setSeam("__AIJ_RANKING__", []);
  vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  withdrawOffer(true);
  clearSeams("__AIJ_RECIPES__", "__AIJ_RANKING__", "__AIJ_ADVICE__");
});

const mountSheet = (projects = [REAL, LAB], directory: string | null = null) =>
  render(<NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={RECIPES} sessions={[STARTED]} initialDirectory={directory} onClose={() => {}} onLaunched={() => {}} />);

describe("New session starts on what fits", () => {
  it("with nothing open, where you last started one here, on the App used there, not the repo a tool wrote in last", async () => {
    mountSheet();
    const sentence = await screen.findByTestId("new-session-sentence");
    expect(sentence.textContent).toMatch(/^Start in web on Claude Code CLI · aij-test/);
  });

  it("Recent: the starts here first; and it holds its order while open, however the histories move", async () => {
    expect(recentFolders([REAL, LAB], [STARTED])).toEqual(["proj-web", "proj-real"]);
    // Without starts here, as before: worked in last first.
    expect(recentFolders([REAL, LAB], [])).toEqual(["proj-real", "proj-web"]);
    // Held: a new history time doesn't move it; a project gone drops out; a new one fills in.
    const later = { ...LAB, history_last_used_ms: NOW };
    expect(steadyRecent(["proj-real", "proj-web"], recentFolders([REAL, later], []), [REAL, later])).toEqual(["proj-real", "proj-web"]);
    const other = project({ id: "proj-api", root: "/Users/you/api", name: "api", last_activity_ms: NOW });
    expect(steadyRecent(["proj-real", "proj-web"], recentFolders([REAL, other], []), [REAL, other])).toEqual(["proj-real", "proj-api"]);
  });

  it("the sheet keeps its Recent order when the projects are read again", async () => {
    const view = mountSheet([REAL, LAB]);
    await screen.findByTestId("new-session-sentence");
    // Read again: the real repo's history moved, and so did the lab's start.
    view.rerender(<NewSessionModal projects={[{ ...REAL, history_last_used_ms: NOW + 5_000 }, LAB]} accounts={ACCOUNTS} recipes={RECIPES} sessions={[STARTED]} initialDirectory={null} onClose={() => {}} onLaunched={() => {}} />);
    expect(screen.getByTestId("new-session-sentence").textContent).toMatch(/^Start in web /);
  });

  it("the project of the page you're on: a project's, a session's (its project, else its folder), a pool's; none elsewhere", () => {
    const pools = [{ id: "pool-1", project_id: "proj-web", scope: ["proj-web"], scope_all: false }];
    const inputs = { pools, projects: [REAL, LAB], sessions: [STARTED, instance({ id: "s-loose", project_id: null, cwd: "/tmp/scratch" })] };
    expect(hereProject({ kind: "project", id: "proj-real" }, inputs)).toBe("/Users/you/Dev/real");
    expect(hereProject({ kind: "session", id: "s-lab" }, inputs)).toBe("/Users/you/lab/web");
    expect(hereProject({ kind: "session", id: "s-loose" }, inputs)).toBe("/tmp/scratch");
    expect(hereProject({ kind: "conversation", projectId: "proj-real" }, inputs)).toBe("/Users/you/Dev/real");
    expect(hereProject({ kind: "pool", id: "pool-1" }, inputs)).toBe("/Users/you/lab/web");
    expect(hereProject({ kind: "none" }, inputs)).toBeNull();
  });

  it("opened from a page it starts there, but a standing Start another still wins", () => {
    const sheet = renderHook(() => useNewSessionSheet());
    act(() => sheet.result.current.show({ here: "/Users/you/lab/web" }));
    expect(sheet.result.current).toMatchObject({ open: true, dir: "/Users/you/lab/web" });
    standOffer({ dir: "/Users/you/Dev/real", after: { roots: ["/Users/you/lab/web"], name: "web", app: null }, sessionId: "s1", toastId: "t" });
    act(() => sheet.result.current.show({ here: "/Users/you/lab/web" }));
    expect(sheet.result.current.dir).toBe("/Users/you/Dev/real");
  });
});

describe("New session doesn't shift", () => {
  const tight: MachineAdviceInfo = { recipe_id: "p-lab", verdict: "tight", words: "Tight — this Mac can just take it", reasons: ["memory is short"], projected_bytes: 2_100_000_000, based_on: 6, relief_session_id: null, relief_bytes: null };

  it("this Mac's line, once read, is there in the sheet's first frame", async () => {
    setSeam("__AIJ_ADVICE__", [tight]);
    await machineAdvices.load();
    mountSheet();
    // No wait: the first frame has it.
    expect(within(screen.getByTestId("launch-preselected")).getByTestId("machine-advice")).toBeTruthy();
  });

  it("the sheet sits at the top and grows downward", () => {
    mountSheet();
    expect(screen.getByTestId("new-session-sheet").className).toContain("items-start");
  });
});

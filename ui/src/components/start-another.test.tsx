/**
 * D-96 F (persona D5, Devon): after Start, New session offers "Start
 * another: <next project>": the next project worked in, none started in
 * this run; taken, it opens there with the usual-choice rule run again
 * for that folder (room checked again, D-31 kept), and the App the user
 * chose kept, after the project's own "Always use" and its own Account
 * (D-103.7: never carried into another Account's repo). ⌘↩ starts it as
 * it did the first.
 */
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { offerFollows, standingOffer, useNewSessionSheet, withdrawOffer, type StartedBefore } from "../lib/new-session-sheet";
import { resetToasts, ToastHost } from "./Toast";

type Next = { dir: string; after: StartedBefore };

const seam = (values: Record<string, unknown>) => Object.assign(window, values);

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];

const recipe = (id: string, account_id: string, account_label: string): LaunchRecipeInfo => ({ id, name: "Main", account_id, account_label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null });
const RECIPES = [recipe("p-1", "claude-work", "Work Max"), recipe("p-3", "claude-side", "Side")];

const project = (name: string, at: number, used: string[]): ProjectInfo => ({
  id: `proj-${name}`,
  root: `/Users/dev/${name}`,
  name,
  last_activity_ms: at,
  accounts_used: used,
  history_sessions: 3,
  history_last_used_ms: null,
  repo: null,
  active_instance: null,
  instance_count: 0,
  missing: false,
});
const PROJECTS = [project("api", Date.now() - 12 * 60_000, ["Work Max"]), project("web", Date.now() - 86_400_000, ["Side"])];

const display = (account: string, left: number) => ({
  account,
  weekly: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: Date.now() + 3 * 3_600_000, badge: "measured", measured_at_ms: Date.now() },
  five_hour: null,
  forecast: { state: "learning", badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
});

beforeEach(() => seam({ __AIJ_RECIPES__: RECIPES, __AIJ_QUOTA_DISPLAY__: [], __AIJ_RANKING__: [] }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  withdrawOffer(true);
  resetToasts();
});

const mountSheet = ({ projects = PROJECTS, dir = null as string | null, account = null as string | null, after = null as StartedBefore | null, onStartAnother = undefined as ((at: Next) => void) | undefined } = {}) => {
  const launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
  const launched = vi.fn();
  render(
    <>
      <NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={dir} initialAccount={account} onClose={() => {}} onLaunched={launched} after={after} onStartAnother={onStartAnother} />
      <ToastHost />
    </>,
  );
  return { launchSpy, launched };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;

// Persona D5: a second repo took Change folder and two more clicks.
describe("start another (D-96 F)", () => {
  const api = "/Users/dev/api";
  const web = "/Users/dev/web";

  it("after Start, a toast offers the next project; taking it opens New session there", async () => {
    const again = vi.fn();
    const { launched } = mountSheet({ onStartAnother: again });
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launched).toHaveBeenCalled());
    const toast = await screen.findByTestId("start-another-toast");
    expect(toast.textContent).toContain("Started in api");
    expect(toast.textContent).toContain("on Claude Code CLI · Work Max");
    const offer = within(toast).getByTestId("toast-action");
    expect(offer.textContent).toBe("Start another: web");
    fireEvent.click(offer);
    expect(again).toHaveBeenCalledWith({ dir: web, after: { roots: [api], name: "api", app: null } });
  });

  it("opened there, the usual-choice rule runs for that folder, and ⌘↩ starts it", async () => {
    const { launchSpy } = mountSheet({ dir: web, after: { roots: [api], name: "api", app: null }, onStartAnother: () => {} });
    expect(sentence()).toBe("Start in web on Claude Code CLI · Side — the only account used here");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", web, undefined));
    // Both started: nothing left to offer.
    await waitFor(() => expect(launchSpy).toHaveBeenCalled());
    expect(screen.queryByTestId("start-another-toast")).toBeNull();
  });

  it("the App the user chose is kept for the next, and said so", async () => {
    const again = vi.fn<(at: Next) => void>();
    mountSheet({ account: "claude-side", onStartAnother: again });
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    fireEvent.click(within(await screen.findByTestId("start-another-toast")).getByTestId("toast-action"));
    const at = again.mock.calls[0]![0];
    expect(at.after.app).toBe("claude-side");
    cleanup();
    mountSheet({ projects: PROJECTS.map((row) => ({ ...row, accounts_used: ["Side", "Work Max"] })), dir: at.dir, after: at.after });
    // web's history names both; the App chosen for api is kept over it.
    expect(sentence()).toBe("Start in web on Claude Code CLI · Side — as you chose for api");
  });

  it("D-103.7: the next repo's own Account wins over the App kept from the start before", () => {
    mountSheet({ projects: PROJECTS.map((row) => ({ ...row, accounts_used: ["Work Max"] })), dir: web, after: { roots: [api], name: "api", app: "claude-side" } });
    // Only Work Max ever ran in web: Side, chosen for api, isn't carried over.
    expect(sentence()).toBe("Start in web on Claude Code CLI · Work Max — the only account used here");
  });

  it("“Always use for web” still wins over the kept App", () => {
    const fixed = PROJECTS.map((row) => (row.name === "web" ? { ...row, default_account_id: "claude-work" } : row));
    mountSheet({ projects: fixed, dir: web, after: { roots: [api], name: "api", app: "claude-side" } });
    expect(sentence()).toBe("Start in web on Claude Code CLI · Work Max — your default here");
  });

  it("room is checked again for the next: the kept App now out is kept, warned and the other offered (D-31)", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-side", 0), display("claude-work", 80)] });
    mountSheet({ projects: PROJECTS.map((row) => ({ ...row, accounts_used: ["Side", "Work Max"] })), dir: web, after: { roots: [api], name: "api", app: "claude-side" } });
    expect(await screen.findByTestId("launch-out-of-room")).toBeTruthy();
    expect(sentence()).toBe("Start in web on Claude Code CLI · Side — as you chose for api");
    expect(screen.getByTestId("launch-use-roomier").textContent).toBe("Use Claude Code CLI · Work Max (80% left)");
  });

  // The 0.4.1 persona walk: the 10 s toast was gone before the page
  // under it was read.
  it("the offer stands until the user works elsewhere, and ⌘N meanwhile opens where it would", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { launched } = mountSheet({ onStartAnother: () => {} });
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launched).toHaveBeenCalled());
    await screen.findByTestId("start-another-toast");
    act(() => void vi.advanceTimersByTime(10 * 60_000));
    expect(screen.getByTestId("start-another-toast")).toBeTruthy();
    // Reading the session it started keeps it.
    act(() => offerFollows("s1"));
    expect(screen.getByTestId("start-another-toast")).toBeTruthy();
    // ⌘N now: New session opens on the next project, and the offer is taken.
    const sheet = renderHook(() => useNewSessionSheet());
    act(() => sheet.result.current.show({ dir: null }));
    expect(sheet.result.current).toMatchObject({ open: true, dir: web, after: { roots: [api], name: "api", app: null } });
    expect(standingOffer()).toBeNull();
    await waitFor(() => expect(screen.queryByTestId("start-another-toast")).toBeNull());
    // Offered again, then the user goes elsewhere: it goes.
    sheet.unmount();
    cleanup();
    const again = mountSheet({ onStartAnother: () => {} });
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(again.launched).toHaveBeenCalled());
    await screen.findByTestId("start-another-toast");
    act(() => offerFollows("another-session"));
    await waitFor(() => expect(screen.queryByTestId("start-another-toast")).toBeNull());
    const later = renderHook(() => useNewSessionSheet());
    act(() => later.result.current.show({ dir: null }));
    expect(later.result.current.dir).toBeNull();
  });

  it("no project left, or no host to open it: no offer", async () => {
    const { launched } = mountSheet({ projects: [PROJECTS[0]!], onStartAnother: () => {} });
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launched).toHaveBeenCalled());
    expect(screen.queryByTestId("start-another-toast")).toBeNull();
    cleanup();
    const second = mountSheet();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(second.launched).toHaveBeenCalled());
    expect(screen.queryByTestId("start-another-toast")).toBeNull();
  });
});

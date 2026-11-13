/**
 * D-103.7 in the New session sheet (persona R2, P4): a repo whose only
 * Account opens on it, said so ("— the only account used here"), and the
 * binding is offered right there in one click ("Always start api on …"),
 * not only inside Change app; taken, it's the project's default. A repo
 * several Accounts ran in (the R2 re-walk's trap) says how many beside
 * its guess and offers the binding for the App chosen, following Change
 * app; one already bound offers nothing. The pure rule's cases are
 * `lib/new-session-own.test.ts`.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as accountsModule from "../lib/ipc/accounts";
import * as launchModule from "../lib/ipc/launch";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const seam = (values: Record<string, unknown>) => Object.assign(window, values);

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];
const recipe = (id: string, account_id: string, account_label: string): LaunchRecipeInfo => ({ id, name: "Main", account_id, account_label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null });
const RECIPES = [recipe("p-1", "claude-work", "Work Max"), recipe("p-3", "claude-side", "Side")];

const project = (used: string[], extra: Partial<ProjectInfo> = {}): ProjectInfo => ({
  id: "proj-api",
  root: "/Users/dev/api",
  name: "api",
  last_activity_ms: Date.now() - 12 * 60_000,
  accounts_used: used,
  history_sessions: 3,
  history_last_used_ms: null,
  repo: null,
  active_instance: null,
  instance_count: 0,
  missing: false,
  ...extra,
});

beforeEach(() => seam({ __AIJ_RECIPES__: RECIPES, __AIJ_QUOTA_DISPLAY__: [], __AIJ_RANKING__: [] }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const mountSheet = (projects: ProjectInfo[], onProjectsChanged?: () => void) => {
  const launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
  render(<NewSessionModal projects={projects} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={null} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} onProjectsChanged={onProjectsChanged} />);
  return { launchSpy };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;

describe("a repo's own Account (D-103.7)", () => {
  it("opens on the only Account that ran there, said so, and offers the binding in one click", async () => {
    const setSpy = vi.spyOn(accountsModule, "projectSetDefaultAccount").mockResolvedValue();
    const reload = vi.fn();
    mountSheet([project(["Work Max"])], reload);
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    const offer = screen.getByTestId("launch-always");
    expect(offer.textContent).toMatch(/^Always start api on .+/);
    fireEvent.click(offer);
    await waitFor(() => expect(setSpy).toHaveBeenCalledWith("proj-api", "claude-work"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — your default here"));
    // Bound now: nothing more to offer.
    expect(screen.queryByTestId("launch-always")).toBeNull();
    expect(reload).toHaveBeenCalled();
  });

  it("several Accounts ran there: the guess says how many, and the binding is offered for the App chosen, following Change app", async () => {
    mountSheet([project(["Side", "Work Max"])]);
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — used here before; 2 accounts worked here");
    const first = screen.getByTestId("launch-always").textContent;
    expect(first).toMatch(/^Always start api on .+/);
    fireEvent.click(screen.getByTestId("change-app"));
    fireEvent.click(await screen.findByTestId("change-app-claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    // Picked by the user: no why to count beside; the offer follows the pick.
    await waitFor(() => expect(screen.getByTestId("launch-always").textContent).not.toBe(first));
    expect(screen.getByTestId("launch-always").textContent).toMatch(/^Always start api on .+/);
  });

  it("several: taking the offer binds the App chosen; opened again, it's the default and nothing is offered", async () => {
    const setSpy = vi.spyOn(accountsModule, "projectSetDefaultAccount").mockResolvedValue();
    mountSheet([project(["Side", "Work Max"])]);
    fireEvent.click(screen.getByTestId("change-app"));
    fireEvent.click(await screen.findByTestId("change-app-claude-side"));
    fireEvent.click(await screen.findByTestId("launch-always"));
    await waitFor(() => expect(setSpy).toHaveBeenCalledWith("proj-api", "claude-side"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here"));
    expect(screen.queryByTestId("launch-always")).toBeNull();
    cleanup();
    // Opened again on the projects as reloaded.
    mountSheet([project(["Side", "Work Max"], { default_account_id: "claude-side" })]);
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here");
    expect(screen.queryByTestId("launch-always")).toBeNull();
  });

  it("already bound: its default, and no binding offered", () => {
    mountSheet([project(["Work Max"], { default_account_id: "claude-side" })]);
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side — your default here");
    expect(screen.queryByTestId("launch-always")).toBeNull();
  });

  it("⌘↩ starts on the repo's own Account", async () => {
    const { launchSpy } = mountSheet([project(["Work Max"])]);
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api", undefined));
  });
});

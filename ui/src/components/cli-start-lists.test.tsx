/**
 * The 0.4.6 RC walks A8 and K4 (F-A8-1) on every list that starts a CLI
 * session from launch settings: a Claude app profile's folder has launch
 * settings too, but it isn't a CLI App unless the Apps list says so
 * (`lib/apps`' `cliRecipes`, D-84), so none offers "Claude Code CLI · Work
 * Max": the project page's "Or start one in another app" and its "New
 * sessions here start with", the inspector's "Launch with another app",
 * and a task's Start a session. New session's
 * own case is `new-session-real-apps.test.tsx`.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RightInspector } from "./RightInspector";
import { StartSession } from "../pages/tasks/StartSession";
import type { StoredAppInfo } from "../lib/ipc/apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { AppModel } from "../lib/apps";
import { account, clearSeams, project, recipe, setSeam } from "../test/fixtures";

vi.mock("./TerminalView", () => ({ TerminalView: () => null }));

const FOLDERS = [account({ id: "cc-work", label: "work" }), account({ id: "app-work", label: "Work Max" })];
const RECIPES = [recipe({ id: "r-cc-work", name: "Main", account_id: "cc-work", account_label: "work" }), recipe({ id: "r-app-work", name: "Main", account_id: "app-work", account_label: "Work Max" })];
const PLACE: KnownPlaceInfo = {
  id: "app:claude-app:/p/Work Max", app_id: "claude-app", app: "Claude", instance: "Work Max", profile: "Work Max", label: "Claude · Work Max", app_pid: null, lost: false,
  keep_alive: false, paused: null, account_id: "app-work", seen_ms: 0, cannot_start: null, recent: [], found: false,
};
const STORED: StoredAppInfo[] = [{ id: "cli:cc-work", kind: "cli", tool: "claude-code", host_app_id: null, label: null, folder_id: "cc-work", account_id: null, last_seen_ms: 0 }];

// The live app's rule, with the backend's list (kept per input, as the hooks keep it).
const offered = new WeakMap<readonly LaunchRecipeInfo[], readonly LaunchRecipeInfo[]>();
const cliApps = new WeakMap<readonly AccountInfo[], AppModel[]>();
vi.mock("../lib/apps", async (original) => {
  const real = await original<typeof import("../lib/apps")>();
  // Read when called: the factory runs before this file's constants.
  const read = () => ({ stored: STORED, places: [PLACE], listed: true });
  return {
    ...real,
    useCliRecipes: (recipes: readonly LaunchRecipeInfo[] | null) => {
      if (recipes === null) return null;
      const kept = offered.get(recipes) ?? real.cliRecipes(recipes, read());
      offered.set(recipes, kept);
      return kept;
    },
    useCliApps: (folders: readonly AccountInfo[] = []) => {
      const kept = cliApps.get(folders) ?? real.appsOf({ ...read(), folders }).filter((app) => app.kind === "cli");
      cliApps.set(folders, kept);
      return kept;
    },
  };
});

const WEB = project({ id: "proj-web", root: "/tmp/lab/repos/web", name: "web", accounts_used: ["work", "Work Max"], history_sessions: 2 });

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_RECIPES__");
});

describe("F-A8-1: a CLI start offers only CLI Apps' launch settings", () => {
  it("the project page and the inspector", () => {
    const { container } = render(
      <RightInspector
        selection={{ kind: "project", id: WEB.id }}
        data={{ instance: null, project: WEB, account: null, provider: null, machineId: "this-mac" }}
        sessions={[]}
        projects={[WEB]}
        accounts={FOLDERS}
        profiles={RECIPES}
        terminalTitle={null}
        freshness={{}}
        now={Date.now()}
        onOpenProject={() => {}}
        onOpenSession={() => {}}
        onLaunch={() => {}}
        onStopped={() => {}}
        onHandover={() => {}}
        onNew={() => {}}
      />,
    );
    fireEvent.click(container.querySelector("#page-tab-overview")!);
    const starts = [...container.querySelectorAll('[data-testid="project-accounts"] button')].filter((button) => !button.closest('[data-testid="project-default-account"]')).map((button) => button.textContent ?? "");
    expect(starts.some((words) => words.includes("Claude Code CLI · work"))).toBe(true);
    expect(container.textContent).not.toContain("Claude Code CLI · Work Max");
    // Its "New sessions here start with": the CLI Apps, by their names.
    fireEvent.click(within(screen.getByTestId("project-default-account")).getByRole("button"));
    expect(screen.getAllByRole("menuitemradio").map((option) => option.textContent)).toEqual(["The app used last", "Claude Code CLI · work"]);
  });

  it("a task's Start a session", async () => {
    setSeam("__AIJ_RECIPES__", RECIPES);
    render(<StartSession taskId="aj-1" />);
    const picker = await screen.findByTestId("task-start-profile");
    // The one CLI App is picked for you, as the only one there is.
    await waitFor(() => expect(picker.getAttribute("data-value")).toBe("r-cc-work"));
    fireEvent.click(picker);
    expect(screen.getAllByRole("menuitemradio").map((option) => option.textContent)).toEqual(["Main · Claude Code CLI · work"]);
  });
});

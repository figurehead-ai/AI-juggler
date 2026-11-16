/**
 * The 0.4.6 RC walk A8 (F-A8-1): New session offers and names only the
 * Apps the user has (D-62, D-84). On one Account, Claude Code on its
 * folder "work" and a Claude app profile "Work Max" on its own folder: the
 * profile's folder has launch settings too, but it is the profile's, not
 * a CLI App (the backend's Apps list, `lib/apps`). The sheet opened on it
 * from the profile's session in the project ("Start in web on Claude Code
 * CLI · Work Max"), an App that doesn't exist, and would have run the
 * CLI on the profile's folder; its Change app listed "Claude Code CLI ·
 * Work Max" and "· Side" (K4). The pure rule is `offeredRecipes`
 * (`lib/new-session-offered.test.ts`); a switch's targets, which are real,
 * are named by their App (`switch-target-apps.test.tsx`).
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import type { AppModel } from "../lib/apps";
import { loadSubscriptions, staleSubscriptions } from "../lib/subscriptions";
import type { StoredAppInfo } from "../lib/ipc/apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { account, clearSeams, instance, project, recipe, setSeam, subscription } from "../test/fixtures";

const NOW = Date.now();

// A8 and K4: Claude Code on "work" (Work) and "personal" (Personal), and the
// Claude app's profiles Work Max (Work) and Side (Personal) on their own folders.
const FOLDERS = [account({ id: "cc-work", label: "work" }), account({ id: "app-work", label: "Work Max" }), account({ id: "cc-personal", label: "personal" }), account({ id: "app-side", label: "Side" })];
const RECIPES = FOLDERS.map((row) => recipe({ id: `r-${row.id}`, account_id: row.id, account_label: row.label }));

const profile = (name: string, folder: string): KnownPlaceInfo => ({
  id: `app:claude-app:/p/${name}`, app_id: "claude-app", app: "Claude", instance: name, profile: name, label: `Claude · ${name}`, app_pid: null, lost: false,
  keep_alive: false, paused: null, account_id: folder, seen_ms: NOW, cannot_start: null, recent: [], found: false,
});
const PLACES = [profile("Work Max", "app-work"), profile("Side", "app-side")];
const stored = (id: string, folder_id: string): StoredAppInfo => ({ id, kind: id.startsWith("cli:") ? "cli" : "desktop", tool: "claude-code", host_app_id: null, label: null, folder_id, account_id: null, last_seen_ms: 0 });
// The backend's list (`apps::users_apps`): no CLI App on either profile's folder.
const STORED = [stored("cli:cc-work", "cc-work"), stored("cli:cc-personal", "cc-personal"), ...PLACES.map((place) => stored(place.id, place.account_id ?? ""))];

// The live app's Apps, as `useApps` reads them with the backend's list.
const joined = new WeakMap<readonly AccountInfo[], AppModel[]>();
vi.mock("../lib/apps", async (original) => {
  const real = await original<typeof import("../lib/apps")>();
  return {
    ...real,
    useApps: (folders: readonly AccountInfo[] = []) => {
      const apps = joined.get(folders) ?? real.appsOf({ stored: STORED, places: PLACES, folders, listed: true });
      joined.set(folders, apps);
      return apps;
    },
  };
});

const WEB = project({ id: "proj-web", root: "/tmp/lab/repos/web", name: "web", last_activity_ms: NOW - 2 * 60_000, accounts_used: ["work", "Work Max"], history_sessions: 2, instance_count: 2 });
const session = (id: string, folder: string, label: string, kind: string, ago: number) =>
  instance({ id, account_id: folder, account_label: label, kind, status: "idle", project_id: WEB.id, cwd: WEB.root, started_at_ms: NOW - ago });
// The Claude app profile's session in web is the newest.
const SESSIONS = [session("w1", "cc-work", "work", "terminal", 10 * 60_000), session("a1", "app-work", "Work Max", "desktop", 2 * 60_000)];

// One Account (the lab's Work) over the CLI App's folder and the profile's.
const member = (id: string, label: string) => ({ id, label, folder: `/tmp/lab/${id}`, plan: null, renewal_date: null, fee_usd: null });
const SUBSCRIPTIONS = [
  subscription({ folder: "cc-work", account_id: "acct:work", email: "w…@example.com", profiles: [member("cc-work", "work"), member("app-work", "Work Max")] }),
  subscription({ folder: "cc-personal", account_id: "acct:personal", email: "p…@example.com", profiles: [member("cc-personal", "personal"), member("app-side", "Side")] }),
];

beforeEach(async () => {
  setSeam("__AIJ_RECIPES__", RECIPES);
  setSeam("__AIJ_QUOTA_DISPLAY__", []);
  setSeam("__AIJ_RANKING__", []);
  setSeam("__AIJ_FIXTURE__", { subscriptions: SUBSCRIPTIONS });
  // Held as the app holds them when the sheet opens.
  await loadSubscriptions(true);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  staleSubscriptions();
  clearSeams("__AIJ_FIXTURE__");
});

function mountSheet({ door = null as string | null, web = WEB } = {}) {
  const launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
  render(<NewSessionModal projects={[web]} accounts={FOLDERS} sessions={SESSIONS} recipes={RECIPES} initialDirectory={web.root} initialAccount={door} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);
  return { launchSpy };
}

describe("F-A8-1 New session offers and names only real Apps", () => {
  it("the repo's own Account opens on its CLI App, by its real name, never on the profile its newest session ran in", () => {
    mountSheet();
    const sentence = screen.getByTestId("new-session-sentence").textContent ?? "";
    expect(sentence).toBe("Start in web on Claude Code CLI · work — the only account used here");
    expect(sentence).not.toContain("Work Max");
    // Its binding is offered on the Account (its one App here), not on a profile's CLI.
    expect(screen.getByTestId("launch-always").textContent).not.toContain("Work Max");
  });

  it("Change app lists the CLI Apps only, never one on a profile's folder (K4: \"Claude Code CLI · Work Max\" and \"· Side\")", async () => {
    mountSheet();
    fireEvent.click(screen.getByTestId("change-app"));
    expect(await screen.findByTestId("change-app-cc-work")).toBeTruthy();
    expect(screen.getByTestId("change-app-cc-personal")).toBeTruthy();
    expect(screen.queryByTestId("change-app-app-work")).toBeNull();
    expect(screen.queryByTestId("change-app-app-side")).toBeNull();
    expect(screen.queryByText(/Work Max|Side/)).toBeNull();
  });

  it("a door that names the profile's folder (the Account page's New session: its first folder) opens on its Account's CLI App", async () => {
    const { launchSpy } = mountSheet({ door: "app-work" });
    expect(screen.getByTestId("new-session-sentence").textContent).toBe("Start in web on Claude Code CLI · work");
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await vi.waitFor(() => expect(launchSpy).toHaveBeenCalled());
    expect(launchSpy.mock.calls[0]?.[0]).toBe("r-cc-work");
  });

  it("a project bound to the profile's folder in rc1 (\"Always start web on Claude Code CLI · Work Max\") stays on its Account", () => {
    mountSheet({ web: { ...WEB, default_account_id: "app-work" } });
    expect(screen.getByTestId("new-session-sentence").textContent).toBe("Start in web on Claude Code CLI · work — your default here");
  });

  it("⌘↩ starts the CLI App's own launch settings, never the profile folder's", async () => {
    const { launchSpy } = mountSheet();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await vi.waitFor(() => expect(launchSpy).toHaveBeenCalled());
    expect(launchSpy.mock.calls[0]?.[0]).toBe("r-cc-work");
  });
});

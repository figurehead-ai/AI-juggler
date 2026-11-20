/**
 * The 0.4.6 RC walks A8 and K4 (D-62, D-84): a CLI start is offered with a
 * CLI App's launch settings only (`lib/apps`' `cliRecipes`). A Claude app
 * profile's folder has launch settings too, but it is a CLI App only once
 * a terminal session ran there or the user named it. A door or binding
 * that names such a folder opens New session on its Account's CLI App.
 * The sheet's case is `components/new-session-real-apps.test.tsx`.
 */
import { describe, expect, it } from "vitest";
import { cliRecipes } from "./apps";
import { chooseApp } from "./new-session-choice";
import type { KnownPlaceInfo } from "./ipc/places";
import type { StoredAppInfo } from "./ipc/apps";
import { account, recipe } from "../test/fixtures";

const cc = recipe({ id: "r-cc", account_id: "cc-work", account_label: "work" });
const profile = recipe({ id: "r-app", account_id: "app-work", account_label: "Work Max" });
const gone = recipe({ id: "r-old", account_id: "removed", account_label: "Old" });
const place: KnownPlaceInfo = {
  id: "app:claude-app:/p/Work Max", app_id: "claude-app", app: "Claude", instance: "Work Max", profile: "Work Max", label: "Claude · Work Max", app_pid: null, lost: false,
  keep_alive: false, paused: null, account_id: "app-work", seen_ms: 0, cannot_start: null, recent: [], found: false,
};
const cli = (folder: string, label: string | null = null): StoredAppInfo => ({ id: `cli:${folder}`, kind: "cli", tool: "claude-code", host_app_id: null, label, folder_id: folder, account_id: null, last_seen_ms: 0 });

describe("the launch settings a CLI start is offered with", () => {
  it("a folder a Claude app profile holds, with no CLI App on it, isn't offered", () => {
    expect(cliRecipes([cc, profile], { stored: [cli("cc-work")], places: [place], listed: true })).toEqual([cc]);
  });

  it("once it is a CLI App (a terminal session ran there, or you named it), it is", () => {
    const both = [cc, profile];
    expect(cliRecipes(both, { stored: [cli("cc-work"), cli("app-work")], places: [place], listed: true })).toBe(both);
  });

  it("before the Apps are read (or in the preview) all are, and a folder no profile holds always is", () => {
    const all = [cc, profile, gone];
    expect(cliRecipes(all, { stored: [], places: [place], listed: false })).toBe(all);
    expect(cliRecipes(all, { stored: [cli("cc-work")], places: [place], listed: true })).toEqual([cc, gone]);
  });
});

describe("a folder that isn't offered, named by a door or a binding", () => {
  const personal = recipe({ id: "r-me", account_id: "cc-me", account_label: "personal" });
  const offered = [personal, cc];
  const three = [account({ id: "cc-me" }), account({ id: "cc-work" }), account({ id: "app-work" })];
  const accountOf = (id: string) => (id === "cc-me" ? "k-me" : id === "removed" ? "removed" : "k-work");
  const room = () => ({ left: null, resetsAtMs: null, credits: false });

  it("opens on its Account's CLI App, never another Account's", () => {
    expect(chooseApp({ recipes: offered, project: null, folders: three, roomOf: room, accountOf, chosen: "app-work" })).toMatchObject({ recipe: cc, why: "chosen" });
  });

  it("a gone one still gives way to the next reason", () => {
    expect(chooseApp({ recipes: offered, project: null, folders: three, roomOf: room, accountOf, chosen: "removed" })?.why).not.toBe("chosen");
  });
});

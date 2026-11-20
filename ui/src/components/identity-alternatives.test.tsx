/**
 * The 0.4.6 RC walks A8 and K4 (F-A8-1) on the wrong-account pause: each
 * "Use …" is a CLI start on other launch settings, so only a CLI App's are
 * offered (`lib/apps`' `cliRecipes`; a Claude app profile's folder isn't
 * one until the Apps list says so, D-84), each named by its App
 * (`targetApp`: your name for it), never "Claude Code CLI · Side".
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { IdentityMismatchDialog } from "./IdentityMismatchDialog";
import { launchSession } from "../lib/ipc/launch";
import type { FolderTarget } from "../lib/apps";
import type { StoredAppInfo } from "../lib/ipc/apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { IdentityCheckInfo } from "../types/generated/IdentityCheckInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import { clearSeams, recipe, setSeam } from "../test/fixtures";

// Claude Code on "work" (launched) and "personal" (named "Home terminal"
// by you), and the Claude app's profile Side on its own folder.
const RECIPES = [
  recipe({ id: "rec-work", account_id: "cc-work", account_label: "work" }),
  recipe({ id: "rec-personal", account_id: "cc-personal", account_label: "personal" }),
  recipe({ id: "rec-side", account_id: "app-side", account_label: "Side" }),
];
const SIDE: KnownPlaceInfo = {
  id: "app:claude-app:/p/Side", app_id: "claude-app", app: "Claude", instance: "Side", profile: "Side", label: "Claude · Side", app_pid: null, lost: false,
  keep_alive: false, paused: null, account_id: "app-side", seen_ms: 0, cannot_start: null, recent: [], found: false,
};
const cli = (folder: string, label: string | null = null): StoredAppInfo => ({ id: `cli:${folder}`, kind: "cli", tool: "claude-code", host_app_id: null, label, folder_id: folder, account_id: null, last_seen_ms: 0 });
const STORED = [cli("cc-work"), cli("cc-personal", "Home terminal")];

// The live app's rules, with the backend's list (kept per input, as the hooks keep them).
const offered = new WeakMap<readonly { account_id: string }[], readonly { account_id: string }[]>();
vi.mock("../lib/apps", async (original) => {
  const real = await original<typeof import("../lib/apps")>();
  // Read when called: the factory runs before this file's constants.
  const read = () => ({ stored: STORED, places: [SIDE], listed: true });
  const appOf = (target: FolderTarget) => real.targetApp(target, read());
  return {
    ...real,
    useTargetApp: () => appOf,
    useCliRecipes: (rows: readonly { account_id: string }[] | null) => {
      if (rows === null) return null;
      const kept = offered.get(rows) ?? real.cliRecipes(rows, read());
      offered.set(rows, kept);
      return kept;
    },
  };
});

const MISMATCH: IdentityCheckInfo = {
  state: "mismatch",
  provider: "claude-code",
  account_id: "cc-work",
  account_label: "work",
  model: null,
  expected: "o…@work.example",
  detected: "o…@home.example",
  reason: null,
  alternatives: [
    { recipe_id: "rec-personal", recipe_name: "Main", account_label: "personal" },
    { recipe_id: "rec-side", recipe_name: "Main", account_label: "Side" },
  ],
};

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_IDENTITY_CHECK__", "__AIJ_LAUNCH_CALLS__");
});

describe("F-A8-1: the wrong-account pause offers and names only real CLI Apps", () => {
  it("each way out is a CLI App's, by its name; a Claude app profile's folder isn't offered", async () => {
    setSeam("__AIJ_IDENTITY_CHECK__", { "rec-work": MISMATCH });
    render(<IdentityMismatchDialog recipes={RECIPES satisfies LaunchRecipeInfo[]} />);
    await act(async () => {
      void launchSession("rec-work", null).catch(() => null);
    });
    await waitFor(() => expect(screen.getByTestId("identity-mismatch")).toBeTruthy());
    expect(screen.getByRole("heading").textContent).toBe("⚠ Wrong account for Claude Code CLI · work");
    expect(screen.getAllByTestId("identity-switch").map((button) => button.querySelector("span")?.textContent)).toEqual(["Use Home terminal (Default)"]);
    expect(screen.getByTestId("identity-mismatch").textContent).not.toContain("Side");
  });
});

/**
 * AJ-E305 vitest — flow (b): the New project sheet. The preselect
 * rule (last-used → most headroom → first recipe) is pinned as a
 * PURE function; the component test walks the 2-click path (tile →
 * Start session) and the no-account inline detour.
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NewProjectSheet, pickRecipe } from "../screens/NewProjectSheet";
import * as bridgeModule from "../lib/bridge";
import type { LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

const recipe = (
  id: string,
  label: string,
  overrides: Partial<LaunchRecipeInfo> = {},
): LaunchRecipeInfo => ({
  id,
  name: "Default",
  account_id: `acc-${id}`,
  account_label: label,
  provider: "claude-code",
  model: null,
  default_cwd: null,
  ...overrides,
});

const project = (root: string, used: string[]): ProjectInfo =>
  ({
    id: root,
    root,
    name: root.split("/").pop() ?? root,
    last_activity_ms: 1000,
    accounts_used: used,
    history_sessions: 2,
    active_instance: null,
    gone: false,
  }) as unknown as ProjectInfo;

const quota = (account: string, state: string) =>
  ({
    account,
    forecast: { state, badge: state },
    renewal: { state: "none", label: "", entered_by_user: false },
  }) as unknown as bridgeModule.QuotaDisplayData;

describe("AJ-E305 pickRecipe — the preselect rule (Req.6)", () => {
  it("last-used on the dir wins, most recent use first", () => {
    const rows = [recipe("a", "Work"), recipe("b", "Personal"), recipe("c", "Side")];
    expect(pickRecipe(rows, project("/x", ["Work", "Personal"]), [])).toBe(rows[1]);
    expect(pickRecipe(rows, project("/x", ["Side", "Work"]), [])).toBe(rows[0]);
  });

  it("no history on the dir → most headroom (engine forecast order)", () => {
    const rows = [recipe("a", "Work"), recipe("b", "Personal"), recipe("c", "Side")];
    const quotas = [quota("acc-a", "exhausts"), quota("acc-b", "wont-exhaust"), quota("acc-c", "learning")];
    expect(pickRecipe(rows, null, quotas)).toBe(rows[1]);
    expect(pickRecipe(rows, project("/x", []), quotas)).toBe(rows[1]);
  });

  it("no signal at all → the first recipe (most recent)", () => {
    const rows = [recipe("a", "Work"), recipe("b", "Personal")];
    expect(pickRecipe(rows, null, [])).toBe(rows[0]);
    expect(pickRecipe([], null, [])).toBe(null);
  });
});

describe("AJ-E305 NewProjectSheet — flow (b) in ≤3 clicks", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  const accounts = [
    { id: "acc-a", provider: "claude-code", label: "Work", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
  ];

  const mountSheet = (extra: { recipes?: LaunchRecipeInfo[] } = {}) => {
    const launchSpy = vi
      .spyOn(bridgeModule, "launchSession")
      .mockResolvedValue({ session_id: "s1" } as never);
    vi.spyOn(bridgeModule, "launchRecipesList").mockResolvedValue(
      extra.recipes ?? [recipe("a", "Work")],
    );
    vi.spyOn(bridgeModule, "quotaDisplayAll").mockResolvedValue([]);
    const launched = vi.fn();
    const mounted = render(
      <NewProjectSheet
        projects={[project("/Users/you/dev-api", ["Work"])]}
        accounts={accounts}
        initialDirectory={null}
        onClose={() => {}}
        onLaunched={launched}
        openLoginTerminal={() => {}}
        onAccountsChanged={() => {}}
      />,
    );
    return { ...mounted, launchSpy, launched };
  };

  it("tile click → recommended account preselected → Start session launches it (2 clicks, 0 typed)", async () => {
    const { container, launchSpy, launched } = mountSheet();
    const tiles = await waitFor(() => {
      const found = container.querySelectorAll("[data-testid='project-tiles'] button[data-tile]");
      expect(found.length).toBeGreaterThan(0);
      return found;
    });
    // Click 1: the recent-dir tile.
    fireEvent.click(tiles[0]!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='project-account']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='project-account']")!.textContent).toContain(
      "Work",
    );
    expect(container.querySelector("[data-testid='project-account']")!.textContent).toContain(
      "last used here",
    );
    // Click 2: Start session → with the recommended recipe + the dir.
    fireEvent.click(container.querySelector("[data-testid='start-session']")!);
    await waitFor(() => expect(launchSpy).toHaveBeenCalled());
    expect(launchSpy).toHaveBeenCalledWith("a", "/Users/you/dev-api");
    expect(launched).toHaveBeenCalledWith("s1", "Work · Default");
  });

  it("no Account → inline flow (d) detour, and the pending directory survives it (Req.7)", async () => {
    const { container } = mountSheet({ recipes: [] });
    const tiles = await waitFor(() => {
      const found = container.querySelectorAll("[data-testid='project-tiles'] button[data-tile]");
      expect(found.length).toBeGreaterThan(0);
      return found;
    });
    fireEvent.click(tiles[0]!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='no-account-detour']")).not.toBeNull(),
    );
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((button) =>
        button.textContent?.includes("Add an account"),
      )!,
    );
    // The wizard opens INLINE over the sheet — same overlay, pending dir kept.
    await waitFor(() =>
      expect(container.querySelector("[data-testid='onboarding-wizard']")).not.toBeNull(),
    );
    expect(container.textContent).toContain("Starting in dev-api");
  });

  it("an unsigned-in recommendation disables Start and says why (honest before commit)", async () => {
    const signedOut = [
      { id: "acc-a", provider: "claude-code", label: "Work", plan: null, login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
    ];
    const launchSpy = vi
      .spyOn(bridgeModule, "launchSession")
      .mockResolvedValue({ session_id: "s1" } as never);
    vi.spyOn(bridgeModule, "launchRecipesList").mockResolvedValue([recipe("a", "Work")]);
    vi.spyOn(bridgeModule, "quotaDisplayAll").mockResolvedValue([]);
    const mounted = render(
      <NewProjectSheet
        projects={[project("/Users/you/dev-api", ["Work"])]}
        accounts={signedOut}
        initialDirectory="/Users/you/dev-api"
        onClose={() => {}}
        onLaunched={() => {}}
        openLoginTerminal={() => {}}
        onAccountsChanged={() => {}}
      />,
    );
    const start = await waitFor(() => {
      const button = mounted.container.querySelector(
        "[data-testid='start-session']",
      ) as HTMLButtonElement;
      expect(button).not.toBeNull();
      return button!;
    });
    expect(start.disabled).toBe(true);
    expect(mounted.container.textContent).toContain("needs sign-in");
    launchSpy.mockRestore();
  });
});

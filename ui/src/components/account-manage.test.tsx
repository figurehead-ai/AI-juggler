/**
 * AJ-E1501 vitest — managing accounts from the account card's menu (the
 * rail's row-menu pattern): Rename (a duplicate refused in words), Make
 * the default, and the remove sheet for each kind of account — an
 * account used where it is keeps its folder, a copy keeps its folder or
 * sends it to the Trash, open sessions keep the account, named. New
 * session preselects the project's default, then the global default.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../screens/Profiles";
import { pickRecipe } from "../screens/NewProjectSheet";
import * as bridge from "../lib/bridge";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, ProjectInfo } from "../lib/bridge";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const account = (id: string, label: string, extra: Partial<AccountInfo> = {}): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
  ...extra,
});

const recipe = (id: string, accountId: string, label: string): LaunchRecipeInfo => ({
  id,
  name: "Main",
  account_id: accountId,
  account_label: label,
  provider: "claude-code",
  model: null,
  default_cwd: null,
  restart_on_crash: null,
});

const session = (id: string, accountId: string, status: string, branch: string): InstanceInfo =>
  ({ id, account_id: accountId, account_label: "x", status, git_branch: branch, cwd: `/repo/${branch}`, started_at_ms: 0 }) as InstanceInfo;

let rows: AccountInfo[];
beforeEach(() => {
  rows = [
    account("acc-own", "Mine", { in_place: true, import_source_path: "/Users/you/.claude" }),
    account("acc-copy", "personal-max", { import_source_path: "/Users/you/ClaudeCodeProfiles/max" }),
  ];
  vi.spyOn(bridge, "accountsList").mockImplementation(async () => rows.map((row) => ({ ...row })));
  vi.spyOn(bridge, "launchRecipesList").mockResolvedValue([]);
  vi.spyOn(bridge, "quotaDisplayAll").mockResolvedValue([]);
  vi.spyOn(bridge, "providersCapabilities").mockResolvedValue([]);
});

const mount = (sessions: InstanceInfo[] = [], onAccountsChanged = vi.fn()) => {
  const view = render(
    <ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} sessions={sessions} onAccountsChanged={onAccountsChanged} />,
  );
  return { ...view, onAccountsChanged };
};

const openMenu = async (view: ReturnType<typeof render>, index: number) => {
  const buttons = await waitFor(() => {
    const found = view.container.querySelectorAll<HTMLButtonElement>("[data-testid='account-menu']");
    expect(found.length).toBe(2);
    return found;
  });
  fireEvent.click(buttons[index]!);
  return view.getByRole("menu");
};

describe("AJ-E1501 the account card's menu", () => {
  it("opens from the ⋯ button with Rename, Make the default and Remove, and closes on Escape", async () => {
    const view = mount();
    const menu = await openMenu(view, 1);
    // AJ-E1537: the registry's account menu; a disabled item says why.
    const items = [...menu.querySelectorAll("[role='menuitem']")].map((item) => item.querySelector("[data-label]")?.textContent);
    expect(items).toEqual(["Open its page", "Start a session", "Edit launch settings", "Measure its limits…", "Rename…", "Copy settings from…", "Make the default", "Remove…"]);
    const start = menu.querySelector("[data-testid='action-start']") as HTMLButtonElement;
    expect(start.disabled).toBe(true);
    expect(start.title).toBe("Add launch settings first");
    expect(document.activeElement?.querySelector("[data-label]")?.textContent).toBe("Open its page");
    fireEvent.keyDown(window, { key: "ArrowDown" });
    expect(document.activeElement?.querySelector("[data-label]")?.textContent).toBe("Edit launch settings");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(view.queryByRole("menu")).toBeNull();
  });

  it("rename saves the new name and plan; another account's name is refused in words", async () => {
    const rename = vi.spyOn(bridge, "accountRename").mockImplementation(async (id, label, plan) => {
      if (label.toLowerCase() === "mine") {
        throw { code: "refused", retryable: false, user_message: "Another account is already called «Mine» — pick another name" };
      }
      rows = rows.map((row) => (row.id === id ? { ...row, label, plan } : row));
      return rows.find((row) => row.id === id)!;
    });
    const view = mount();
    fireEvent.click((await openMenu(view, 1)).querySelector("[data-testid='action-rename']")!);
    const input = view.getByTestId("rename-account-label") as HTMLInputElement;
    expect(input.value).toBe("personal-max");

    fireEvent.change(input, { target: { value: "mine" } });
    fireEvent.click(view.getByTestId("rename-account-save"));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("already called «Mine»"));

    fireEvent.change(input, { target: { value: "Max (personal)" } });
    fireEvent.change(view.getByTestId("rename-account-plan"), { target: { value: "Max" } });
    fireEvent.click(view.getByTestId("rename-account-save"));
    await waitFor(() => expect(view.queryByTestId("rename-account")).toBeNull());
    expect(rename).toHaveBeenLastCalledWith("acc-copy", "Max (personal)", "Max");
    await waitFor(() =>
      expect([...view.container.querySelectorAll("[data-testid='account-label']")].map((n) => n.textContent)).toEqual(["Mine", "Max (personal)"]),
    );
    expect(view.getByTestId("accounts-notice").textContent).toBe("Renamed to Max (personal).");
    expect(view.onAccountsChanged).toHaveBeenCalled();
  });

  it("Make the default marks the account, and the menu then offers to stop", async () => {
    const setDefault = vi.spyOn(bridge, "accountSetDefault").mockImplementation(async (id) => {
      rows = rows.map((row) => ({ ...row, is_default: row.id === id }));
      return rows;
    });
    const view = mount();
    fireEvent.click((await openMenu(view, 0)).querySelector("[data-testid='action-default']")!);
    await waitFor(() => expect(view.getByTestId("account-default")).toBeTruthy());
    expect(setDefault).toHaveBeenCalledWith("acc-own");
    expect(view.getByTestId("accounts-notice").textContent).toContain("New sessions start with Mine");
    const menu = await openMenu(view, 0);
    expect(menu.querySelector("[data-testid='action-default']")!.textContent).toBe("Stop being the default");
  });
});

describe("AJ-E1501 the remove sheet", () => {
  it("an account used where it is: its folder stays exactly as it is — no Trash choice", async () => {
    const remove = vi.spyOn(bridge, "accountRemove").mockResolvedValue({ id: "acc-own", label: "Mine", folder: "untouched" });
    const view = mount();
    fireEvent.click((await openMenu(view, 0)).querySelector("[data-testid='action-remove']")!);
    const sheet = view.getByTestId("remove-account");
    expect(sheet.dataset.kind).toBe("in-place");
    expect(sheet.textContent).toContain("Your folder stays exactly as it is");
    expect(sheet.textContent).toContain("~/.claude");
    expect(sheet.textContent).toContain("marked “removed account”");
    expect(view.queryByTestId("remove-folder-trash")).toBeNull();
    fireEvent.click(view.getByTestId("remove-account-confirm"));
    await waitFor(() => expect(remove).toHaveBeenCalledWith("acc-own", "keep"));
    await waitFor(() => expect(view.getByTestId("accounts-notice").textContent).toBe("Removed Mine. Your folder wasn't touched."));
  });

  it("a copy: keep its folder by default, or move it to the Trash when chosen", async () => {
    const remove = vi.spyOn(bridge, "accountRemove").mockResolvedValue({ id: "acc-copy", label: "personal-max", folder: "trashed" });
    const view = mount();
    fireEvent.click((await openMenu(view, 1)).querySelector("[data-testid='action-remove']")!);
    expect(view.getByTestId("remove-account").dataset.kind).toBe("copy");
    expect((view.getByTestId("remove-folder-keep") as HTMLInputElement).checked).toBe(true);
    fireEvent.click(view.getByTestId("remove-folder-trash"));
    fireEvent.click(view.getByTestId("remove-account-confirm"));
    await waitFor(() => expect(remove).toHaveBeenCalledWith("acc-copy", "trash"));
    await waitFor(() => expect(view.getByTestId("accounts-notice").textContent).toBe("Removed personal-max. Its folder is in the Trash."));
  });

  it("open sessions keep the account: they are named and there is no Remove button", async () => {
    const remove = vi.spyOn(bridge, "accountRemove");
    const view = mount([
      session("s1", "acc-copy", "running", "api"),
      session("s2", "acc-copy", "waiting_user", "web"),
      session("s3", "acc-copy", "finished", "old"),
    ]);
    fireEvent.click((await openMenu(view, 1)).querySelector("[data-testid='action-remove']")!);
    expect(view.getByTestId("remove-account-open").textContent).toContain("has 2 sessions open: api, web");
    expect(view.queryByTestId("remove-account-confirm")).toBeNull();
    expect(remove).not.toHaveBeenCalled();
  });

  it("a refusal from the app is shown in its words, and nothing changes", async () => {
    vi.spyOn(bridge, "accountRemove").mockRejectedValue({ code: "io", retryable: true, user_message: "move the copy to the Trash: the Trash is full" });
    const view = mount();
    fireEvent.click((await openMenu(view, 1)).querySelector("[data-testid='action-remove']")!);
    fireEvent.click(view.getByTestId("remove-folder-trash"));
    await act(async () => {
      fireEvent.click(view.getByTestId("remove-account-confirm"));
    });
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("the Trash is full"));
    expect(view.getByTestId("remove-account")).toBeTruthy();
  });
});

describe("AJ-E1501 New session's preselect honours the defaults", () => {
  const recipes = [recipe("r-a", "acc-a", "A"), recipe("r-b", "acc-b", "B"), recipe("r-c", "acc-c", "C")];
  const project = (extra: Partial<ProjectInfo>): ProjectInfo => ({ id: "p", root: "/x", name: "x", accounts_used: ["A"], ...extra }) as ProjectInfo;

  it("the project's own default wins, then the global default, then the last used (as before)", () => {
    const accounts = [account("acc-a", "A"), account("acc-b", "B", { is_default: true }), account("acc-c", "C")];
    expect(pickRecipe(recipes, project({ default_account_id: "acc-c" }), [], accounts)?.id).toBe("r-c");
    expect(pickRecipe(recipes, project({}), [], accounts)?.id).toBe("r-b");
    expect(pickRecipe(recipes, project({}), [], [account("acc-a", "A"), account("acc-b", "B")])?.id).toBe("r-a");
    // A default with no launch settings falls through.
    expect(pickRecipe(recipes, project({ default_account_id: "acc-gone" }), [], [])?.id).toBe("r-a");
  });
});

import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import * as copy from "../lib/ipc/accountSettings";
import type { SetupCheckInfo } from "../lib/ipc/switchSetup";
import { FreshAccountStep } from "./FreshAccountStep";

/**
 * AJ-E1508: Add account's fresh account starts with the user's usual
 * settings from an account of the same tool: one line, ticked, opening to
 * what comes with it; copied once the account is made.
 */

type Seams = { __AIJ_ACCOUNT_SETTINGS__?: Record<string, SetupCheckInfo> };

const WORK = { id: "acc-work", label: "Work", provider: "claude-code" } as unknown as bridge.AccountInfo;
const CODEX = { id: "acc-codex", label: "Codex", provider: "codex" } as unknown as bridge.AccountInfo;
const NEW = { id: "acc-new", label: "Side project", login_state: "signed_out" } as unknown as bridge.AccountInfo;

const OFFERS: SetupCheckInfo = {
  from: "Work",
  to: "the new account",
  already_there: 0,
  offers: [
    { id: "o-docs", kind: "mcp-server", name: "docs", scope: "user", verdict: "copy", note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
    { id: "o-perm", kind: "permissions", name: "permissions", scope: "user", verdict: "copy", note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
    { id: "o-line", kind: "status-line", name: "status line", scope: "user", verdict: "copy", note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false },
    {
      id: "o-tidy",
      kind: "plugin",
      name: "tidy@market",
      scope: "user",
      verdict: "needs-you",
      note: "install it in this account with `claude plugin install tidy@market`",
      suggested: false,
      about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false,
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete (window as never as Seams).__AIJ_ACCOUNT_SETTINGS__;
});

function step(accounts: bridge.AccountInfo[], onCreated = vi.fn()) {
  (window as never as Seams).__AIJ_ACCOUNT_SETTINGS__ = { "acc-work": structuredClone(OFFERS) };
  const view = render(
    <FreshAccountStep
      toolName="Claude Code"
      provider="claude-code"
      taken={accounts.map((a) => a.label)}
      accounts={accounts}
      canGoBack={false}
      onBack={() => undefined}
      onImportFolder={() => undefined}
      onCreated={onCreated}
    />,
  );
  return { view, onCreated };
}

describe("a fresh account's usual settings", () => {
  it("offers nothing without another account of the same tool", async () => {
    const { view } = step([CODEX]);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("start-with-settings")).toBeNull();
  });

  it("starts ticked, says what comes with it, and opens to the list", async () => {
    const { view } = step([WORK, CODEX]);
    const line = await view.findByTestId("start-with-settings");
    expect((line.querySelector('input[type="checkbox"]') as HTMLInputElement).checked).toBe(true);
    expect(line.textContent).toContain("Start with the same settings as");
    expect(line.textContent).toContain("Work");
    const more = await view.findByRole("button", { name: /Its MCP servers, permissions and status line/ });
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(more.getAttribute("aria-expanded")).toBe("true");
    expect(view.getByText(/claude plugin install tidy@market/)).toBeTruthy();
    expect(line.textContent).toContain("Never its sign-in, history or caches");
  });

  it("copies what's ticked once the account is made", async () => {
    vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    const apply = vi.spyOn(copy, "accountSettingsApply");
    const { view, onCreated } = step([WORK]);
    fireEvent.click(await view.findByRole("button", { name: /Its MCP servers/ }));
    // The user unticks the status line.
    fireEvent.click(view.getByRole("checkbox", { name: /Status line/ }));
    fireEvent.click(view.getByTestId("first-run-fresh-create"));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(apply).toHaveBeenCalledWith("acc-new", "acc-work", ["o-docs", "o-perm"]);
    expect(onCreated.mock.calls[0]![0]).toMatchObject({ accountId: "acc-new" });
  });

  it("copies nothing when the line is unticked", async () => {
    vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    const apply = vi.spyOn(copy, "accountSettingsApply");
    const { view, onCreated } = step([WORK]);
    const line = await view.findByTestId("start-with-settings");
    await view.findByRole("button", { name: /Its MCP servers/ });
    fireEvent.click(line.querySelector('input[type="checkbox"]')!);
    fireEvent.click(view.getByTestId("first-run-fresh-create"));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(apply).not.toHaveBeenCalled();
  });

  it("says what couldn't be copied before moving on", async () => {
    vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    vi.spyOn(copy, "accountSettingsApply").mockResolvedValue([
      { offer: "o-docs", record: "r1", message: "Set up the MCP server docs", note: null },
      { offer: "o-perm", record: null, message: "it has its own permissions already", note: null },
    ]);
    const { view, onCreated } = step([WORK]);
    await view.findByRole("button", { name: /Its MCP servers/ });
    fireEvent.click(view.getByTestId("first-run-fresh-create"));
    expect(await view.findByTestId("first-run-fresh-settings")).toBeTruthy();
    expect(view.getByText(/it has its own permissions already/)).toBeTruthy();
    expect(onCreated).not.toHaveBeenCalled();
    fireEvent.click(view.getByRole("button", { name: "Continue" }));
    expect(onCreated).toHaveBeenCalled();
  });

  it("waits for what the account offers before Create", async () => {
    // Review of AJ-E1508 (M3): a quick Create copied nothing.
    let answer: (check: SetupCheckInfo) => void = () => undefined;
    vi.spyOn(copy, "accountSettingsCheck").mockReturnValue(new Promise((resolve) => (answer = resolve)));
    const { view } = step([WORK]);
    const create = view.getByTestId("first-run-fresh-create") as HTMLButtonElement;
    expect(create.disabled).toBe(true);
    answer(structuredClone(OFFERS));
    await waitFor(() => expect(create.disabled).toBe(false));
  });

  it("never carries one account's ticks over to another", async () => {
    const PERSONAL = { id: "acc-personal", label: "Personal", provider: "claude-code" } as unknown as bridge.AccountInfo;
    (window as never as Seams).__AIJ_ACCOUNT_SETTINGS__ = {
      "acc-work": structuredClone(OFFERS),
      "acc-personal": { ...structuredClone(OFFERS), from: "Personal", offers: [{ ...OFFERS.offers[1]!, suggested: false }] },
    };
    vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    const apply = vi.spyOn(copy, "accountSettingsApply");
    const view = render(
      <FreshAccountStep
        toolName="Claude Code"
        provider="claude-code"
        taken={["Work", "Personal"]}
        accounts={[WORK, PERSONAL]}
        canGoBack={false}
        onBack={() => undefined}
        onImportFolder={() => undefined}
        onCreated={() => undefined}
      />,
    );
    await view.findByRole("button", { name: /Its MCP servers/ });
    fireEvent.change(view.getByRole("combobox", { name: "Account to copy the settings from" }), { target: { value: "acc-personal" } });
    // Personal's permissions aren't ticked for the user: Work's ticks don't
    // come with the change.
    await view.findByRole("button", { name: /Nothing ticked/ });
    fireEvent.click(view.getByTestId("first-run-fresh-create"));
    await waitFor(() => expect(bridge.createAccount).toHaveBeenCalled());
    expect(apply).not.toHaveBeenCalled();
  });

  it("shows the line when the accounts arrive after it first draws", async () => {
    // Review of AJ-E1508 (M4).
    (window as never as Seams).__AIJ_ACCOUNT_SETTINGS__ = { "acc-work": structuredClone(OFFERS) };
    const props = {
      toolName: "Claude Code",
      provider: "claude-code",
      taken: null,
      canGoBack: false,
      onBack: () => undefined,
      onImportFolder: () => undefined,
      onCreated: () => undefined,
    };
    const view = render(<FreshAccountStep {...props} accounts={[]} />);
    expect(view.queryByTestId("start-with-settings")).toBeNull();
    view.rerender(<FreshAccountStep {...props} taken={["Work"]} accounts={[WORK]} />);
    expect(await view.findByTestId("start-with-settings")).toBeTruthy();
  });

  it("says what the user should know about what was copied", async () => {
    vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    vi.spyOn(copy, "accountSettingsApply").mockResolvedValue([
      {
        offer: "o-line",
        record: "r1",
        message: "Set up the status line (it runs a script from the other account's folder)",
        note: "it runs a script from the other account's folder",
      },
    ]);
    const { view, onCreated } = step([WORK]);
    await view.findByRole("button", { name: /Its MCP servers/ });
    fireEvent.click(view.getByTestId("first-run-fresh-create"));
    expect(await view.findByText(/runs a script from the other account's folder/)).toBeTruthy();
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("Enter waits for what the account offers too", async () => {
    // Re-review of AJ-E1508: Enter in the name field skipped the wait.
    let answer: (check: SetupCheckInfo) => void = () => undefined;
    vi.spyOn(copy, "accountSettingsCheck").mockReturnValue(new Promise((resolve) => (answer = resolve)));
    const made = vi.spyOn(bridge, "createAccount").mockResolvedValue(NEW);
    const { view } = step([WORK]);
    fireEvent.submit(view.getByTestId("first-run-fresh"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(made).not.toHaveBeenCalled();
    answer(structuredClone(OFFERS));
    await view.findByRole("button", { name: /Its MCP servers/ });
    fireEvent.submit(view.getByTestId("first-run-fresh"));
    await waitFor(() => expect(made).toHaveBeenCalled());
  });
});

import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import * as copy from "../lib/ipc/accountSettings";
import { AccountSettingsSheet } from "./AccountSettingsSheet";
import { EXAMPLE_ACCOUNT_SETTINGS } from "../preview/account-settings";

/**
 * AJ-E1508: "Copy settings from…" on an account, for one brought in as a
 * copy or any account later: the same tool's other accounts, what this one
 * lacks, ticked when it can be set up, and Undo for each.
 */

const PERSONAL = { id: "claude-personal", label: "Personal", provider: "claude-code" } as unknown as bridge.AccountInfo;
const WORK = { id: "claude-work", label: "Work Max", provider: "claude-code" } as unknown as bridge.AccountInfo;
const CODEX = { id: "codex-1", label: "Codex", provider: "codex" } as unknown as bridge.AccountInfo;

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("copy settings into an account", () => {
  it("offers the same tool's other accounts, copies what's ticked, and says what happened", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([WORK, PERSONAL, CODEX]);
    const apply = vi.spyOn(copy, "accountSettingsApply");
    const onClose = vi.fn();
    const view = render(<AccountSettingsSheet account={PERSONAL} onClose={onClose} />);
    expect(await view.findByRole("dialog", { name: "Copy settings into Personal" })).toBeTruthy();
    expect(await view.findByText("Work Max")).toBeTruthy();
    // Codex's account isn't offered: another tool.
    expect(view.queryByRole("combobox")).toBeNull();
    const button = await view.findByRole("button", { name: "Copy 3 settings" });
    fireEvent.click(view.getByRole("checkbox", { name: /Status line/ }));
    expect(view.getByRole("button", { name: "Copy 2 settings" })).toBe(button);
    fireEvent.click(button);
    await waitFor(() => expect(apply).toHaveBeenCalledWith("claude-personal", "claude-work", ["o-docs", "o-perm"]));
    expect(await view.findByText(/Set up the docs/)).toBeTruthy();
    fireEvent.click(view.getByRole("button", { name: "Done" }));
    expect(onClose).toHaveBeenCalledWith(2);
  });

  it("says so when there's no other account of the tool", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([PERSONAL, CODEX]);
    const view = render(<AccountSettingsSheet account={PERSONAL} onClose={() => undefined} />);
    expect(await view.findByText("There's no other account of this tool to copy from.")).toBeTruthy();
    expect(view.queryByTestId("account-settings-apply")).toBeNull();
  });

  it("says so when the account already has everything", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([WORK, PERSONAL]);
    vi.spyOn(copy, "accountSettingsCheck").mockResolvedValue({ ...EXAMPLE_ACCOUNT_SETTINGS["claude-work"]!, offers: [] });
    const view = render(<AccountSettingsSheet account={PERSONAL} onClose={() => undefined} />);
    expect(await view.findByText(/Personal already has everything Work Max has to copy/)).toBeTruthy();
    expect((view.getByTestId("account-settings-apply") as HTMLButtonElement).disabled).toBe(true);
  });
});

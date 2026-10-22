/**
 * Settings › Data, "Before you uninstall": one button puts the status
 * lines back and lists what's left, in words; nothing is deleted.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { UninstallCard } from "./UninstallCard";
import * as app from "../lib/ipc/app";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Before you uninstall", () => {
  it("puts status lines back and lists the data folder, Keychain items and live worktrees", async () => {
    vi.spyOn(app, "appPrepareUninstall").mockResolvedValue({
      status_lines: [
        { account: "Work Max", outcome: "restored" },
        { account: "Personal", outcome: "failed", problem: "settings.json can't be read" },
      ],
      data_folder: "~/Library/Application Support/AI Juggler",
      keychain_services: ["ai.figurehead.juggler", "ai.figurehead.juggler.vault"],
      worktrees: [
        { path: "~/Dev/a-fix", branch: "juggler/fix", exists: true },
        { path: "~/Dev/gone", branch: "juggler/old", exists: false },
      ],
    });
    render(<UninstallCard />);
    fireEvent.click(screen.getByTestId("uninstall-prepare"));
    const result = await screen.findByTestId("uninstall-result");
    const lines = screen.getAllByTestId("uninstall-status-line").map((line) => line.textContent);
    expect(lines[0]).toBe("Work Max: its settings are back as they were");
    expect(lines[1]).toContain("not put back (settings.json can't be read)");
    expect(result.textContent).toContain("~/Library/Application Support/AI Juggler");
    expect(result.textContent).toContain("ai.figurehead.juggler.vault");
    expect(result.textContent).toContain("~/Dev/a-fix");
    expect(result.textContent).not.toContain("~/Dev/gone");
  });

  it("says when there's nothing to put back, and a failure in words", async () => {
    vi.spyOn(app, "appPrepareUninstall")
      .mockRejectedValueOnce({ code: "io", retryable: true, user_message: "Storage is busy" })
      .mockResolvedValue({ status_lines: [], data_folder: "~/d", keychain_services: [], worktrees: [] });
    render(<UninstallCard />);
    fireEvent.click(screen.getByTestId("uninstall-prepare"));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Storage is busy");
    fireEvent.click(screen.getByTestId("uninstall-prepare"));
    await waitFor(() => expect(screen.getByTestId("uninstall-result").textContent).toContain("None to put back"));
  });
});

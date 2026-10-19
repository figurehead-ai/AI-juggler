/**
 * AJ-E1010 vitest — About & updates: the Beta label and version always;
 * a development build says it never checks; a check shows version and
 * changes before anything installs; install is the user's click; a
 * failed install says why (version untouched) and offers Retry; restart
 * is refused with the reason while Instances run.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { UpdatesCard } from "./UpdatesCard";
import type { UpdateStatusInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_UPDATE_CALLS__ as string[] | undefined) ?? [];
const base: UpdateStatusInfo = {
  current_version: "0.1.0",
  channel: "beta",
  configured: true,
  check_on_launch: true,
  state: "idle",
  available: null,
  progress: null,
  error: null,
  backup_path: null,
  checked_at_ms: null,
};
const available = { version: "0.2.0", notes: "## New\n- **History**: find any past session.", date_ms: 1_800_000_000_000 };

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_UPDATE"))) delete w()[key];
});

describe("AJ-E1010 updates", () => {
  it("a development build shows the version and Beta, and never checks", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, configured: false };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-unconfigured']")).not.toBeNull());
    expect(container.querySelector("[data-testid='beta-label']")!.textContent).toBe("Beta");
    expect(container.querySelector("[data-testid='update-version']")!.textContent).toContain("0.1.0");
    expect(container.querySelector("[data-testid='update-check']")).toBeNull();
    expect(calls()).toEqual([]);
  });

  it("a check shows the version and changes; install is the user's click", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    w().__AIJ_UPDATE_AFTER_CHECK__ = { ...base, state: "available", available };
    w().__AIJ_UPDATE_AFTER_INSTALL__ = { ...base, state: "installed", available, progress: 100, backup_path: "/data/backups/before-update.ajbackup" };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-check']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-check']")!);
    });
    expect(container.querySelector("[data-testid='update-available']")!.textContent).toContain("0.2.0 (beta)");
    expect(container.querySelector("[data-testid='update-notes']")!.textContent).toContain("History");
    expect(calls()).toEqual(["check"]);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-install']")!);
    });
    expect(calls()).toEqual(["check", "install"]);
    expect(container.querySelector("[data-testid='update-installed']")!.textContent).toContain("before-update.ajbackup");
  });

  it("a failed install says why and offers Retry; restart waits for Instances", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      state: "failed",
      available,
      error: "The download didn't finish or didn't verify: signature verification failed. Your current version is untouched.",
    };
    const { container, unmount } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-failed']")).not.toBeNull());
    expect(container.querySelector("[data-testid='update-failed']")!.textContent).toContain("untouched");
    expect(container.querySelector("[data-testid='update-install']")!.textContent).toBe("Retry");
    unmount();
    w().__AIJ_UPDATE_STATUS__ = { ...base, state: "installed", available };
    w().__AIJ_UPDATE_RESTART_ERROR__ = "invalid update: 2 Instances are still running — restarting would stop them. Restart when they finish.";
    const second = render(<UpdatesCard />);
    await waitFor(() => expect(second.container.querySelector("[data-testid='update-restart']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(second.container.querySelector("[data-testid='update-restart']")!);
    });
    expect(second.container.querySelector("[data-testid='update-error']")!.textContent).toContain("2 Instances are still running");
  });

  it("the launch-time check can be turned off", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-check-on-launch']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-check-on-launch']")!);
    });
    expect(calls()).toEqual(["check-on-launch:false"]);
    expect((container.querySelector("[data-testid='update-check-on-launch']") as HTMLInputElement).checked).toBe(false);
  });
});

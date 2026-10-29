/**
 * AJ-E1205 vitest — the Cloud backup panel: shows the last backup (or
 * why it failed, local data untouched); the schedule is off / daily /
 * weekly; Back up now runs one upload; Restore lists the backups, asks
 * for the recovery code on a Mac without the key, and hands the
 * verified plan to the shared restore dialog; nothing shows while Cloud
 * backup is off or the code isn't saved.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { CloudBackupPanel } from "./CloudBackupPanel";
import { pickerLine } from "../test/pick";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_CLOUD_BACKUP_CALLS__ as string[] | undefined) ?? [];
const base = { enabled: true, code_saved: true, schedule: "weekly", last: null, uploading: false, limit_bytes: 25 * 1024 * 1024 };

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_CLOUD") || key.startsWith("__AIJ_VAULT"))) delete w()[key];
});

describe("AJ-E1205 Cloud backup", () => {
  it("is absent while off or before the recovery code is saved", async () => {
    w().__AIJ_CLOUD_BACKUP__ = { ...base, enabled: false };
    const off = render(<CloudBackupPanel />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(off.container.querySelector("[data-testid='cloud-backup-panel']")).toBeNull();
    off.unmount();
    w().__AIJ_CLOUD_BACKUP__ = { ...base, code_saved: false };
    const unsaved = render(<CloudBackupPanel />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(unsaved.container.querySelector("[data-testid='cloud-backup-panel']")).toBeNull();
  });

  it("backs up now, shows the result, and changes the schedule", async () => {
    w().__AIJ_CLOUD_BACKUP__ = base;
    const { container } = render(<CloudBackupPanel />);
    await waitFor(() => expect(container.querySelector("[data-testid='cloud-backup-last']")!.textContent).toBe("No backup yet."));
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='cloud-backup-now']")!);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='cloud-backup-last']")!.textContent).toContain("Last backup just now"));
    fireEvent.click(container.querySelector<HTMLElement>("[data-testid='cloud-backup-schedule']")!);
    await act(async () => {
      fireEvent.click(pickerLine("every day"));
    });
    expect(calls()).toEqual(["now", "schedule:daily"]);
  });

  it("a failed backup says why and that nothing local changed", async () => {
    w().__AIJ_CLOUD_BACKUP__ = {
      ...base,
      last: { at_ms: Date.now() - 2 * 3_600_000, ok: false, size_bytes: null, error: "This backup is larger than your plan allows (25 MB) — nothing was uploaded." },
    };
    const { container } = render(<CloudBackupPanel />);
    await waitFor(() => expect(container.querySelector("[data-testid='cloud-backup-last']")!.textContent).toContain("larger than your plan allows"));
    expect(container.querySelector("[data-testid='cloud-backup-last']")!.textContent).toContain("Nothing on this computer was changed");
  });

  it("restore on a new Mac asks for the code, then hands the plan to the restore dialog", async () => {
    w().__AIJ_CLOUD_BACKUP__ = base;
    w().__AIJ_VAULT_STATUS__ = { has_key: false, code_saved: true };
    w().__AIJ_CLOUD_BACKUPS__ = [
      { id: "bak_2", label: "AI Juggler · Studio Mac", size_bytes: 412_000, created_at: "2026-10-18T09:00:00Z", this_mac: false },
    ];
    const plan = { path: "/data/from-cloud.ajbackup", token: "t", other_machine: true, notes: [], manifest: {} };
    w().__AIJ_CLOUD_RESTORE_PLAN__ = plan;
    const handed: unknown[] = [];
    const listener = (event: Event) => handed.push((event as CustomEvent).detail);
    window.addEventListener("aij:restore-plan", listener);
    const { container } = render(<CloudBackupPanel />);
    await waitFor(() => expect(container.querySelector("[data-testid='cloud-backup-restore']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='cloud-backup-restore']")!);
    });
    expect(container.querySelector("[data-testid='cloud-backup-row']")!.textContent).toContain("Studio Mac");
    const pick = () => container.querySelector<HTMLButtonElement>("[data-testid='cloud-backup-pick']")!;
    expect(pick().disabled).toBe(true);
    fireEvent.change(container.querySelector("[data-testid='cloud-backup-code']")!, {
      target: { value: "MZXW-6YTB-OI3D-EZLM-NFXG-K4TU-OJSW-Y5DF-MFZG-K3TF-ON2G-K3DF-NZTW-Q2AB" },
    });
    await act(async () => {
      fireEvent.click(pick());
    });
    window.removeEventListener("aij:restore-plan", listener);
    expect(calls()).toEqual(["list", "prepare:bak_2:code"]);
    expect(handed).toEqual([plan]);
  });
});

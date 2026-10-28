/**
 * AJ-E1009 vitest — Settings › Backup & restore: export shows progress
 * and ends by saying what it wrote and withheld; a restore is checked
 * first and says what it will do, with the other-Mac identity warning;
 * the restore runs only from the plan (its token); failures name the
 * file and the reason; cancelling restores nothing.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { BackupRestoreCard } from "./BackupRestoreCard";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_BACKUP_CALLS__ as string[] | undefined) ?? [];
const manifest = {
  format: "ai-juggler-backup",
  format_version: 1,
  schema_version: 27,
  app_version: "0.0.1",
  created_at_ms: 1_800_000_000_000,
  origin: { fingerprint: "other", label: "Studio Mac" },
  counts: { accounts: 2, instances: 41, switch_edges: 3 },
  files: [],
  withheld: ["Environment values of the Launch settings “Main” (Work Max): ANTHROPIC_API_KEY"],
  excluded: [],
};

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_BACKUP") || key.startsWith("__AIJ_RESTORE"))) {
    delete w()[key];
  }
});

describe("AJ-E1009 backup", () => {
  it("export reports progress, then what it wrote and what it withheld", async () => {
    w().__AIJ_BACKUP_DEST__ = "/Users/dev/Backups/move.ajbackup";
    w().__AIJ_BACKUP_PROGRESS__ = [
      [10, 100],
      [55, 100],
    ];
    w().__AIJ_BACKUP_EXPORT__ = { path: "", bytes: 3_400_000, manifest };
    const { container } = render(<BackupRestoreCard />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='backup-export']")!);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='backup-done']")).not.toBeNull());
    const done = container.querySelector("[data-testid='backup-done']")!.textContent!;
    expect(done).toContain("3 MB");
    expect(done).toContain("/Users/dev/Backups/move.ajbackup");
    expect(done).toContain("41 Sessions");
    expect(done).toContain("ANTHROPIC_API_KEY");
    expect(calls()).toEqual(["export:/Users/dev/Backups/move.ajbackup"]);
  });

  it("a failed export names the file and the reason", async () => {
    w().__AIJ_BACKUP_DEST__ = "/Volumes/Full/a.ajbackup";
    w().__AIJ_BACKUP_EXPORT_ERROR__ = "backup write: No space left on device (/Volumes/Full/a.ajbackup)";
    const { container } = render(<BackupRestoreCard />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='backup-export']")!);
    });
    const alert = container.querySelector("[data-testid='backup-failed']")!;
    expect(alert.getAttribute("role")).toBe("alert");
    expect(alert.textContent).toContain("The backup was not written");
    expect(alert.textContent).toContain("/Volumes/Full/a.ajbackup");
  });

  it("restore checks first, warns about another Mac, and runs only from the plan", async () => {
    w().__AIJ_BACKUP_FILE__ = "/Users/dev/move.ajbackup";
    w().__AIJ_RESTORE_PLAN__ = {
      path: "",
      manifest,
      other_machine: true,
      notes: ["This backup comes from Studio Mac. It carries that computer's data, never its identity…", "Sign in to each Account again afterwards."],
      token: "tok-1",
    };
    w().__AIJ_RESTORE_RESULT__ = {
      counts: { accounts: 2, instances: 41, switch_edges: 3 },
      kept_before: ["/data/juggler.sqlite.before-restore-1"],
      next_steps: ["Restart AI Juggler to finish.", "Sign in to each Account again."],
    };
    const { container } = render(<BackupRestoreCard />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='backup-restore']")!);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='restore-plan']")).not.toBeNull());
    expect(container.querySelector("[data-testid='restore-other-machine']")!.textContent).toContain(
      "never its identity",
    );
    expect(calls()).toEqual(["verify:/Users/dev/move.ajbackup"]);
    expect(document.activeElement).toBe(container.querySelector("[data-testid='restore-go']"));
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='restore-go']")!);
    });
    expect(calls()).toEqual(["verify:/Users/dev/move.ajbackup", "restore:/Users/dev/move.ajbackup:tok-1"]);
    expect(container.querySelector("[data-testid='restore-done']")!.textContent).toContain("Restart AI Juggler");
  });

  it("a refused backup names the file and the reason; cancel restores nothing", async () => {
    w().__AIJ_BACKUP_FILE__ = "/Users/dev/old.ajbackup";
    w().__AIJ_RESTORE_PLAN_ERROR__ =
      "invalid restore: /Users/dev/old.ajbackup can't be restored: db/juggler.sqlite does not match its checksum — the file is damaged";
    const { container } = render(<BackupRestoreCard />);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='backup-restore']")!);
    });
    const alert = container.querySelector("[data-testid='backup-failed']")!;
    expect(alert.textContent).toContain("Nothing was restored");
    expect(alert.textContent).toContain("old.ajbackup");
    expect(alert.textContent).toContain("checksum");
    expect(calls().some((call) => call.startsWith("restore:"))).toBe(false);
  });
});

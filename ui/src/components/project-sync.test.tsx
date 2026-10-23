import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as sync from "../lib/ipc/projectSync";
import * as setup from "../lib/ipc/switchSetup";
import { ProjectSyncSheet } from "./ProjectSyncSheet";
import { ProjectFacts } from "../pages/ProjectPage";
import { EXAMPLE_PROJECT_SYNC, type ProjectSyncSeam } from "../preview/project-sync";
import type { ProjectInfo } from "../lib/bridge";

/**
 * AJ-E1507: a project's rules files and MCP servers, made the same in one
 * reviewed click: the header says whether the rules agree, the sheet shows
 * the diff first, and every change says what it did and can be undone.
 */

type Seams = { __AIJ_PROJECT_SYNC__?: ProjectSyncSeam };

const PROJECT = {
  id: "p-shop",
  name: "shop",
  root: "/Users/you/Dev/shop",
} as unknown as ProjectInfo;

const seed = (seam: ProjectSyncSeam = EXAMPLE_PROJECT_SYNC) => {
  (window as never as Seams).__AIJ_PROJECT_SYNC__ = structuredClone(seam);
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete (window as never as Seams).__AIJ_PROJECT_SYNC__;
});

describe("the project's rules line", () => {
  it("says nothing when there's nothing to compare", async () => {
    const view = render(<ProjectFacts project={PROJECT} sessions={[]} />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("project-rules")).toBeNull();
  });

  it("says a file differs, and opens the sheet", async () => {
    seed();
    const view = render(<ProjectFacts project={PROJECT} sessions={[]} />);
    const line = await view.findByTestId("project-rules");
    expect(line.textContent).toBe("1 rules file differs");
    fireEvent.click(line);
    expect(await within(document.body).findByRole("dialog", { name: "Rules and MCP servers" })).toBeTruthy();
  });

  it("reads the rules again when the sheet closes, wherever it was opened", async () => {
    seed();
    const view = render(<ProjectFacts project={PROJECT} sessions={[]} />);
    expect((await view.findByTestId("project-rules")).textContent).toBe("1 rules file differs");
    // The menu's sheet applies the master and closes.
    const sheet = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    await sheet.findByTestId("sync-diff");
    fireEvent.click(sheet.getByRole("button", { name: "Apply to AGENTS.md" }));
    await waitFor(() => expect(sheet.getByTestId("project-sync-summary").textContent).toBe("Rules in sync"));
    sheet.unmount();
    await waitFor(() => expect(view.getByTestId("project-rules").textContent).toBe("Rules in sync"));
  });
});

describe("the rules and servers sheet", () => {
  it("shows the first file that differs as a diff, and applies the master to it only", async () => {
    seed();
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    const diff = await view.findByTestId("sync-diff");
    expect(diff.textContent).toContain("Use tabs.");
    expect(diff.textContent).toContain("Removed: ");
    expect(diff.textContent).toContain("Added: Use two spaces.");
    expect(view.getByText(/2 lines added, 1 removed/)).toBeTruthy();
    fireEvent.click(view.getByRole("button", { name: "Apply to AGENTS.md" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("AGENTS.md now has the rules from CLAUDE.md"));
    expect(document.activeElement).toBe(view.getByRole("status"));
    expect(view.getByRole("button", { name: "Undo" })).toBeTruthy();
    await waitFor(() => expect(view.getByTestId("project-sync-summary").textContent).toBe("Rules in sync"));
  });

  it("lets the user pick the master", async () => {
    seed();
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    fireEvent.click(await view.findByRole("radio", { name: "Use AGENTS.md as the master" }));
    await waitFor(() =>
      expect(within(view.getByTestId("sync-file-CLAUDE.md")).getByText("Differs from AGENTS.md")).toBeTruthy(),
    );
    expect((view.getByRole("radio", { name: "Use AGENTS.md as the master" }) as HTMLInputElement).checked).toBe(true);
  });

  it("offers a file its tool can point at the master instead of copying", async () => {
    const seam = structuredClone(EXAMPLE_PROJECT_SYNC);
    seam.rules!.files[1]!.import_line = "@CLAUDE.md";
    seed(seam);
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    const read = await view.findByRole("button", { name: "Read CLAUDE.md instead" });
    expect(read.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(read);
    expect(read.getAttribute("aria-pressed")).toBe("true");
    // Apply waits for the new diff.
    const apply = view.getByRole("button", { name: "Apply to AGENTS.md" }) as HTMLButtonElement;
    await waitFor(() => expect(apply.disabled).toBe(false));
    fireEvent.click(apply);
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("AGENTS.md now reads CLAUDE.md"));
  });

  it("lists each account's servers and copies one, saying what stays behind", async () => {
    seed();
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    const servers = await view.findByTestId("project-sync-servers");
    expect(within(servers).getByText("docs, github")).toBeTruthy();
    expect(within(servers).getByText(/GITHUB_TOKEN stays behind/)).toBeTruthy();
    fireEvent.click(within(servers).getByRole("button", { name: "Copy github to Codex" }));
    await waitFor(() => expect(within(servers).getByRole("status").textContent).toContain("Set up the MCP server github"));
    // The Copy button has gone: focus is on what happened, inside the sheet.
    expect(document.activeElement).toBe(within(servers).getByRole("status"));
    expect(within(servers).queryByRole("button", { name: "Copy github to Codex" })).toBeNull();
    expect(within(servers).getByText("Every account has the same servers.")).toBeTruthy();
  });

  it("says when no file has rules of its own yet", async () => {
    seed({
      rules: {
        master: null,
        in_sync: true,
        summary: null,
        files: [
          { ...EXAMPLE_PROJECT_SYNC.rules!.files[0]!, standing: "missing", words: "Not there", can_be_master: false, sha256: null, modified_ms: null },
        ],
      },
    });
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    expect(await view.findByText(/nothing to compare/)).toBeTruthy();
    expect(view.queryByTestId("sync-apply")).toBeNull();
  });

  it("says why an apply was refused, and leaves the panel to try again", async () => {
    seed();
    vi.spyOn(sync, "projectSyncApply").mockRejectedValue({
      code: "refused",
      retryable: false,
      user_message: "AGENTS.md or CLAUDE.md changed since you looked: look again before applying",
    });
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    await view.findByTestId("sync-diff");
    fireEvent.click(view.getByRole("button", { name: "Apply to AGENTS.md" }));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("changed since you looked"));
    expect(view.getByRole("button", { name: "Apply to AGENTS.md" })).toBeTruthy();
    expect(view.queryByRole("status")).toBeNull();
  });

  it("undoes an apply from the sheet", async () => {
    seed();
    const undo = vi.spyOn(setup, "switchSetupUndo").mockResolvedValue("Put back the rules in AGENTS.md as it was");
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    await view.findByTestId("sync-diff");
    fireEvent.click(view.getByRole("button", { name: "Apply to AGENTS.md" }));
    fireEvent.click(await view.findByRole("button", { name: "Undo" }));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("Put back the rules in AGENTS.md as it was"));
    expect(undo).toHaveBeenCalledWith("record-AGENTS.md");
    expect(view.queryByRole("button", { name: "Undo" })).toBeNull();
    expect(document.activeElement).toBe(view.getByRole("status"));
  });

  it("doesn't push a missing file the summary doesn't count", async () => {
    const seam = structuredClone(EXAMPLE_PROJECT_SYNC);
    seam.rules = {
      ...seam.rules!,
      in_sync: true,
      summary: "Rules in sync",
      files: [
        seam.rules!.files[0]!,
        { ...seam.rules!.files[1]!, standing: "missing", words: "Not there", counts: false, sha256: null, modified_ms: null },
      ],
    };
    seed(seam);
    const view = render(<ProjectSyncSheet project={PROJECT} onClose={() => undefined} />);
    const row = await view.findByTestId("sync-file-AGENTS.md");
    expect(view.queryByTestId("sync-apply")).toBeNull();
    expect(within(row).getByText("Not there").className).not.toContain("text-warning");
    // It can still be applied, on request.
    fireEvent.click(within(row).getByRole("button", { name: "Show changes" }));
    expect(await view.findByTestId("sync-apply")).toBeTruthy();
  });
});

/**
 * 0.3.4, the page-switching report — a project's header line and its
 * Overview's checkout callout read git's state and the rules once
 * between them, and a page opened again shows the last answer at once
 * (both drew nothing until it came, then pushed the page down). The
 * rules sheet closing reads the rules again.
 */
import { act, cleanup, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProjectFacts } from "../ProjectPage";
import { CheckoutCallout } from "./Callouts";
import { PROJECT_SYNC_EVENT } from "../../components/ProjectSyncSheet";
import * as projects from "../../lib/ipc/projects";
import * as sync from "../../lib/ipc/projectSync";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { GitStatusInfo } from "../../types/generated/GitStatusInfo";
import type { RulesSyncInfo } from "../../types/generated/RulesSyncInfo";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const project = { id: "proj-1", root: "/w/alpha", name: "alpha" } as ProjectInfo;
const GIT = { branch: "trunk", changed: 2, untracked: 1, ahead: 0, behind: 0, worktree: false, created_by_juggler: false, path: "/w/alpha" };

const both = () =>
  render(
    <>
      <ProjectFacts project={project} sessions={[]} />
      <CheckoutCallout project={project} sharedBy={1} />
    </>,
  );

describe("0.3.4 a project's checkout, kept", () => {
  it("the header and the callout share one read of each", async () => {
    const git = vi.spyOn(projects, "projectsGitStatus").mockResolvedValue(GIT as never);
    const rules = vi.spyOn(sync, "projectSyncRules").mockResolvedValue({ summary: "Rules in sync", in_sync: true } as never);
    const view = both();
    await waitFor(() => expect(view.getByTestId("project-rules").textContent).toBe("Rules in sync"));
    expect(git).toHaveBeenCalledTimes(1);
    expect(rules).toHaveBeenCalledTimes(1);
  });

  it("opened again, shows the last answer in its first frame, then reads again", async () => {
    const git = vi.spyOn(projects, "projectsGitStatus").mockResolvedValue(GIT as never);
    vi.spyOn(sync, "projectSyncRules").mockResolvedValue({ summary: "1 rules file differs", in_sync: false } as never);
    const first = both();
    await waitFor(() => expect(first.getByTestId("project-rules")).toBeTruthy());
    first.unmount();
    const again = both();
    expect(again.getByTestId("project-branch").textContent).toBe("· trunk");
    expect(again.getByTestId("project-rules").textContent).toBe("1 rules file differs");
    expect(again.getByTestId("project-uncommitted-callout")).toBeTruthy();
    await waitFor(() => expect(git).toHaveBeenCalledTimes(2));
  });

  it("the rules sheet closing reads the rules again", async () => {
    vi.spyOn(projects, "projectsGitStatus").mockResolvedValue(GIT as never);
    const rules = vi.spyOn(sync, "projectSyncRules").mockResolvedValue({ summary: "1 rules file differs", in_sync: false } as never);
    const view = both();
    await waitFor(() => expect(view.getByTestId("project-rules").textContent).toBe("1 rules file differs"));
    rules.mockResolvedValue({ summary: "Rules in sync", in_sync: true } as never);
    await act(async () => void window.dispatchEvent(new CustomEvent(PROJECT_SYNC_EVENT, { detail: "proj-1" })));
    await waitFor(() => expect(view.getByTestId("project-rules").textContent).toBe("Rules in sync"));
  });
});

// 0.5.1 F66 (PAT-COLOR-2, PAT-SURF-1): what needs the user in the checkout
// is a tone tile with an amber glyph and an amber mark by its title; the
// card gets no amber outline and no amber fill.
describe("0.5.1 F66 the checkout's look", () => {
  const dirty: GitStatusInfo = { branch: "trunk", changed: 2, untracked: 1, ahead: 0, behind: 0, worktree: false, created_by_juggler: false, path: "/w/alpha" };
  const rules = (in_sync: boolean): RulesSyncInfo => ({ master: null, files: [], in_sync, summary: in_sync ? "Rules in sync" : "1 rules file differs" });

  it("an uncommitted checkout: an amber mark and glyph, no amber fill or outline", async () => {
    vi.spyOn(projects, "projectsGitStatus").mockResolvedValue(dirty);
    vi.spyOn(sync, "projectSyncRules").mockResolvedValue(rules(false));
    const view = render(<CheckoutCallout project={project} sharedBy={2} />);
    const uncommitted = await view.findByTestId("project-uncommitted-callout");
    expect(view.getByTestId("project-checkout").className).not.toMatch(/warning/);
    const tile = within(uncommitted).getByTestId("callout-tile");
    expect(tile.className).toContain("bg-surface-2");
    expect(tile.className).toContain("text-warning");
    expect(tile.className).not.toMatch(/bg-warning/);
    expect(within(uncommitted).getByTestId("callout-mark").getAttribute("data-level")).toBe("2");
    await waitFor(() => expect(view.getAllByTestId("callout-mark")).toHaveLength(3));
    expect(view.container.innerHTML).not.toMatch(/bg-warning|border-warning/);
  });

  it("all committed and in sync: no amber at all", async () => {
    vi.spyOn(projects, "projectsGitStatus").mockResolvedValue({ ...dirty, changed: 0, untracked: 0 });
    vi.spyOn(sync, "projectSyncRules").mockResolvedValue(rules(true));
    const view = render(<CheckoutCallout project={project} sharedBy={1} />);
    await waitFor(() => expect(view.container.textContent).toContain("Every tool works from the same rules."));
    expect(view.queryAllByTestId("callout-mark")).toHaveLength(0);
    expect(view.container.innerHTML).not.toMatch(/warning/);
  });
});

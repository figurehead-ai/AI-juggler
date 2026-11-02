/**
 * 0.3.4, the page-switching report — a project's header line and its
 * Overview's checkout callout read git's state and the rules once
 * between them, and a page opened again shows the last answer at once
 * (both drew nothing until it came, then pushed the page down). The
 * rules sheet closing reads the rules again.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProjectFacts } from "../ProjectPage";
import { CheckoutCallout } from "./Callouts";
import { PROJECT_SYNC_EVENT } from "../../components/ProjectSyncSheet";
import * as projects from "../../lib/ipc/projects";
import * as sync from "../../lib/ipc/projectSync";
import type { ProjectInfo } from "../../lib/bridge";

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

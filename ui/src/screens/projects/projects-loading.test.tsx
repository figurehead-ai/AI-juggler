/**
 * AJ-E1552 — the Projects list's skeleton draws only while the shell's
 * first read loads (Still loading after 3 s), then its own empty words.
 *
 * The list is the shell's first read, which has its own deadline
 * (shell/use-app-data): when it misses, the read counts as done and the
 * "service stopped answering" banner, with its Retry, takes over. So the
 * list is tested for loading and empty here; its error is the shell's.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, it, vi } from "vitest";
import { expectEmpty, expectSlowSkeleton } from "../../test/loading-kit";
import { ProjectsScreen } from "./ProjectsScreen";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const projects = (loading: boolean) => render(<ProjectsScreen projects={[]} sessions={[]} loading={loading} now={0} onNewProject={() => undefined} />);

describe("the Projects list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    projects(true);
    await expectSlowSkeleton("projects-loading");
  });
  it("empty", async () => {
    projects(false);
    await expectEmpty("projects-empty", "No projects yet. A project appears when a session runs in its folder.", "projects-loading");
  });
});

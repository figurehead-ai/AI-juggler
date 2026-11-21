/**
 * AJ-E2312 item 6 (the LC2 walk): New pool kept a project picked under
 * one template ("team") after the template changed to "Sealed client
 * project". A pick is kept only where the new template uses a project the
 * same way (as many, kept apart or not); else it's cleared and Where says
 * so plainly. Review names exactly what is picked.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { templateById } from "../../lib/pool-templates";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { NewPool, projectsOnChoose } from "./NewPool";

beforeEach(async () => {
  localStorage.clear();
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await refreshPools();
});
afterEach(cleanup);

const place = (id: string, root: string): ProjectInfo => ({ ...MOCK_PROJECTS[0]!, id, name: root.split("/").pop()!, root, active_instance: null, last_activity_ms: null });
const TEAM = place("team", "/Users/you/lab/team");
const CLIENT = place("client", "/Users/you/lab/client");
// The work is in team: a one-project template starts there.
const RUNNING: InstanceInfo = { ...MOCK_SESSIONS[0]!, id: "s-team", project_id: "team", cwd: TEAM.root, status: "running", started_at_ms: Date.now() - 60_000 };

const card = (template: string) => screen.getAllByTestId("new-pool-template").find((one) => one.dataset.template === template)!;
const pickTemplate = (template: string) => {
  // Templates other than Start from scratch may sit behind a fold.
  const fold = screen.queryByTestId("new-pool-templates-fold");
  if (fold && !fold.hasAttribute("data-open")) fireEvent.click(within(fold).getAllByRole("button")[0]!);
  fireEvent.click(within(card(template)).getByRole("radio"));
};
const where = () => screen.getByRole("group", { name: "Which project" });
const checked = (name: string) => within(where()).getByRole<HTMLInputElement>("checkbox", { name }).checked;
const back = () => fireEvent.click(screen.getByTestId("new-pool-back"));
const next = () => fireEvent.click(screen.getByTestId("new-pool-next"));

describe("the project pick follows the template (AJ-E2312 item 6)", () => {
  it("a pick made for a team is cleared under Sealed client project, and Where says so plainly", () => {
    render(<NewPool sessions={[RUNNING]} projects={[TEAM, CLIENT]} accounts={[]} project={null} onClose={() => undefined} />);
    pickTemplate("writer-reviewer");
    next();
    expect(checked("team")).toBe(true);
    back();
    pickTemplate("sealed-client");
    next();
    // Not carried into a pool that keeps its project apart.
    expect(checked("team")).toBe(false);
    expect(checked("client")).toBe(false);
    expect(screen.getByTestId("new-pool-where-words").textContent).toBe("«Sealed client project» keeps one project apart, so your pick (team) was cleared. Pick at least one project: only its sessions can join.");
    expect(screen.getByTestId("new-pool-next")).toHaveProperty("disabled", true);
    // Picked again: the note goes, and Next goes on.
    fireEvent.click(within(where()).getByRole("checkbox", { name: "client" }));
    expect(screen.getByTestId("new-pool-where-words").textContent).not.toContain("cleared");
    expect(screen.getByTestId("new-pool-next")).toHaveProperty("disabled", false);
  });

  it("a template that uses a project the same way keeps the person's pick, not where the work is", () => {
    render(<NewPool sessions={[RUNNING]} projects={[TEAM, CLIENT]} accounts={[]} project={null} onClose={() => undefined} />);
    pickTemplate("writer-reviewer");
    next();
    fireEvent.click(within(where()).getByRole("checkbox", { name: "team" }));
    fireEvent.click(within(where()).getByRole("checkbox", { name: "client" }));
    back();
    pickTemplate("feature-team");
    next();
    expect(checked("client")).toBe(true);
    expect(checked("team")).toBe(false);
    expect(screen.getByTestId("new-pool-where-words").textContent).not.toContain("cleared");
  });

  it("Review names exactly what is picked", () => {
    render(<NewPool sessions={[RUNNING]} projects={[TEAM, CLIENT]} accounts={[]} project={null} onClose={() => undefined} />);
    pickTemplate("sealed-client");
    next();
    fireEvent.click(within(where()).getByRole("checkbox", { name: "team" }));
    fireEvent.click(within(where()).getByRole("checkbox", { name: "client" }));
    for (let n = 0; n < 9 && !screen.queryByTestId("new-pool-step-review"); n += 1) next();
    const row = screen.getAllByTestId("new-pool-review-rows")[0]!.querySelector('[data-row="where"] [data-testid="new-pool-review-line"]')!;
    expect(row.textContent).toMatch(/ works in client\.$/);
  });

  it("the rule: kept where the use is the same; cleared, and said, where it isn't; before Where, nothing was picked", () => {
    const names = (ids: (string | null)[]) => ids.join(" and ");
    const team = { template: "writer-reviewer", every: false, projects: ["team"] as (string | null)[] };
    expect(projectsOnChoose(team, templateById("scouts"), { start: "web", picked: true, names })).toEqual({ every: false, projects: ["team"], cleared: null });
    expect(projectsOnChoose(team, templateById("across-repos"), { start: "web", picked: true, names })).toEqual({ every: false, projects: [], cleared: { what: "team", why: "«Across repositories» works across two or more projects" } });
    expect(projectsOnChoose(team, templateById("reviewer-for-everything"), { start: "web", picked: true, names })).toEqual({ every: true, projects: [], cleared: { what: "team", why: "«A reviewer for everything» works in every project" } });
    expect(projectsOnChoose({ template: "reviewer-for-everything", every: true, projects: [] }, templateById("blank"), { start: "web", picked: true, names }).cleared?.what).toBe("every project");
    // Where not reached yet: the new template starts where the work is.
    expect(projectsOnChoose(team, templateById("sealed-client"), { start: "web", picked: false, names })).toEqual({ every: false, projects: ["web"], cleared: null });
  });
});

describe("Review's Where names a project exactly", () => {
  it("two projects that share a name are told apart by their folder; no project is said as the card says it", () => {
    const one = place("web-1", "/Users/you/lab/web");
    const two = place("web-2", "/Users/you/work/web");
    render(<NewPool sessions={[]} projects={[one, two]} accounts={[]} project="web-2" onClose={() => undefined} />);
    next();
    fireEvent.click(within(where()).getByRole("checkbox", { name: "Sessions with no project" }));
    for (let n = 0; n < 9 && !screen.queryByTestId("new-pool-step-review"); n += 1) next();
    const row = screen.getAllByTestId("new-pool-review-rows")[0]!.querySelector('[data-row="where"] [data-testid="new-pool-review-line"]')!;
    expect(row.textContent).toMatch(/ works in web \(~\/work\/web\) and sessions with no project\.$/);
  });
});

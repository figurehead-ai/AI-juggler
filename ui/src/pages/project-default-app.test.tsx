/**
 * AJ-E3104 R1: a project's header has a door to its default App, the one
 * its new sessions start in; a project without one has none. Worked on
 * more than one Mac, a door to Fleet ("on 2 Macs").
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { OPEN_APP } from "../lib/apps";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { ProjectFacts } from "./ProjectPage";
import { MacsDoor } from "./session/HeaderDoors";

afterEach(cleanup);

const folders: AccountInfo[] = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const project: ProjectInfo = { id: "proj-1", root: "/w/shop", name: "shop", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false };

it("opens its default App's page in one click", async () => {
  const opened = vi.fn();
  const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
  window.addEventListener(OPEN_APP, listen);
  const view = render(<ProjectFacts project={{ ...project, default_account_id: "claude-personal" }} sessions={[]} folders={folders} />);
  const door = await view.findByTestId("door-default-app");
  expect(door.textContent).toBe("starts inClaude Code · Personal");
  expect(door.getAttribute("title")).toBe("New sessions here start in Claude Code · Personal. Open its page");
  fireEvent.click(door);
  window.removeEventListener(OPEN_APP, listen);
  expect(opened).toHaveBeenCalledWith({ id: "cli:claude-personal" });
});

it("has no door without a default App, or when its folder isn't one of yours", () => {
  expect(render(<ProjectFacts project={project} sessions={[]} folders={folders} />).queryByTestId("door-default-app")).toBeNull();
  cleanup();
  expect(render(<ProjectFacts project={{ ...project, default_account_id: "gone" }} sessions={[]} folders={folders} />).queryByTestId("door-default-app")).toBeNull();
});

it("a project worked on two Macs opens Fleet from \"on 2 Macs\"; on this Mac alone there's no door", () => {
  const opened = vi.fn();
  const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
  window.addEventListener("aij:open-screen", listen);
  const view = render(<MacsDoor count={2} />);
  fireEvent.click(view.getByTestId("door-macs"));
  window.removeEventListener("aij:open-screen", listen);
  expect(view.getByTestId("door-macs").textContent).toBe("on 2 Macs");
  expect(opened).toHaveBeenCalledWith({ id: "fleet" });
  cleanup();
  expect(render(<MacsDoor count={1} />).container.textContent).toBe("");
});

/**
 * ADR-0028 (`automation_handover`): what wakes a conversation by itself is
 * named before the switch (moves to the copy, off on the original; or
 * stays when a terminal can't take it) and after it (when each side takes
 * effect, with Restart for the copy's app).
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AutomationAfter, AutomationBefore, joined } from "./switch/AutomationLine";
import type { PreflightInfoData, SwitchExecutedData } from "../lib/bridge";

const restart = vi.fn();
vi.mock("../lib/go-to", () => ({ restartPlace: (...args: unknown[]) => restart(...args) }));

afterEach(() => {
  cleanup();
  restart.mockReset();
});

const preflight = (over: Partial<PreflightInfoData>): PreflightInfoData => ({ ...over }) as PreflightInfoData;
const executed = (over: Partial<SwitchExecutedData>): SwitchExecutedData => ({ session_id: "s", pid: 1, edge_id: "e", source_session_id: "a", ...over });

describe("before the switch", () => {
  it("says nothing when nothing runs by itself", () => {
    const { container } = render(<AutomationBefore preflight={preflight({})} destination="desktop" app="Claude" />);
    expect(container.textContent).toBe("");
  });

  it("says the copy takes it over and the original's is turned off", () => {
    render(<AutomationBefore preflight={preflight({ automation: ["Auto-fix on pull request #210 (org/repo)"], automation_needs_desktop: true })} destination="desktop" app="Claude" />);
    expect(screen.getByTestId("switch-automation-moves").textContent).toBe(
      "Auto-fix on pull request #210 (org/repo) moves to the copy and is turned off on the original, so only one of them works.",
    );
  });

  it("warns that a terminal can't take what only the app runs", () => {
    render(<AutomationBefore preflight={preflight({ automation: ["Auto-fix on pull request #210 (org/repo)"], automation_needs_desktop: true })} destination="cli" app="Claude" />);
    expect(screen.getByTestId("switch-automation-left").textContent).toContain("stays with the original: only Claude runs it. Continue in Claude to take it along.");
  });

  it("a goal moves on the CLI too, and what the account keeps is said as staying", () => {
    render(
      <AutomationBefore
        preflight={preflight({ automation: ["Its goal, “Ship 0.4”, which Codex keeps working on"], automation_stays: ["The heartbeat “Brief” stays with this account"] })}
        destination="cli"
        app="Codex"
      />,
    );
    expect(screen.getByTestId("switch-automation-moves")).toBeTruthy();
    expect(screen.getByTestId("switch-automation-stays").textContent).toBe("The heartbeat “Brief” stays with this account.");
  });
});

describe("after the switch", () => {
  it("offers the copy's app's restart when it takes it over only then", () => {
    render(<AutomationAfter result={executed({ automation_moved: ["Auto-fix on pull request #210 (org/repo)"], automation_copy_place: "claude:/p", automation_original_waits: true })} app="Claude · aij-test" sourceApp="Claude · olivia" />);
    const text = screen.getByTestId("switch-done-automation").textContent ?? "";
    expect(text).toContain("runs in the copy once Claude · aij-test restarts");
    expect(text).toContain("turned off when Claude · olivia next closes");
    fireEvent.click(screen.getByTestId("switch-done-automation-restart"));
    expect(restart).toHaveBeenCalledWith("claude:/p", true);
  });

  it("says it's done when both sides took effect", () => {
    render(<AutomationAfter result={executed({ automation_moved: ["A", "B"] })} app="Codex" sourceApp="Codex" />);
    expect(screen.getByTestId("switch-done-automation").textContent).toBe("A and B now run in the copy. On the original they're off; switching back turns them on there again.");
    expect(screen.queryByTestId("switch-done-automation-restart")).toBeNull();
  });

  it("joins lists the way a sentence does", () => {
    expect(joined(["A", "B", "C"])).toBe("A, B and C");
  });
});

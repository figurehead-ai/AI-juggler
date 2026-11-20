/**
 * AJ-E2311 item 3 (the 0.4.6 walks: the Stop toast read "Stopping web ·
 * Claude Code CLI · aij-test", the folder, not the session): a Stop names
 * the session as the rail does, then where it runs.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { resetToasts, ToastHost } from "../components/Toast";
import { holdStop, stopOf } from "./stop";
import { shortName } from "./session-titles";
import { instance, project } from "../test/fixtures";

afterEach(() => {
  resetToasts();
  cleanup();
});

describe("the Stop toast names the session", () => {
  it("by its name, then its folder and App", () => {
    render(<ToastHost />);
    const named = instance({ id: "s-p1", label: "p1-writer", project_id: "proj-web", provider: "claude-code", account_label: "aij-test" });
    act(() => holdStop(stopOf(named, [project({ id: "proj-web", name: "web" })])));
    const toast = screen.getByTestId("stop-undo-toast").textContent ?? "";
    expect(toast).toContain("Stopping p1-writer.");
    expect(toast).toContain("web · Claude Code CLI · aij-test. Undo keeps it running.");
  });

  it("an untitled one by the rail's short name", () => {
    const untitled = instance({ id: "s-2", label: undefined, title: undefined, project_id: null, cwd: "/Users/you/web" });
    expect(stopOf(untitled, []).label).toBe(shortName(untitled));
  });
});
